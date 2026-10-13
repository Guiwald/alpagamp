//! Spotify dans Alpagamp (ADR 0002, 0010, 0018), et l'annuaire de radios de la Bibliothèque.
//!
//! - Connexion par le navigateur, une seule fois : les identifiants réutilisables sont ensuite gardés par
//!   librespot dans le dossier de l'appli.
//! - Lecture : chaque morceau Spotify devient une source du moteur d'Alpagamp (égaliseur, visualisation, fondu
//!   enchaîné s'appliquent) ; librespot décode, un tampon fait le lien (`sink`).
//! - Spotify Connect : Alpagamp apparaît comme appareil dans l'appli Spotify ; ce qu'on y lance passe aussi par
//!   le moteur.
//! - Bibliothèque : playlists, titres likés, albums et recherche par l'API web ; radios par radio-browser.info.

pub mod oauth;
pub mod radios;
pub mod sink;
pub mod webapi;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use alpagamp_audio::{AudioError, Opener, PcmSource, TrackInfo};
use alpagamp_core::library::LibraryItem;
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

use crate::oauth::OAuthToken;
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
    Library { request: u64, result: Result<Vec<LibraryItem>, String> },
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
}

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
    fn web_token(&self) -> Result<String, String> {
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
        let session = self.session.lock().clone().ok_or("Spotify : pas connecté")?;
        let t = self.rt.block_on(session.login5().auth_token()).map_err(readable)?;
        Ok(t.access_token)
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

    /// Se connecter : ouvre le navigateur, attend l'autorisation, ouvre la session.
    pub fn login(&self) {
        if matches!(self.status(), Status::Authorizing | Status::Connecting | Status::Connected { .. }) {
            return;
        }
        let inner = self.inner.clone();
        std::thread::Builder::new()
            .name("alpagamp-spotify-login".into())
            .spawn(move || {
                inner.set_status(Status::Authorizing);
                let token = match oauth::authorize(&*inner.open_url) {
                    Ok(t) => t,
                    Err(e) => {
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

    /// Se déconnecter et oublier les identifiants.
    pub fn logout(&self) {
        if let Some(s) = self.inner.spirc.lock().take() {
            let _ = s.shutdown();
        }
        if let Some(s) = self.inner.session.lock().take() {
            s.shutdown();
        }
        let _ = std::fs::remove_dir_all(self.inner.dir.join("credentials"));
        let _ = std::fs::remove_file(self.inner.token_path());
        *self.inner.token.lock() = None;
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

    /// Lance une requête de bibliothèque ; la réponse arrive en [`Event::Library`] avec ce numéro.
    pub fn request(&self, req: LibraryRequest) -> u64 {
        let id = self.inner.next_request.fetch_add(1, Ordering::Relaxed);
        let inner = self.inner.clone();
        std::thread::Builder::new()
            .name("alpagamp-bibliotheque".into())
            .spawn(move || {
                let result = match &req {
                    LibraryRequest::RadioTop => radios::top(),
                    LibraryRequest::RadioSearch(q) => radios::search(q),
                    other => inner.web_token().and_then(|tok| match other {
                        LibraryRequest::Playlists { liked_label } => webapi::playlists(&tok, liked_label),
                        LibraryRequest::Tracks(uri) => webapi::tracks_of(&tok, uri),
                        LibraryRequest::Albums => webapi::albums(&tok),
                        LibraryRequest::Search(q) => webapi::search(&tok, q),
                        _ => Ok(Vec::new()),
                    }),
                };
                let _ = inner.tx.send(Event::Library { request: id, result });
            })
            .ok();
        id
    }
}

impl Drop for Spotify {
    fn drop(&mut self) {
        if let Some(s) = self.inner.spirc.lock().take() {
            let _ = s.shutdown();
        }
    }
}
