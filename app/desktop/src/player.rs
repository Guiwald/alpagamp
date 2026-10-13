//! Le contrôleur de lecture : playlist + moteur audio + réglages.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use alpagamp_audio::{Decoder, Engine, Event, Media, PlayState, Token, TrackInfo};
use alpagamp_core::eq::EqSettings;
use alpagamp_core::formats::{is_audio_path, is_playlist_path, read_playlist_bytes, write_m3u};
use alpagamp_core::time::format_duration;
use alpagamp_core::{Entry, Playlist, Settings, Source};
use alpagamp_ui::{MainView, Transport};

pub struct Player {
    pub engine: Engine,
    pub playlist: Playlist,
    pub settings: Settings,
    token_seq: Token,
    /// Morceau entendu : (jeton, identifiant de l'entrée).
    playing: Option<(Token, u64)>,
    /// Morceau suivant préparé pour l'enchaînement : (jeton, identifiant de l'entrée).
    planned: Option<(Token, u64)>,
    /// Entrée dont la lecture vient de commencer (pour les préréglages EQ automatiques).
    track_changed: Option<u64>,
    scanner: Scanner,
    pub info: Option<TrackInfo>,
    /// Arrêt en fondu : (début, volume de départ).
    fade: Option<(Instant, u8)>,
    pub stop_after_current: bool,
    /// Message d'erreur récent (affiché dans le titre).
    pub last_error: Option<(String, Instant)>,
}

impl Player {
    pub fn new(settings: Settings) -> Player {
        let engine = Engine::new();
        let p = Player {
            engine,
            playlist: Playlist::new(),
            settings,
            token_seq: 0,
            playing: None,
            planned: None,
            track_changed: None,
            scanner: Scanner::new(),
            info: None,
            fade: None,
            stop_after_current: false,
            last_error: None,
        };
        p.apply_volume();
        p.engine.set_eq(p.settings.eq);
        p
    }

    fn next_token(&mut self) -> Token {
        self.token_seq += 1;
        self.token_seq
    }

    pub fn apply_volume(&self) {
        self.engine.set_volume(self.settings.volume as f32 / 255.0);
        self.engine.set_balance(self.settings.balance as f32 / 127.0);
    }

    pub fn state(&self) -> PlayState {
        self.engine.status().state
    }

    pub fn set_eq(&mut self, eq: EqSettings) {
        self.settings.eq = eq;
        self.engine.set_eq(eq);
    }

    /// Entrée dont la lecture vient de commencer, une seule fois.
    pub fn take_track_changed(&mut self) -> Option<u64> {
        self.track_changed.take()
    }

    /// À appeler après toute modification de la playlist (ajout, tri, suppression…).
    pub fn playlist_changed(&mut self) {
        self.scanner.pending = true;
        self.replan();
    }

    /// Index dans la playlist du morceau entendu (s'il y est toujours).
    pub fn playing_index(&self) -> Option<usize> {
        self.playing.and_then(|(_, id)| self.playlist.index_of(id))
    }

    // --- Playlist ---------------------------------------------------------------------------------

    /// Développe des chemins (fichiers, dossiers, playlists) en entrées de playlist.
    pub fn expand(paths: &[PathBuf]) -> Vec<Entry> {
        let mut out = Vec::new();
        for p in paths {
            if p.is_dir() {
                let mut files = Vec::new();
                collect_dir(p, &mut files, 0);
                files.sort_by(|a, b| alpagamp_core::playlist::natural_cmp(&a.to_string_lossy().to_lowercase(), &b.to_string_lossy().to_lowercase()));
                out.extend(files.into_iter().map(|f| Entry::new(Source::File(f))));
            } else if is_playlist_path(p) {
                if let Ok(bytes) = std::fs::read(p) {
                    out.extend(read_playlist_bytes(&bytes, p.parent()));
                }
            } else {
                out.push(Entry::new(Source::parse(&p.to_string_lossy(), None)));
            }
        }
        out
    }

    /// Ajoute à la playlist ; avec `play`, remplace la liste et lance la lecture.
    pub fn open(&mut self, paths: &[PathBuf], play: bool) {
        let entries = Self::expand(paths);
        if entries.is_empty() {
            return;
        }
        if play {
            self.stop();
            self.playlist.clear();
            self.playlist.extend(entries);
            self.play_index(0);
            self.playlist_changed();
        } else {
            self.playlist.extend(entries);
            self.playlist_changed();
        }
    }

    pub fn load_playlist_file(&mut self, path: &Path) {
        if let Ok(bytes) = std::fs::read(path) {
            self.playlist.clear();
            self.playlist.extend(read_playlist_bytes(&bytes, path.parent()));
            if let Some(i) = self.settings.resume_index.filter(|&i| i < self.playlist.len()) {
                self.playlist.set_current(Some(i));
            }
            self.playlist.dirty = false;
            self.playlist_changed();
        }
    }

    pub fn save_playlist_file(&self, path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, write_m3u(self.playlist.entries(), true));
    }

    // --- Transport --------------------------------------------------------------------------------

    fn media_for(&self, i: usize) -> Option<Media> {
        let e = self.playlist.entries().get(i)?;
        match &e.source {
            Source::File(p) => Some(Media::File(p.clone())),
            // Flux et Spotify arrivent dans les versions suivantes.
            Source::Url(_) | Source::Spotify(_) => None,
        }
    }

    pub fn play_index(&mut self, i: usize) {
        if i >= self.playlist.len() {
            return;
        }
        self.playlist.set_current(Some(i));
        self.fade = None;
        match self.media_for(i) {
            Some(m) => {
                let t = self.next_token();
                self.playing = Some((t, self.playlist.entries()[i].id));
                self.planned = None;
                self.engine.play(t, m, 0, false);
            }
            None => {
                self.error(format!("{} : source pas encore prise en charge", self.playlist.entries()[i].display_title()));
            }
        }
    }

    /// Bouton « Lecture » : reprend la pause, redémarre le morceau, ou joue le courant.
    pub fn play_pressed(&mut self) {
        match self.state() {
            PlayState::Paused => self.engine.resume(),
            _ => {
                let i = self.playlist.current().unwrap_or(0);
                self.play_index(i);
            }
        }
    }

    pub fn pause_pressed(&mut self) {
        match self.state() {
            PlayState::Playing => self.engine.pause(),
            PlayState::Paused => self.engine.resume(),
            PlayState::Stopped => {}
        }
    }

    pub fn stop(&mut self) {
        self.fade = None;
        self.engine.stop();
        self.playing = None;
        self.planned = None;
        self.info = None;
        self.apply_volume();
    }

    pub fn stop_with_fadeout(&mut self) {
        if self.state() == PlayState::Playing {
            self.fade = Some((Instant::now(), self.settings.volume));
        } else {
            self.stop();
        }
    }

    pub fn next(&mut self) {
        if let Some(i) = self.playlist.next_index(self.settings.shuffle, self.settings.repeat) {
            let was_stopped = self.state() == PlayState::Stopped;
            if was_stopped {
                self.playlist.set_current(Some(i));
            } else {
                self.play_index(i);
            }
        }
    }

    pub fn previous(&mut self) {
        if let Some(i) = self.playlist.prev_index(self.settings.shuffle, self.settings.repeat) {
            if self.state() == PlayState::Stopped {
                self.playlist.set_current(Some(i));
            } else {
                self.play_index(i);
            }
        }
    }

    pub fn first(&mut self) {
        if !self.playlist.is_empty() {
            self.play_index(0);
        }
    }

    pub fn last(&mut self) {
        if !self.playlist.is_empty() {
            self.play_index(self.playlist.len() - 1);
        }
    }

    pub fn seek_relative(&mut self, delta_ms: i64) {
        let st = self.engine.status();
        if st.state == PlayState::Stopped || !st.info.as_ref().is_some_and(|i| i.seekable) {
            return;
        }
        let target = (st.position_ms as i64 + delta_ms).max(0) as u64;
        let target = st.info.and_then(|i| i.duration_ms).map(|d| target.min(d.saturating_sub(500))).unwrap_or(target);
        self.engine.seek(target);
    }

    pub fn seek(&mut self, ms: u64) {
        self.engine.seek(ms);
    }

    pub fn set_volume(&mut self, v: u8) {
        self.settings.volume = v;
        self.fade = None;
        self.apply_volume();
    }

    pub fn set_balance(&mut self, b: i8) {
        self.settings.balance = b;
        self.apply_volume();
    }

    pub fn toggle_shuffle(&mut self) {
        self.settings.shuffle = !self.settings.shuffle;
        self.playlist.rebuild_shuffle();
        self.replan();
    }

    pub fn toggle_repeat(&mut self) {
        self.settings.repeat = !self.settings.repeat;
        self.replan();
    }

    pub fn toggle_manual_advance(&mut self) {
        self.settings.manual_advance = !self.settings.manual_advance;
        self.replan();
    }

    fn error(&mut self, msg: String) {
        log::warn!("{msg}");
        self.last_error = Some((msg, Instant::now()));
    }

    /// Prépare (ou annule) l'enchaînement sans blanc vers le morceau suivant.
    pub fn replan(&mut self) {
        if self.playing.is_none() || self.stop_after_current {
            if self.planned.take().is_some() {
                self.engine.set_next(None);
            }
            return;
        }
        let s = &self.settings;
        let next = self.playlist.peek_auto_advance(s.shuffle, s.repeat, s.manual_advance);
        match next.and_then(|i| self.media_for(i).map(|m| (self.playlist.entries()[i].id, m))) {
            Some((id, m)) => {
                if self.planned.is_some_and(|(_, pid)| pid == id) {
                    return;
                }
                let t = self.next_token();
                self.planned = Some((t, id));
                self.engine.set_next(Some((t, m)));
            }
            None => {
                if self.planned.take().is_some() {
                    self.engine.set_next(None);
                }
            }
        }
    }

    // --- Boucle -----------------------------------------------------------------------------------

    /// À appeler régulièrement : traite les événements du moteur et le fondu.
    pub fn tick(&mut self) {
        self.scan_metadata();
        if let Some((start, from)) = self.fade {
            let k = start.elapsed().as_secs_f32() / 2.0;
            if k >= 1.0 {
                self.stop();
            } else {
                let v = from as f32 * (1.0 - k);
                self.engine.set_volume(v / 255.0);
            }
        }
        for ev in self.engine.poll_events() {
            match ev {
                Event::Started { token, info } => {
                    if let Some((_, id)) = self.playing.filter(|(t, _)| *t == token) {
                        self.on_track_info(token, &info);
                        self.info = Some(info);
                        self.track_changed = Some(id);
                        self.replan();
                    }
                }
                Event::InfoChanged { token, info } => {
                    if self.playing.is_some_and(|(t, _)| t == token) {
                        self.on_track_info(token, &info);
                        self.info = Some(info);
                    }
                }
                Event::Advanced { token, info, .. } => {
                    if let Some((pt, id)) = self.planned.take().filter(|(t, _)| *t == token) {
                        self.playing = Some((pt, id));
                        if let Some(i) = self.playlist.index_of(id) {
                            self.playlist.set_current(Some(i));
                        }
                        self.on_track_info(token, &info);
                        self.info = Some(info);
                        self.track_changed = Some(id);
                        if self.stop_after_current {
                            self.stop_after_current = false;
                            self.stop();
                        } else {
                            self.replan();
                        }
                    }
                }
                Event::Ended { token } => {
                    if self.playing.is_some_and(|(t, _)| t == token) {
                        self.playing = None;
                        self.info = None;
                        if self.stop_after_current {
                            self.stop_after_current = false;
                            continue;
                        }
                        let s = &self.settings;
                        if let Some(i) = self.playlist.auto_advance(s.shuffle, s.repeat, s.manual_advance) {
                            self.play_index(i);
                        }
                    }
                }
                Event::Error { token, message } => {
                    let is_current = self.playing.is_some_and(|(t, _)| t == token);
                    let is_planned = self.planned.is_some_and(|(t, _)| t == token);
                    if is_current || is_planned {
                        self.error(message);
                    }
                    if is_planned {
                        self.planned = None;
                    }
                    if is_current {
                        // Morceau illisible : on passe au suivant (comme Winamp), sans boucler à l'infini.
                        self.playing = None;
                        let s = &self.settings;
                        if let Some(i) = self.playlist.auto_advance(s.shuffle, false, false) {
                            self.play_index(i);
                        }
                    }
                }
            }
        }
    }

    /// Met à jour l'entrée de playlist avec le titre et la durée lus dans le fichier.
    fn on_track_info(&mut self, token: Token, info: &TrackInfo) {
        let Some((_, id)) = self.playing.filter(|(t, _)| *t == token) else { return };
        let Some(i) = self.playlist.index_of(id) else { return };
        if let Some(e) = self.playlist.entry_mut(i) {
            if let Some(t) = info.display_title() {
                e.title = Some(t);
            }
            if info.duration_ms.is_some() {
                e.duration_ms = info.duration_ms;
            }
            e.meta_loaded = true;
        }
    }

    /// Envoie les entrées sans métadonnées au lecteur d'étiquettes et applique ses réponses.
    fn scan_metadata(&mut self) {
        if std::mem::take(&mut self.scanner.pending) {
            for e in self.playlist.entries() {
                if let Source::File(p) = &e.source {
                    if !e.meta_loaded && self.scanner.queued.insert(e.id) {
                        let _ = self.scanner.tx.send((e.id, p.clone()));
                    }
                }
            }
        }
        while let Ok((id, title, duration)) = self.scanner.rx.try_recv() {
            if let Some(i) = self.playlist.index_of(id) {
                if let Some(e) = self.playlist.entry_mut(i) {
                    if !e.meta_loaded {
                        if title.is_some() {
                            e.title = title;
                        }
                        e.duration_ms = duration.or(e.duration_ms);
                        e.meta_loaded = true;
                    }
                }
            }
        }
    }

    /// Données pour la fenêtre principale.
    pub fn main_view(&self, clock_ms: u64) -> MainView {
        let st = self.engine.status();
        let transport = match st.state {
            PlayState::Stopped => Transport::Stopped,
            PlayState::Playing => Transport::Playing,
            PlayState::Paused => Transport::Paused,
        };
        let info = st.info.as_ref();
        let title = match (&self.last_error, self.playlist.current_entry()) {
            (Some((msg, at)), _) if at.elapsed() < Duration::from_secs(4) => msg.clone(),
            (_, Some(e)) => {
                let n = self.playlist.current().unwrap_or(0) + 1;
                let dur = e.duration_ms.or(info.and_then(|i| i.duration_ms));
                match dur {
                    Some(d) => format!("{n}. {} ({})", e.display_title(), format_duration(d)),
                    None => format!("{n}. {}", e.display_title()),
                }
            }
            _ => format!("Alpagamp {}", env!("CARGO_PKG_VERSION")),
        };
        MainView {
            transport,
            buffering: st.buffering,
            position_ms: st.position_ms,
            duration_ms: info.and_then(|i| i.duration_ms),
            seekable: info.is_some_and(|i| i.seekable),
            title,
            kbps: info.and_then(|i| i.bitrate_kbps),
            khz: info.map(|i| (i.sample_rate + 500) / 1000),
            stereo: info.map(|i| i.channels >= 2),
            volume: self.settings.volume,
            balance: self.settings.balance,
            shuffle: self.settings.shuffle,
            repeat: self.settings.repeat,
            eq_open: false,
            pl_open: false,
            time_remaining: self.settings.time_remaining,
            always_on_top: self.settings.always_on_top,
            double_size: self.settings.double_size,
            active: true,
            vis: self.settings.vis.clone(),
            clock_ms,
        }
    }
}

/// Lit titres et durées en arrière-plan, sans bloquer l'interface.
struct Scanner {
    tx: Sender<(u64, PathBuf)>,
    rx: Receiver<(u64, Option<String>, Option<u64>)>,
    queued: HashSet<u64>,
    pending: bool,
}

impl Scanner {
    fn new() -> Scanner {
        let (tx, jobs) = channel::<(u64, PathBuf)>();
        let (done, rx) = channel();
        let _ = std::thread::Builder::new().name("alpagamp-tags".into()).spawn(move || {
            while let Ok((id, path)) = jobs.recv() {
                let (title, duration) = match Decoder::open_file(&path) {
                    Ok(d) => (d.info.display_title(), d.info.duration_ms),
                    Err(_) => (None, None),
                };
                if done.send((id, title, duration)).is_err() {
                    break;
                }
            }
        });
        Scanner { tx, rx, queued: HashSet::new(), pending: false }
    }
}

fn collect_dir(dir: &Path, out: &mut Vec<PathBuf>, depth: u32) {
    if depth > 16 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
            continue;
        }
        if p.is_dir() {
            collect_dir(&p, out, depth + 1);
        } else if is_audio_path(&p) {
            out.push(p);
        }
    }
}
