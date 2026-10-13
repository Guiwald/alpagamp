//! Moteur audio d'Alpagamp.
//!
//! Un fil de décodage ouvre les sources, décode, rééchantillonne vers la fréquence de sortie, applique
//! l'égaliseur, puis remplit une file partagée que le rappel audio vide. Le morceau suivant peut être
//! préparé à l'avance : il s'enchaîne sans blanc (gapless). La position affichée est celle de ce qui
//! est réellement joué.

pub mod decode;
pub mod dsp;
pub mod output;
pub mod resample;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use alpagamp_core::eq::EqSettings;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use parking_lot::Mutex;

pub use decode::{Decoder, TrackInfo};
use dsp::Dsp;
use output::{Output, Shared};
use resample::StereoResampler;

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AudioError {
    #[error("impossible d'ouvrir : {0}")]
    Open(String),
    #[error("format non pris en charge : {0}")]
    Unsupported(String),
    #[error("erreur de décodage : {0}")]
    Decode(String),
    #[error("déplacement impossible : {0}")]
    Seek(String),
}

/// Identifiant d'une demande de lecture (attribué par l'application).
pub type Token = u64;

/// Une source PCM : produit de l'audio stéréo entrelacé à sa propre fréquence.
pub trait PcmSource: Send {
    fn info(&self) -> &TrackInfo;
    /// Ajoute des trames stéréo à `out` ; renvoie 0 en fin de source.
    fn read(&mut self, out: &mut Vec<f32>) -> Result<usize, AudioError>;
    /// Se place à `ms` ; renvoie la position atteinte.
    fn seek(&mut self, ms: u64) -> Result<u64, AudioError>;
    /// Informations mises à jour en cours de lecture (titre d'un flux…), à prendre une fois.
    fn take_info_update(&mut self) -> Option<TrackInfo> {
        None
    }
}

impl PcmSource for Decoder {
    fn info(&self) -> &TrackInfo {
        &self.info
    }
    fn read(&mut self, out: &mut Vec<f32>) -> Result<usize, AudioError> {
        self.decode_next(out)
    }
    fn seek(&mut self, ms: u64) -> Result<u64, AudioError> {
        Decoder::seek(self, ms)
    }
}

/// Fabrique de source (ouverture éventuellement lente : réseau, Spotify).
pub type Opener = Arc<dyn Fn() -> Result<Box<dyn PcmSource>, AudioError> + Send + Sync>;

/// Ce qu'on demande de jouer.
#[derive(Clone)]
pub enum Media {
    File(PathBuf),
    Custom(Opener),
}

impl std::fmt::Debug for Media {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Media::File(p) => write!(f, "File({})", p.display()),
            Media::Custom(_) => write!(f, "Custom"),
        }
    }
}

impl Media {
    fn open(&self) -> Result<Box<dyn PcmSource>, AudioError> {
        match self {
            Media::File(p) => Ok(Box::new(Decoder::open_file(p)?)),
            Media::Custom(o) => o(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

/// Réglage ReplayGain.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ReplayGain {
    pub mode: ReplayGainMode,
    /// Préampli ReplayGain (dB).
    pub preamp_db: f32,
    /// Gain des fichiers sans information (dB).
    pub untagged_db: f32,
    /// Éviter l'écrêtage grâce aux crêtes connues.
    pub prevent_clipping: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReplayGainMode {
    #[default]
    Off,
    Track,
    Album,
}

impl ReplayGain {
    fn gain_db(&self, info: &TrackInfo) -> f32 {
        let (gain, peak) = match self.mode {
            ReplayGainMode::Off => return 0.0,
            ReplayGainMode::Track => (info.rg_track_gain.or(info.rg_album_gain), info.rg_track_peak.or(info.rg_album_peak)),
            ReplayGainMode::Album => (info.rg_album_gain.or(info.rg_track_gain), info.rg_album_peak.or(info.rg_track_peak)),
        };
        let Some(g) = gain else { return self.untagged_db };
        let mut db = g + self.preamp_db;
        if self.prevent_clipping {
            if let Some(p) = peak.filter(|p| *p > 0.0) {
                let max_db = -20.0 * p.log10();
                db = db.min(max_db);
            }
        }
        db
    }
}

/// Événements envoyés à l'application.
#[derive(Debug, Clone)]
pub enum Event {
    /// Le son de ce morceau commence à être entendu.
    Started { token: Token, info: TrackInfo },
    /// Nouvelles informations (titre d'un flux, débit…).
    InfoChanged { token: Token, info: TrackInfo },
    /// Fin naturelle, sans morceau suivant préparé.
    Ended { token: Token },
    /// Le morceau suivant préparé a été atteint sans blanc (gapless) ; `previous` est terminé.
    Advanced { previous: Token, token: Token, info: TrackInfo },
    Error { token: Token, message: String },
}

/// État instantané.
#[derive(Debug, Clone, Default)]
pub struct Status {
    pub state: PlayState,
    pub token: Option<Token>,
    pub position_ms: u64,
    pub info: Option<TrackInfo>,
    /// Ouverture en cours (réseau, mise en mémoire tampon).
    pub buffering: bool,
}

enum Cmd {
    Play { token: Token, media: Media, start_ms: u64, paused: bool },
    SetNext(Option<(Token, Media)>),
    Pause,
    Resume,
    Stop,
    Seek(u64),
    Eq(EqSettings),
    ReplayGain(ReplayGain),
    Shutdown,
}

struct Segment {
    start: u64,
    token: Token,
    base_ms: u64,
    info: TrackInfo,
    announced: bool,
}

#[derive(Default)]
struct Timeline {
    segments: VecDeque<Segment>,
    eos_at: Option<u64>,
    state: PlayState,
    buffering: bool,
}

/// Le moteur. Clonable à bas coût n'est pas nécessaire : l'application en garde un.
pub struct Engine {
    cmds: Sender<Cmd>,
    events: Receiver<Event>,
    shared: Arc<Shared>,
    timeline: Arc<Mutex<Timeline>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _output: Output,
}

impl Engine {
    pub fn new() -> Engine {
        let shared = Arc::new(Shared::default());
        let output = output::start(shared.clone());
        let timeline = Arc::new(Mutex::new(Timeline::default()));
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let (ev_tx, ev_rx) = crossbeam_channel::unbounded();
        let worker = Worker::new(shared.clone(), timeline.clone(), cmd_rx, ev_tx);
        let thread = std::thread::Builder::new().name("alpagamp-décodage".into()).spawn(move || worker.run()).expect("fil de décodage");
        Engine { cmds: cmd_tx, events: ev_rx, shared, timeline, thread: Some(thread), _output: output }
    }

    /// Joue `media` (remplace ce qui joue).
    pub fn play(&self, token: Token, media: Media, start_ms: u64, paused: bool) {
        let _ = self.cmds.send(Cmd::Play { token, media, start_ms, paused });
    }

    /// Prépare (ou annule) le morceau suivant pour un enchaînement sans blanc.
    pub fn set_next(&self, next: Option<(Token, Media)>) {
        let _ = self.cmds.send(Cmd::SetNext(next));
    }

    pub fn pause(&self) {
        self.shared.paused.store(true, Ordering::Relaxed);
        let _ = self.cmds.send(Cmd::Pause);
    }

    pub fn resume(&self) {
        self.shared.paused.store(false, Ordering::Relaxed);
        let _ = self.cmds.send(Cmd::Resume);
    }

    pub fn stop(&self) {
        let _ = self.cmds.send(Cmd::Stop);
    }

    pub fn seek(&self, ms: u64) {
        let _ = self.cmds.send(Cmd::Seek(ms));
    }

    pub fn set_eq(&self, eq: EqSettings) {
        let _ = self.cmds.send(Cmd::Eq(eq));
    }

    pub fn set_replay_gain(&self, rg: ReplayGain) {
        let _ = self.cmds.send(Cmd::ReplayGain(rg));
    }

    /// Volume linéaire 0..=1.
    pub fn set_volume(&self, v: f32) {
        self.shared.set_volume(v);
    }

    /// Balance −1 (gauche) ..= 1 (droite).
    pub fn set_balance(&self, b: f32) {
        self.shared.set_balance(b);
    }

    pub fn has_output_device(&self) -> bool {
        self.shared.has_device.load(Ordering::Relaxed)
    }

    pub fn output_rate(&self) -> u32 {
        self.shared.out_rate.load(Ordering::Relaxed)
    }

    /// Événements en attente.
    pub fn poll_events(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }

    pub fn events(&self) -> &Receiver<Event> {
        &self.events
    }

    /// Position et état de ce qui est entendu.
    pub fn status(&self) -> Status {
        let played = self.shared.queue.lock().played;
        let tl = self.timeline.lock();
        let rate = self.output_rate().max(1) as u64;
        let seg = tl.segments.iter().rev().find(|s| s.start <= played).or_else(|| tl.segments.front());
        match seg {
            Some(s) => {
                let mut pos = s.base_ms + played.saturating_sub(s.start) * 1000 / rate;
                if let Some(d) = s.info.duration_ms {
                    pos = pos.min(d);
                }
                Status { state: tl.state, token: Some(s.token), position_ms: pos, info: Some(s.info.clone()), buffering: tl.buffering }
            }
            None => Status { state: tl.state, buffering: tl.buffering, ..Default::default() },
        }
    }

    /// Dernières trames jouées (gauche, droite) pour les visualisations.
    pub fn vis_latest(&self, left: &mut [f32], right: &mut [f32]) {
        let state = self.timeline.lock().state;
        if state != PlayState::Playing {
            left.fill(0.0);
            right.fill(0.0);
            return;
        }
        self.shared.vis.lock().latest(left, right);
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.cmds.send(Cmd::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

// --- Fil de décodage ---------------------------------------------------------------------------

struct Current {
    token: Token,
    media: Media,
    src: Box<dyn PcmSource>,
    resampler: StereoResampler,
    finished: bool,
}

struct Opening {
    token: Token,
    media: Media,
    start_ms: u64,
    rx: Receiver<Result<Box<dyn PcmSource>, AudioError>>,
}

enum NextState {
    None,
    Opening(Token, Media, Receiver<Result<Box<dyn PcmSource>, AudioError>>, Instant),
    Ready(Token, Media, Box<dyn PcmSource>),
    Failed,
}

struct Worker {
    shared: Arc<Shared>,
    timeline: Arc<Mutex<Timeline>>,
    cmds: Receiver<Cmd>,
    events: Sender<Event>,
    current: Option<Current>,
    opening: Option<Opening>,
    next: NextState,
    /// Média du morceau suivant demandé (pour le rouvrir après un déplacement en fin de morceau).
    next_request: Option<(Token, Media)>,
    dsp: Dsp,
    rg: ReplayGain,
    decoded: Vec<f32>,
    resampled: Vec<f32>,
    /// Média de chaque morceau encore dans la file (pour un déplacement dans un morceau déjà quitté).
    media_of: Vec<(Token, Media)>,
}

fn spawn_open(media: Media) -> Receiver<Result<Box<dyn PcmSource>, AudioError>> {
    let (tx, rx) = crossbeam_channel::bounded(1);
    std::thread::Builder::new()
        .name("alpagamp-ouverture".into())
        .spawn(move || {
            let _ = tx.send(media.open());
        })
        .ok();
    rx
}

impl Worker {
    fn new(shared: Arc<Shared>, timeline: Arc<Mutex<Timeline>>, cmds: Receiver<Cmd>, events: Sender<Event>) -> Worker {
        let rate = shared.out_rate.load(Ordering::Relaxed);
        Worker {
            shared,
            timeline,
            cmds,
            events,
            current: None,
            opening: None,
            next: NextState::None,
            next_request: None,
            dsp: Dsp::new(rate),
            rg: ReplayGain::default(),
            decoded: Vec::with_capacity(16_384),
            resampled: Vec::with_capacity(16_384),
            media_of: Vec::new(),
        }
    }

    fn out_rate(&self) -> u32 {
        self.shared.out_rate.load(Ordering::Relaxed)
    }

    fn run(mut self) {
        loop {
            let busy = self.current.is_some() || self.opening.is_some();
            let timeout = if busy { Duration::from_millis(4) } else { Duration::from_millis(50) };
            match self.cmds.recv_timeout(timeout) {
                Ok(cmd) => {
                    if !self.handle(cmd) {
                        return;
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return,
            }
            loop {
                match self.cmds.try_recv() {
                    Ok(cmd) => {
                        if !self.handle(cmd) {
                            return;
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }
            self.poll_opening();
            self.poll_next();
            self.fill();
            self.monitor();
        }
    }

    /// Renvoie `false` pour arrêter le fil.
    fn handle(&mut self, cmd: Cmd) -> bool {
        match cmd {
            Cmd::Play { token, media, start_ms, paused } => {
                self.hard_stop();
                self.shared.paused.store(paused, Ordering::Relaxed);
                {
                    let mut tl = self.timeline.lock();
                    tl.state = if paused { PlayState::Paused } else { PlayState::Playing };
                    tl.buffering = true;
                }
                let rx = spawn_open(media.clone());
                self.opening = Some(Opening { token, media, start_ms, rx });
            }
            Cmd::SetNext(n) => {
                self.next_request = n.clone();
                self.next = match n {
                    Some((t, m)) => NextState::Opening(t, m.clone(), spawn_open(m), Instant::now()),
                    None => NextState::None,
                };
                // Un « suivant » arrive alors que la fin était déjà marquée : on reprend l'enchaînement.
                if self.current.as_ref().is_some_and(|c| c.finished) {
                    self.timeline.lock().eos_at = None;
                }
            }
            Cmd::Pause => {
                let mut tl = self.timeline.lock();
                if tl.state == PlayState::Playing {
                    tl.state = PlayState::Paused;
                }
            }
            Cmd::Resume => {
                let mut tl = self.timeline.lock();
                if tl.state == PlayState::Paused {
                    tl.state = PlayState::Playing;
                }
            }
            Cmd::Stop => self.hard_stop(),
            Cmd::Seek(ms) => self.seek(ms),
            Cmd::Eq(eq) => self.dsp.set_eq(eq),
            Cmd::ReplayGain(rg) => {
                self.rg = rg;
                if let Some(c) = &self.current {
                    let db = self.rg.gain_db(c.src.info());
                    self.dsp.set_replay_gain_db(db);
                }
            }
            Cmd::Shutdown => return false,
        }
        true
    }

    fn hard_stop(&mut self) {
        self.shared.queue.lock().flush();
        self.shared.vis.lock().clear();
        self.current = None;
        self.opening = None;
        self.next = NextState::None;
        self.next_request = None;
        self.media_of.clear();
        let mut tl = self.timeline.lock();
        tl.segments.clear();
        tl.eos_at = None;
        tl.state = PlayState::Stopped;
        tl.buffering = false;
    }

    fn poll_opening(&mut self) {
        let Some(op) = &self.opening else { return };
        let res = match op.rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(AudioError::Open("ouverture interrompue".into())),
        };
        let op = self.opening.take().unwrap();
        match res {
            Ok(mut src) => {
                let mut base = 0;
                if op.start_ms > 0 {
                    match src.seek(op.start_ms) {
                        Ok(p) => base = p,
                        Err(e) => log::warn!("reprise à {} ms impossible : {e}", op.start_ms),
                    }
                }
                self.begin(op.token, op.media, src, base);
                self.timeline.lock().buffering = false;
            }
            Err(e) => {
                {
                    let mut tl = self.timeline.lock();
                    tl.state = PlayState::Stopped;
                    tl.buffering = false;
                }
                let _ = self.events.send(Event::Error { token: op.token, message: e.to_string() });
            }
        }
    }

    /// Commence à décoder une source : nouveau segment à la fin de la file.
    fn begin(&mut self, token: Token, media: Media, src: Box<dyn PcmSource>, base_ms: u64) {
        let info = src.info().clone();
        let resampler = StereoResampler::new(info.sample_rate, self.out_rate());
        self.dsp.set_rate(self.out_rate());
        self.dsp.set_replay_gain_db(self.rg.gain_db(&info));
        let start = self.shared.queue.lock().pushed;
        {
            let mut tl = self.timeline.lock();
            tl.segments.push_back(Segment { start, token, base_ms, info, announced: false });
            tl.eos_at = None;
        }
        self.media_of.retain(|(t, _)| *t != token);
        self.media_of.push((token, media.clone()));
        self.current = Some(Current { token, media, src, resampler, finished: false });
    }

    fn poll_next(&mut self) {
        if let NextState::Opening(t, m, rx, since) = &self.next {
            match rx.try_recv() {
                Ok(Ok(src)) => self.next = NextState::Ready(*t, m.clone(), src),
                Ok(Err(e)) => {
                    log::warn!("préparation du morceau suivant : {e}");
                    let _ = self.events.send(Event::Error { token: *t, message: e.to_string() });
                    self.next = NextState::Failed;
                }
                Err(TryRecvError::Disconnected) => self.next = NextState::Failed,
                Err(TryRecvError::Empty) => {
                    if since.elapsed() > Duration::from_secs(20) {
                        self.next = NextState::Failed;
                    }
                }
            }
        }
    }

    fn fill(&mut self) {
        let target = (self.out_rate() as usize) / 5; // 200 ms d'avance
        let mut guard = 0;
        while guard < 64 {
            guard += 1;
            let Some(cur) = self.current.as_mut() else { return };
            if self.shared.queue.lock().queued_frames() >= target {
                return;
            }
            if cur.finished {
                // Enchaîner sur le suivant s'il est prêt.
                match std::mem::replace(&mut self.next, NextState::None) {
                    NextState::Ready(t, m, src) => {
                        self.next_request = None;
                        self.begin(t, m, src, 0);
                        continue;
                    }
                    NextState::Opening(t, m, rx, since) => {
                        // On attend (le tampon s'épuise peut-être : petit blanc plutôt qu'un arrêt).
                        self.next = NextState::Opening(t, m, rx, since);
                        return;
                    }
                    NextState::None | NextState::Failed => {
                        let mut tl = self.timeline.lock();
                        if tl.eos_at.is_none() {
                            tl.eos_at = Some(self.shared.queue.lock().pushed);
                        }
                        return;
                    }
                }
            }
            self.decoded.clear();
            let n = match cur.src.read(&mut self.decoded) {
                Ok(n) => n,
                Err(e) => {
                    log::warn!("décodage : {e}");
                    let _ = self.events.send(Event::Error { token: cur.token, message: e.to_string() });
                    0
                }
            };
            if let Some(info) = cur.src.take_info_update() {
                let token = cur.token;
                {
                    let mut tl = self.timeline.lock();
                    if let Some(s) = tl.segments.iter_mut().rev().find(|s| s.token == token) {
                        s.info = info.clone();
                    }
                }
                let _ = self.events.send(Event::InfoChanged { token, info });
            }
            self.resampled.clear();
            if n == 0 {
                cur.resampler.flush(&mut self.resampled);
                cur.finished = true;
            } else {
                cur.resampler.push(&self.decoded, &mut self.resampled);
            }
            if !self.resampled.is_empty() {
                self.dsp.process(&mut self.resampled);
                let mut q = self.shared.queue.lock();
                q.samples.extend(self.resampled.iter().copied());
                q.pushed += (self.resampled.len() / 2) as u64;
            }
        }
    }

    /// Annonce les morceaux qui commencent à être entendus, et la fin.
    fn monitor(&mut self) {
        let played = self.shared.queue.lock().played;
        let mut events = Vec::new();
        {
            let mut tl = self.timeline.lock();
            // Segments atteints.
            let mut prev_token = None;
            for i in 0..tl.segments.len() {
                let (start, announced) = (tl.segments[i].start, tl.segments[i].announced);
                if start <= played && !announced {
                    tl.segments[i].announced = true;
                    let s = &tl.segments[i];
                    match prev_token.or_else(|| if i > 0 { Some(tl.segments[i - 1].token) } else { None }) {
                        Some(p) if p != s.token => events.push(Event::Advanced { previous: p, token: s.token, info: s.info.clone() }),
                        _ => events.push(Event::Started { token: s.token, info: s.info.clone() }),
                    }
                }
                prev_token = Some(tl.segments[i].token);
            }
            // Oublie les segments entièrement joués (on garde le dernier atteint).
            while tl.segments.len() > 1 && tl.segments[1].start <= played {
                let gone = tl.segments.pop_front().unwrap();
                self.media_of.retain(|(t, _)| *t != gone.token || tl.segments.iter().any(|s| s.token == gone.token));
            }
            if let Some(eos) = tl.eos_at {
                if played >= eos && tl.state != PlayState::Stopped {
                    let token = tl.segments.back().map(|s| s.token);
                    tl.state = PlayState::Stopped;
                    tl.eos_at = None;
                    tl.segments.clear();
                    if let Some(t) = token {
                        events.push(Event::Ended { token: t });
                    }
                    self.current = None;
                    self.media_of.clear();
                }
            }
        }
        for e in events {
            let _ = self.events.send(e);
        }
    }

    fn seek(&mut self, ms: u64) {
        let played = self.shared.queue.lock().played;
        let heard = {
            let tl = self.timeline.lock();
            tl.segments.iter().rev().find(|s| s.start <= played).or_else(|| tl.segments.front()).map(|s| s.token)
        };
        let Some(heard) = heard else { return };
        let decoding_same = self.current.as_ref().is_some_and(|c| c.token == heard);
        self.shared.queue.lock().flush();
        self.dsp.reset_state();
        if decoding_same {
            let cur = self.current.as_mut().unwrap();
            match cur.src.seek(ms) {
                Ok(actual) => {
                    cur.resampler.reset();
                    cur.finished = false;
                    let start = self.shared.queue.lock().pushed;
                    let mut tl = self.timeline.lock();
                    tl.eos_at = None;
                    let info = tl.segments.iter().find(|s| s.token == heard).map(|s| s.info.clone()).unwrap_or_default();
                    tl.segments.clear();
                    tl.segments.push_back(Segment { start, token: heard, base_ms: actual, info, announced: true });
                }
                Err(e) => log::warn!("déplacement : {e}"),
            }
        } else {
            // Le morceau entendu n'est plus celui qu'on décode (enchaînement déjà commencé) : on le rouvre.
            let Some(media) = self.media_of.iter().find(|(t, _)| *t == heard).map(|(_, m)| m.clone()) else { return };
            let next = self.next_request.clone().or_else(|| self.current.as_ref().map(|c| (c.token, c.media.clone())));
            let state = self.timeline.lock().state;
            self.hard_stop();
            self.timeline.lock().state = state;
            match media.open() {
                Ok(mut src) => {
                    let base = src.seek(ms).unwrap_or(0);
                    self.begin(heard, media, src, base);
                    if let Some(seg) = self.timeline.lock().segments.back_mut() {
                        seg.announced = true;
                    }
                    if let Some((t, m)) = next {
                        self.next_request = Some((t, m.clone()));
                        self.next = NextState::Opening(t, m.clone(), spawn_open(m), Instant::now());
                    }
                }
                Err(e) => {
                    let _ = self.events.send(Event::Error { token: heard, message: e.to_string() });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::tests::sine_wav;

    fn wait_for(engine: &Engine, timeout_ms: u64, mut pred: impl FnMut(&Event) -> bool) -> Option<Event> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        while Instant::now() < deadline {
            if let Ok(e) = engine.events().recv_timeout(Duration::from_millis(20)) {
                if pred(&e) {
                    return Some(e);
                }
            }
        }
        None
    }

    #[test]
    fn plays_seeks_and_chains_gaplessly() {
        let dir = std::env::temp_dir().join(format!("alpagamp-eng-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.wav");
        let b = dir.join("b.wav");
        sine_wav(&a, 44_100, 1.0, 440.0);
        sine_wav(&b, 22_050, 0.6, 880.0);
        let engine = Engine::new();
        engine.play(1, Media::File(a.clone()), 0, false);
        let started = wait_for(&engine, 3000, |e| matches!(e, Event::Started { token: 1, .. }));
        assert!(started.is_some(), "pas de démarrage");
        engine.set_next(Some((2, Media::File(b.clone()))));
        std::thread::sleep(Duration::from_millis(150));
        let st = engine.status();
        assert_eq!(st.state, PlayState::Playing);
        assert!(st.position_ms > 0 && st.position_ms < 1000, "position {}", st.position_ms);
        let adv = wait_for(&engine, 3000, |e| matches!(e, Event::Advanced { previous: 1, token: 2, .. }));
        assert!(adv.is_some(), "pas d'enchaînement");
        let end = wait_for(&engine, 3000, |e| matches!(e, Event::Ended { token: 2 }));
        assert!(end.is_some(), "pas de fin");
        assert_eq!(engine.status().state, PlayState::Stopped);
        // Déplacement.
        engine.play(3, Media::File(a.clone()), 0, false);
        wait_for(&engine, 3000, |e| matches!(e, Event::Started { token: 3, .. }));
        engine.seek(700);
        std::thread::sleep(Duration::from_millis(60));
        let pos = engine.status().position_ms;
        assert!((700..900).contains(&pos), "après déplacement : {pos}");
        engine.stop();
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(engine.status().state, PlayState::Stopped);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn error_on_missing_file() {
        let engine = Engine::new();
        engine.play(9, Media::File(PathBuf::from("/nonexistent/x.mp3")), 0, false);
        let e = wait_for(&engine, 2000, |e| matches!(e, Event::Error { token: 9, .. }));
        assert!(e.is_some());
    }
}
