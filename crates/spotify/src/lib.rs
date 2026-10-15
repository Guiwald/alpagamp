//! Spotify dans Alpagamp (ADR 0002, 0010, 0018), et l'annuaire de radios de la Bibliothèque.
//!
//! - Connexion par le navigateur, une seule fois : les identifiants réutilisables sont ensuite gardés par
//!   librespot dans le dossier de l'appli.
//! - Lecture : chaque morceau Spotify devient une source du moteur d'Alpagamp (égaliseur, visualisation, fondu
//!   enchaîné s'appliquent) ; librespot décode, un tampon fait le lien (`sink`).
//! - Spotify Connect : Alpagamp apparaît comme appareil dans l'appli Spotify ; ce qu'on y lance passe aussi par
//!   le moteur.
//! - Bibliothèque (ADR 0022) : playlists, titres likés, albums et recherche de titres par les canaux internes
//!   de Spotify ; albums et playlists d'une recherche par l'API web ; listes gardées sur disque ; radios par
//!   radio-browser.info.

pub mod diagnostic;
pub mod internal;
pub mod library;
pub mod oauth;
pub mod radios;
pub mod sink;
pub mod webapi;

pub use library::{Fault, LibraryReply, Notice};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use alpagamp_audio::{AudioError, Opener, PcmSource, TrackInfo};
use alpagamp_core::library::{LibraryItem, LibraryKind};
use crossbeam_channel::{Receiver, Sender};
use librespot_connect::{ConnectConfig, Spirc};
use librespot_core::authentication::Credentials;
use librespot_core::cache::Cache;
use librespot_core::config::DeviceType;
use librespot_core::{Session, SessionConfig, SpotifyUri};
use librespot_metadata::audio::{AudioItem, UniqueFields};
use librespot_playback::config::{Bitrate, PlayerConfig};
use librespot_playback::mixer::softmixer::SoftMixer;
use librespot_playback::mixer::{Mixer, MixerConfig, NoOpVolume};
use librespot_playback::player::{Player, PlayerEvent};
use parking_lot::Mutex;

use crate::library::{DEFAULT_WAIT, FRESH, ListCache, cache_key_for_uri, now_secs};
use crate::oauth::{Attempt, AuthError, OAuthToken};
use crate::sink::{BufferSink, BufferSource, Shared};

/// État de la connexion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    LoggedOut,
    /// Navigateur ouvert, en attente de l'autorisation.
    Authorizing,
    Connecting,
    Connected { user: String },
    Failed(String),
}

/// Ce que la bibliothèque peut demander.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryRequest {
    /// Mes playlists (avec « Titres likés » sous ce libellé).
    Playlists { liked_label: String },
    /// Morceaux d'une playlist, d'un album ou des titres likés.
    Tracks(String),
    Albums,
    Search(String),
    RadioTop,
    RadioSearch(String),
}

/// Événements pour l'application.
#[derive(Debug, Clone)]
pub enum Event {
    Status(Status),
    /// Spotify Connect : Alpagamp a été choisi dans l'appli Spotify et commence à jouer.
    ConnectStart,
    ConnectPause,
    ConnectResume,
    /// Connect s'arrête (autre appareil choisi, déconnexion).
    ConnectStop,
    /// Réponse de la Bibliothèque (plusieurs peuvent arriver pour une requête : voir [`LibraryReply::last`]).
    Library { request: u64, reply: LibraryReply },
}

/// Commandes de Spotify Connect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectCommand {
    Play,
    Pause,
    Next,
    Previous,
    Seek(u64),
}

struct Inner {
    rt: tokio::runtime::Runtime,
    dir: PathBuf,
    device_id: String,
    device_name: String,
    device_type: DeviceType,
    open_url: Box<dyn Fn(&str) + Send + Sync>,
    tx: Sender<Event>,
    status: Mutex<Status>,
    session: Mutex<Option<Session>>,
    spirc: Mutex<Option<Spirc>>,
    connect: Arc<Shared>,
    connect_active: AtomicBool,
    token: Mutex<Option<OAuthToken>>,
    next_request: AtomicU64,
    /// Connexion par le navigateur en cours (pour rouvrir la page ou annuler).
    login: Mutex<Option<Arc<Attempt>>>,
    lists: ListCache,
    /// Attente demandée par Spotify, par canal (« interne », « web ») : échéance en secondes depuis 1970.
    waits: Mutex<HashMap<String, u64>>,
}

/// Canaux de la Bibliothèque.
const INTERNAL: &str = "interne";
const WEB: &str = "web";

/// Le client Spotify (et l'annuaire de radios).
pub struct Spotify {
    inner: Arc<Inner>,
    events: Receiver<Event>,
}

fn player_config() -> PlayerConfig {
    PlayerConfig { bitrate: Bitrate::Bitrate320, gapless: false, normalisation: false, ..PlayerConfig::default() }
}

/// Informations du moteur à partir des métadonnées de librespot.
fn info_from(item: &AudioItem) -> TrackInfo {
    let (artist, album) = match &item.unique_fields {
        UniqueFields::Track { artists, album, .. } => (Some(artists.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", ")), Some(album.clone())),
        UniqueFields::Episode { show_name, .. } => (Some(show_name.clone()), None),
        UniqueFields::Local { artists, album, .. } => (artists.clone(), album.clone()),
    };
    TrackInfo {
        duration_ms: Some(item.duration_ms as u64),
        sample_rate: sink::RATE,
        channels: 2,
        bitrate_kbps: Some(320),
        title: Some(item.name.clone()),
        artist: artist.filter(|a| !a.is_empty()),
        album: album.filter(|a| !a.is_empty()),
        seekable: true,
        codec: "SPOTIFY".into(),
        ..Default::default()
    }
}

fn readable(e: impl std::fmt::Display) -> String {
    let s = e.to_string();
    if s.to_lowercase().contains("premium") {
        "Spotify Premium est nécessaire".into()
    } else {
        s
    }
}

impl Inner {
    fn set_status(&self, s: Status) {
        *self.status.lock() = s.clone();
        let _ = self.tx.send(Event::Status(s));
    }

    fn cache(&self) -> Option<Cache> {
        Cache::new(Some(self.dir.join("credentials")), None, None, None).ok()
    }

    fn token_path(&self) -> PathBuf {
        self.dir.join("oauth.json")
    }

    fn save_token(&self, t: &OAuthToken) {
        let _ = std::fs::create_dir_all(&self.dir);
        if let Ok(s) = serde_json::to_string(t) {
            let p = self.token_path();
            let _ = std::fs::write(&p, s);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
            }
        }
        *self.token.lock() = Some(t.clone());
    }

    /// Ouvre la session, le lecteur Connect et Spirc.
    async fn connect(self: &Arc<Self>, credentials: Credentials) -> Result<(), String> {
        self.set_status(Status::Connecting);
        let config = SessionConfig { client_id: oauth::CLIENT_ID.to_string(), device_id: self.device_id.clone(), ..SessionConfig::default() };
        let session = Session::new(config, self.cache());
        let mixer = SoftMixer::open(MixerConfig::default()).map_err(readable)?;
        let shared = self.connect.clone();
        let player = Player::new(player_config(), session.clone(), mixer.get_soft_volume(), move || Box::new(BufferSink(shared)));
        self.watch_connect_player(&player);
        let config = ConnectConfig { name: self.device_name.clone(), device_type: self.device_type, initial_volume: u16::MAX / 2, ..ConnectConfig::default() };
        let (spirc, task) = Spirc::new(config, session.clone(), credentials, player, Arc::new(mixer)).await.map_err(readable)?;
        self.rt.spawn(task);
        let user = session.username();
        *self.session.lock() = Some(session);
        *self.spirc.lock() = Some(spirc);
        self.set_status(Status::Connected { user });
        Ok(())
    }

    /// Suit le lecteur de Spotify Connect.
    fn watch_connect_player(self: &Arc<Self>, player: &Arc<Player>) {
        let mut events = player.get_player_event_channel();
        let me = self.clone();
        self.rt.spawn(async move {
            while let Some(ev) = events.recv().await {
                match ev {
                    PlayerEvent::Playing { position_ms, .. } => {
                        if !me.connect_active.swap(true, Ordering::AcqRel) {
                            me.connect.reopen();
                            me.connect.flush(Some(position_ms as u64));
                            let _ = me.tx.send(Event::ConnectStart);
                        } else {
                            let _ = me.tx.send(Event::ConnectResume);
                        }
                    }
                    PlayerEvent::Paused { .. } => {
                        let _ = me.tx.send(Event::ConnectPause);
                    }
                    PlayerEvent::TrackChanged { audio_item } => {
                        me.connect.set_info(info_from(&audio_item));
                        if me.connect_active.load(Ordering::Acquire) {
                            me.connect.flush(Some(0));
                        }
                    }
                    PlayerEvent::Seeked { position_ms, .. } | PlayerEvent::PositionCorrection { position_ms, .. } => {
                        me.connect.flush(Some(position_ms as u64));
                    }
                    PlayerEvent::Stopped { .. } | PlayerEvent::SessionDisconnected { .. } => {
                        if me.connect_active.swap(false, Ordering::AcqRel) {
                            me.connect.set_ended();
                            let _ = me.tx.send(Event::ConnectStop);
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    /// Attend que la session soit ouverte.
    fn wait_session(&self, timeout: Duration) -> Result<Session, String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(s) = self.session.lock().clone() {
                return Ok(s);
            }
            match &*self.status.lock() {
                Status::LoggedOut => return Err("Spotify : pas connecté (menu Bibliothèque › Se connecter)".into()),
                Status::Failed(e) => return Err(format!("Spotify : {e}")),
                _ => {}
            }
            if Instant::now() > deadline {
                return Err("Spotify : connexion trop longue".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Jeton pour l'API web : celui de la connexion (renouvelé), sinon celui de la session.
    fn web_token(&self) -> Result<String, Fault> {
        let current = self.token.lock().clone();
        if let Some(t) = current {
            if !t.is_expired() {
                return Ok(t.access_token);
            }
            if let Ok(n) = oauth::refresh(&t) {
                self.save_token(&n);
                return Ok(n.access_token);
            }
        }
        let session = self.session.lock().clone().ok_or(Fault::NotConnected)?;
        let t = self.rt.block_on(session.login5().auth_token()).map_err(|e| Fault::from_librespot(&e))?;
        Ok(t.access_token)
    }

    // --- Bibliothèque ---------------------------------------------------------------------------------

    /// Échéance de l'attente demandée par Spotify sur ce canal, si elle court encore.
    fn wait_until(&self, channel: &str) -> Option<u64> {
        self.waits.lock().get(channel).copied().filter(|u| *u > now_secs())
    }

    fn set_wait(&self, channel: &str, wait: Option<Duration>) -> u64 {
        let until = now_secs() + wait.unwrap_or(DEFAULT_WAIT).as_secs().max(1);
        let mut w = self.waits.lock();
        w.insert(channel.to_string(), until);
        w.retain(|_, u| *u > now_secs());
        self.lists.store_waits(&w);
        until
    }

    /// La session, pour la Bibliothèque : on attend un peu si la connexion est en cours.
    fn library_session(&self) -> Result<Session, Fault> {
        if let Some(s) = self.session.lock().clone() {
            return Ok(s);
        }
        if matches!(*self.status.lock(), Status::Connecting) {
            return self.wait_session(Duration::from_secs(20)).map_err(|_| Fault::NotConnected);
        }
        Err(Fault::NotConnected)
    }

    fn send(&self, id: u64, reply: LibraryReply) {
        let _ = self.tx.send(Event::Library { request: id, reply });
    }

    /// Sert une requête de la Bibliothèque (dans un fil à part).
    fn serve(self: &Arc<Self>, id: u64, req: LibraryRequest, force: bool) {
        match req {
            LibraryRequest::RadioTop => self.send(id, radio_reply(radios::top())),
            LibraryRequest::RadioSearch(q) => self.send(id, radio_reply(radios::search(&q))),
            LibraryRequest::Search(q) => self.serve_search(id, &q),
            LibraryRequest::Playlists { liked_label } => {
                let liked = LibraryItem::new(LibraryKind::Playlist, liked_label, "", "spotify:collection");
                self.serve_list(id, Some("playlists".into()), force, move |mut v| {
                    v.insert(0, liked.clone());
                    v
                }, |me, s, _| me.rt.block_on(internal::rootlist(s)));
            }
            LibraryRequest::Albums => self.serve_list(id, Some("albums".into()), force, |v| v, |me, s, progress| me.fetch_albums(s, progress)),
            LibraryRequest::Tracks(uri) => {
                let key = cache_key_for_uri(&uri);
                self.serve_list(id, key, force, |v| v, move |me, s, progress| me.fetch_tracks(s, &uri, progress));
            }
        }
    }

    /// Une liste : gardée si elle est récente, sinon redemandée (la liste gardée s'affiche en attendant).
    fn serve_list(
        self: &Arc<Self>,
        id: u64,
        key: Option<String>,
        force: bool,
        decorate: impl Fn(Vec<LibraryItem>) -> Vec<LibraryItem>,
        fetch: impl FnOnce(&Arc<Self>, &Session, &mut dyn FnMut(Vec<LibraryItem>, usize)) -> Result<Vec<LibraryItem>, Fault>,
    ) {
        let cached = key.as_deref().and_then(|k| self.lists.load(k));
        let refreshable = key.is_some();
        let reply = |items: Vec<LibraryItem>, complete: bool, cached_age: Option<u64>, notice: Option<Notice>, last: bool| LibraryReply {
            items: decorate(items),
            complete,
            cached_age,
            notice,
            progress: None,
            last,
            refreshable,
        };
        let from_cache = |notice: Option<Notice>| match &cached {
            Some((items, age)) => reply(items.clone(), true, Some(*age), notice, true),
            None => LibraryReply { refreshable, ..LibraryReply::failed(notice.unwrap_or(Notice::NotConnected)) },
        };
        if let Some((items, age)) = &cached {
            if !force && *age < FRESH.as_secs() {
                self.send(id, reply(items.clone(), true, Some(*age), None, true));
                return;
            }
        }
        if let Some(until) = self.wait_until(INTERNAL) {
            self.send(id, from_cache(Some(Notice::Wait { until, partial: false })));
            return;
        }
        let session = match self.library_session() {
            Ok(s) => s,
            Err(f) => {
                self.send(id, from_cache(Some(if f == Fault::NotConnected { Notice::NotConnected } else { Notice::Failed(f) })));
                return;
            }
        };
        if let Some((items, age)) = &cached {
            self.send(id, reply(items.clone(), true, Some(*age), None, false));
        }
        let show_progress = cached.is_none();
        let mut last_sent = Instant::now();
        let mut progress = |items: Vec<LibraryItem>, total: usize| {
            if show_progress && last_sent.elapsed() >= Duration::from_millis(700) {
                last_sent = Instant::now();
                let n = items.len();
                let mut r = reply(items, false, None, None, false);
                r.progress = Some((n, total));
                self.send(id, r);
            }
        };
        match fetch(self, &session, &mut progress) {
            Ok(items) => {
                if let Some(k) = &key {
                    self.lists.store(k, &items);
                }
                self.send(id, reply(items, true, None, None, true));
            }
            Err(Fault::RateLimited(wait)) => {
                let until = self.set_wait(INTERNAL, wait);
                self.send(id, from_cache(Some(Notice::Wait { until, partial: false })));
            }
            Err(f) => {
                log::warn!("Spotify (bibliothèque) : {f:?}");
                self.send(id, from_cache(Some(Notice::Failed(f))));
            }
        }
    }

    /// Morceaux d'une liste d'adresses, avec leurs noms (par paquets ; `progress` reçoit les morceaux prêts).
    fn named_tracks(&self, session: &Session, uris: &[String], progress: &mut dyn FnMut(Vec<LibraryItem>, usize)) -> Result<Vec<LibraryItem>, Fault> {
        let total = uris.len();
        let meta = self.rt.block_on(internal::metadata(session, uris, |m| {
            let ready: Vec<LibraryItem> = internal::track_items(uris, m).into_iter().filter(|i| m.contains_key(&i.uri)).collect();
            progress(ready, total);
        }))?;
        Ok(internal::track_items(uris, &meta))
    }

    fn fetch_tracks(&self, session: &Session, uri: &str, progress: &mut dyn FnMut(Vec<LibraryItem>, usize)) -> Result<Vec<LibraryItem>, Fault> {
        if uri == "spotify:collection" {
            let all = self.rt.block_on(internal::collection(session))?;
            let uris: Vec<String> = all.into_iter().map(|c| c.uri).filter(|u| internal::is_playable_uri(u)).take(internal::MAX_ITEMS).collect();
            self.named_tracks(session, &uris, progress)
        } else if let Some(id) = uri.strip_prefix("spotify:playlist:") {
            let (uris, _) = self.rt.block_on(internal::playlist_uris(session, id))?;
            self.named_tracks(session, &uris, progress)
        } else if uri.starts_with("spotify:album:") {
            let (uris, album_artists) = self.rt.block_on(internal::album_uris(session, uri))?;
            let mut items = self.named_tracks(session, &uris, progress)?;
            for it in items.iter_mut().filter(|i| i.subtitle.is_empty()) {
                it.subtitle = album_artists.clone();
            }
            Ok(items)
        } else {
            Ok(Vec::new())
        }
    }

    fn fetch_albums(&self, session: &Session, progress: &mut dyn FnMut(Vec<LibraryItem>, usize)) -> Result<Vec<LibraryItem>, Fault> {
        let all = self.rt.block_on(internal::collection(session))?;
        let uris: Vec<String> = all.into_iter().map(|c| c.uri).filter(|u| u.starts_with("spotify:album:")).take(internal::MAX_ITEMS).collect();
        let total = uris.len();
        let to_items = |m: &HashMap<String, internal::Meta>, only_ready: bool| -> Vec<LibraryItem> {
            uris.iter()
                .filter_map(|u| match m.get(u) {
                    Some(a) => Some(LibraryItem::new(LibraryKind::Album, if a.name.is_empty() { u.as_str() } else { a.name.as_str() }, a.artists.clone(), u.clone())),
                    None if only_ready => None,
                    None => Some(LibraryItem::new(LibraryKind::Album, u.clone(), "", u.clone())),
                })
                .collect()
        };
        let meta = self.rt.block_on(internal::metadata(session, &uris, |m| progress(to_items(m, true), total)))?;
        Ok(to_items(&meta, false))
    }

    /// Recherche : titres par le canal interne (envoyés d'abord), puis albums et playlists par l'API web si
    /// elle ne demande pas de patienter.
    fn serve_search(self: &Arc<Self>, id: u64, query: &str) {
        if let Some(until) = self.wait_until(INTERNAL) {
            self.send(id, LibraryReply::failed(Notice::Wait { until, partial: false }));
            return;
        }
        let session = match self.library_session() {
            Ok(s) => s,
            Err(f) => {
                self.send(id, LibraryReply::failed(if f == Fault::NotConnected { Notice::NotConnected } else { Notice::Failed(f) }));
                return;
            }
        };
        let tracks = self.rt.block_on(internal::search_tracks(&session, query)).and_then(|uris| self.named_tracks(&session, &uris, &mut |_, _| {}));
        let mut items = match tracks {
            Ok(t) => t,
            Err(Fault::RateLimited(wait)) => {
                let until = self.set_wait(INTERNAL, wait);
                self.send(id, LibraryReply::failed(Notice::Wait { until, partial: false }));
                return;
            }
            Err(f) => {
                log::warn!("Spotify (recherche) : {f:?}");
                Vec::new()
            }
        };
        let mut notice = None;
        if let Some(until) = self.wait_until(WEB) {
            notice = Some(Notice::Wait { until, partial: true });
        } else {
            self.send(id, LibraryReply { last: false, ..LibraryReply::done(items.clone()) });
            match self.web_token().and_then(|tok| webapi::search_albums_playlists(&tok, query)) {
                Ok(more) => items.extend(more),
                Err(Fault::RateLimited(wait)) => {
                    let until = self.set_wait(WEB, wait);
                    notice = Some(Notice::Wait { until, partial: true });
                }
                Err(f) => log::warn!("Spotify (recherche, API web) : {f:?}"),
            }
        }
        self.send(id, LibraryReply { notice, ..LibraryReply::done(items) });
    }

    fn open_track(self: &Arc<Self>, uri: &str) -> Result<Box<dyn PcmSource>, AudioError> {
        let session = self.wait_session(Duration::from_secs(25)).map_err(AudioError::Open)?;
        let id = SpotifyUri::from_uri(uri).map_err(|e| AudioError::Open(format!("{uri} : {e}")))?;
        let shared = Shared::new();
        let sh = shared.clone();
        let player = Player::new(player_config(), session, Box::new(NoOpVolume), move || Box::new(BufferSink(sh)));
        let mut events = player.get_player_event_channel();
        let sh = shared.clone();
        self.rt.spawn(async move {
            while let Some(ev) = events.recv().await {
                match ev {
                    PlayerEvent::TrackChanged { audio_item } => sh.set_info(info_from(&audio_item)),
                    PlayerEvent::EndOfTrack { .. } | PlayerEvent::Stopped { .. } => sh.set_ended(),
                    PlayerEvent::Unavailable { .. } => sh.set_error("morceau indisponible sur Spotify".into()),
                    _ => {}
                }
            }
        });
        player.load(id, true, 0);
        shared.wait_ready(Duration::from_secs(25)).map_err(AudioError::Open)?;
        let info = shared.info().unwrap_or_else(|| TrackInfo { sample_rate: sink::RATE, channels: 2, seekable: true, codec: "SPOTIFY".into(), ..Default::default() });
        let p_seek = player.clone();
        let p_stop = player;
        Ok(Box::new(BufferSource::new(shared, info, Box::new(move |ms| p_seek.seek(ms as u32)), Box::new(move || p_stop.stop()))))
    }
}

impl Spotify {
    /// `dir` : dossier de l'appli pour Spotify ; `device_name` : nom de l'appareil Connect ;
    /// `open_url` : ouvre une page dans le navigateur.
    pub fn new(dir: PathBuf, device_name: &str, phone: bool, open_url: Box<dyn Fn(&str) + Send + Sync>) -> Spotify {
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_name("alpagamp-spotify").enable_all().build().expect("tokio");
        let _ = std::fs::create_dir_all(&dir);
        let id_path = dir.join("device-id");
        let device_id = std::fs::read_to_string(&id_path).ok().map(|s| s.trim().to_string()).filter(|s| s.len() >= 16).unwrap_or_else(|| {
            let mut b = [0u8; 20];
            let _ = getrandom::fill(&mut b);
            let id: String = b.iter().map(|x| format!("{x:02x}")).collect();
            let _ = std::fs::write(&id_path, &id);
            id
        });
        let token = std::fs::read(dir.join("oauth.json")).ok().and_then(|b| serde_json::from_slice(&b).ok());
        let lists = ListCache::new(&dir);
        let waits = Mutex::new(lists.load_waits());
        let (tx, events) = crossbeam_channel::unbounded();
        let inner = Arc::new(Inner {
            rt,
            dir,
            device_id,
            device_name: device_name.to_string(),
            device_type: if phone { DeviceType::Smartphone } else { DeviceType::Computer },
            open_url,
            tx,
            status: Mutex::new(Status::LoggedOut),
            session: Mutex::new(None),
            spirc: Mutex::new(None),
            connect: Shared::new(),
            connect_active: AtomicBool::new(false),
            token: Mutex::new(token),
            next_request: AtomicU64::new(1),
            login: Mutex::new(None),
            lists,
            waits,
        });
        let s = Spotify { inner, events };
        s.reconnect();
        s
    }

    /// Reprend la session avec les identifiants gardés, s'il y en a.
    fn reconnect(&self) {
        let Some(creds) = self.inner.cache().and_then(|c| c.credentials()) else { return };
        let inner = self.inner.clone();
        self.inner.rt.spawn(async move {
            if let Err(e) = inner.connect(creds).await {
                log::warn!("Spotify : {e}");
                inner.set_status(Status::Failed(e));
            }
        });
    }

    pub fn status(&self) -> Status {
        self.inner.status.lock().clone()
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.status(), Status::Connected { .. })
    }

    pub fn poll_events(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }

    /// Se connecter : ouvre le navigateur, attend l'autorisation, ouvre la session. Pendant l'attente, un
    /// nouvel appel rouvre la même page (l'onglet a pu être fermé).
    pub fn login(&self) {
        match self.status() {
            Status::Authorizing => {
                self.reopen_login();
                return;
            }
            Status::Connecting | Status::Connected { .. } => return,
            _ => {}
        }
        let attempt = Arc::new(Attempt::default());
        *self.inner.login.lock() = Some(attempt.clone());
        self.inner.set_status(Status::Authorizing);
        let inner = self.inner.clone();
        std::thread::Builder::new()
            .name("alpagamp-spotify-login".into())
            .spawn(move || {
                let result = oauth::authorize(&*inner.open_url, &attempt, oauth::AUTHORIZE_TIMEOUT);
                // Une autre tentative a pris la place de celle-ci (annulée puis relancée) : elle décide.
                let current = inner.login.lock().as_ref().is_some_and(|a| Arc::ptr_eq(a, &attempt));
                if !current {
                    return;
                }
                *inner.login.lock() = None;
                let token = match result {
                    Ok(t) => t,
                    Err(AuthError::Cancelled | AuthError::TimedOut) => {
                        inner.set_status(Status::LoggedOut);
                        return;
                    }
                    Err(AuthError::Failed(e)) => {
                        inner.set_status(Status::Failed(e));
                        return;
                    }
                };
                inner.save_token(&token);
                let creds = Credentials::with_access_token(&token.access_token);
                let i2 = inner.clone();
                if let Err(e) = inner.rt.block_on(async move { i2.connect(creds).await }) {
                    inner.set_status(Status::Failed(e));
                }
            })
            .ok();
    }

    /// Rouvre la page de connexion (même page, même attente).
    pub fn reopen_login(&self) {
        let url = self.inner.login.lock().as_ref().and_then(|a| a.url());
        if let Some(u) = url {
            (self.inner.open_url)(&u);
        }
    }

    /// Abandonne la connexion en cours : retour à « Se connecter ».
    pub fn cancel_login(&self) {
        if let Some(a) = self.inner.login.lock().take() {
            a.cancel();
        }
        if matches!(self.status(), Status::Authorizing) {
            self.inner.set_status(Status::LoggedOut);
        }
    }

    /// Se déconnecter et oublier les identifiants (et les listes gardées).
    pub fn logout(&self) {
        self.cancel_login();
        if let Some(s) = self.inner.spirc.lock().take() {
            let _ = s.shutdown();
        }
        if let Some(s) = self.inner.session.lock().take() {
            s.shutdown();
        }
        let _ = std::fs::remove_dir_all(self.inner.dir.join("credentials"));
        let _ = std::fs::remove_file(self.inner.token_path());
        *self.inner.token.lock() = None;
        self.inner.lists.clear();
        self.inner.waits.lock().clear();
        self.inner.set_status(Status::LoggedOut);
    }

    /// Fabrique de source pour un morceau (`spotify:track:…`), pour le moteur audio.
    pub fn opener(&self, uri: &str) -> Opener {
        let inner = self.inner.clone();
        let uri = uri.to_string();
        Arc::new(move || inner.open_track(&uri))
    }

    /// Fabrique de la source « Spotify Connect » (ce que joue l'appli Spotify sur cet appareil).
    pub fn connect_opener(&self) -> Opener {
        let inner = self.inner.clone();
        Arc::new(move || {
            let shared = inner.connect.clone();
            let info = shared.info().unwrap_or_else(|| TrackInfo { sample_rate: sink::RATE, channels: 2, seekable: true, codec: "SPOTIFY".into(), ..Default::default() });
            let i_seek = inner.clone();
            let i_drop = inner.clone();
            Ok(Box::new(BufferSource::new(
                shared,
                info,
                Box::new(move |ms| {
                    if let Some(s) = &*i_seek.spirc.lock() {
                        let _ = s.set_position_ms(ms as u32);
                    }
                }),
                Box::new(move || {
                    // Alpagamp passe à autre chose : Spotify Connect se met en pause ; une reprise depuis l'appli
                    // Spotify relancera la lecture ici (ConnectStart).
                    if i_drop.connect_active.swap(false, Ordering::AcqRel) {
                        if let Some(s) = &*i_drop.spirc.lock() {
                            let _ = s.pause();
                        }
                    }
                }),
            )) as Box<dyn PcmSource>)
        })
    }

    pub fn connect_active(&self) -> bool {
        self.inner.connect_active.load(Ordering::Acquire)
    }

    pub fn connect_command(&self, c: ConnectCommand) {
        let spirc = self.inner.spirc.lock();
        let Some(s) = spirc.as_ref() else { return };
        let _ = match c {
            ConnectCommand::Play => s.play(),
            ConnectCommand::Pause => s.pause(),
            ConnectCommand::Next => s.next(),
            ConnectCommand::Previous => s.prev(),
            ConnectCommand::Seek(ms) => s.set_position_ms(ms as u32),
        };
    }

    /// Lance une requête de bibliothèque ; les réponses arrivent en [`Event::Library`] avec ce numéro. Une
    /// liste gardée de moins de 15 min est rendue telle quelle.
    pub fn request(&self, req: LibraryRequest) -> u64 {
        self.spawn_request(req, false)
    }

    /// Comme [`Spotify::request`], mais redemande la liste à Spotify (« Actualiser »), sauf si Spotify a demandé
    /// de patienter.
    pub fn refresh(&self, req: LibraryRequest) -> u64 {
        self.spawn_request(req, true)
    }

    fn spawn_request(&self, req: LibraryRequest, force: bool) -> u64 {
        let id = self.inner.next_request.fetch_add(1, Ordering::Relaxed);
        let inner = self.inner.clone();
        std::thread::Builder::new().name("alpagamp-bibliotheque".into()).spawn(move || inner.serve(id, req, force)).ok();
        id
    }
}

fn radio_reply(r: Result<Vec<LibraryItem>, String>) -> LibraryReply {
    match r {
        Ok(items) => LibraryReply::done(items),
        Err(e) => LibraryReply::failed(Notice::Failed(Fault::Network(e))),
    }
}

impl Drop for Spotify {
    fn drop(&mut self) {
        if let Some(s) = self.inner.spirc.lock().take() {
            let _ = s.shutdown();
        }
    }
}
