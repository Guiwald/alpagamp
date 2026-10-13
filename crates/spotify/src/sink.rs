//! Le pont entre le lecteur de librespot (qui pousse du PCM) et le moteur d'Alpagamp (qui en tire) : un tampon
//! partagé, une « sortie » librespot qui le remplit, une source qui le vide.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use alpagamp_audio::{AudioError, PcmSource, TrackInfo};
use librespot_playback::audio_backend::{Sink, SinkResult};
use librespot_playback::convert::Converter;
use librespot_playback::decoder::AudioPacket;
use parking_lot::{Condvar, Mutex};

/// Fréquence et canaux de librespot.
pub const RATE: u32 = 44_100;
/// Au-delà (≈ 2 s), la sortie attend que la source consomme.
const MAX_SAMPLES: usize = RATE as usize * 2 * 2;
/// En dessous (≈ 0,2 s), la source se déclare « pas prête » (sauf fin de morceau).
const LOW_WATER: usize = RATE as usize * 2 / 5;

#[derive(Default)]
struct State {
    samples: VecDeque<f32>,
    /// Fin du morceau : plus rien n'arrivera.
    ended: bool,
    /// La source a été abandonnée : la sortie ne doit plus attendre.
    closed: bool,
    info: Option<TrackInfo>,
    info_changed: bool,
    /// Nouvelle position (ms) à annoncer au moteur (changement de morceau en Connect).
    position_reset: Option<u64>,
    error: Option<String>,
}

/// Tampon partagé.
#[derive(Default)]
pub struct Shared {
    st: Mutex<State>,
    cond: Condvar,
}

impl Shared {
    pub fn new() -> Arc<Shared> {
        Arc::new(Shared::default())
    }

    pub fn set_info(&self, info: TrackInfo) {
        let mut st = self.st.lock();
        st.info = Some(info);
        st.info_changed = true;
        self.cond.notify_all();
    }

    pub fn info(&self) -> Option<TrackInfo> {
        self.st.lock().info.clone()
    }

    pub fn set_ended(&self) {
        self.st.lock().ended = true;
        self.cond.notify_all();
    }

    pub fn set_error(&self, e: String) {
        let mut st = self.st.lock();
        st.error = Some(e);
        st.ended = true;
        self.cond.notify_all();
    }

    /// Vide le tampon (déplacement, changement de morceau) et repart.
    pub fn flush(&self, new_position: Option<u64>) {
        let mut st = self.st.lock();
        st.samples.clear();
        st.ended = false;
        if new_position.is_some() {
            st.position_reset = new_position;
        }
        self.cond.notify_all();
    }

    pub fn close(&self) {
        let mut st = self.st.lock();
        st.closed = true;
        st.samples.clear();
        self.cond.notify_all();
    }

    pub fn reopen(&self) {
        let mut st = self.st.lock();
        st.closed = false;
        st.ended = false;
        st.samples.clear();
    }

    /// Attend des données ou la fin (pour l'ouverture d'une source).
    pub fn wait_ready(&self, timeout: Duration) -> Result<(), String> {
        let deadline = std::time::Instant::now() + timeout;
        let mut st = self.st.lock();
        loop {
            if let Some(e) = &st.error {
                return Err(e.clone());
            }
            if st.samples.len() >= LOW_WATER || st.ended {
                return Ok(());
            }
            if self.cond.wait_until(&mut st, deadline).timed_out() {
                return Err("Spotify ne répond pas".into());
            }
        }
    }

    fn push(&self, data: &[f32]) {
        let mut st = self.st.lock();
        while st.samples.len() >= MAX_SAMPLES && !st.closed {
            self.cond.wait_for(&mut st, Duration::from_millis(100));
        }
        if !st.closed {
            st.samples.extend(data.iter().copied());
        }
        self.cond.notify_all();
    }
}

/// « Sortie audio » de librespot qui écrit dans le tampon.
pub struct BufferSink(pub Arc<Shared>);

impl Sink for BufferSink {
    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        if let AudioPacket::Samples(s) = packet {
            let f = converter.f64_to_f32(&s);
            self.0.push(&f);
        }
        Ok(())
    }
}

/// Source pour le moteur : lit le tampon.
pub struct BufferSource {
    shared: Arc<Shared>,
    info: TrackInfo,
    seek: Box<dyn FnMut(u64) + Send>,
    on_drop: Option<Box<dyn FnOnce() + Send>>,
}

impl BufferSource {
    pub fn new(shared: Arc<Shared>, info: TrackInfo, seek: Box<dyn FnMut(u64) + Send>, on_drop: Box<dyn FnOnce() + Send>) -> BufferSource {
        BufferSource { shared, info, seek, on_drop: Some(on_drop) }
    }
}

impl Drop for BufferSource {
    fn drop(&mut self) {
        self.shared.close();
        if let Some(f) = self.on_drop.take() {
            f();
        }
    }
}

impl PcmSource for BufferSource {
    fn info(&self) -> &TrackInfo {
        &self.info
    }

    fn read(&mut self, out: &mut Vec<f32>) -> Result<usize, AudioError> {
        let mut st = self.shared.st.lock();
        if st.samples.is_empty() {
            if let Some(e) = st.error.take() {
                return Err(AudioError::Decode(e));
            }
            return Ok(0); // fin (la source n'est lue que « prête »)
        }
        let n = st.samples.len().min(8192) & !1;
        out.extend(st.samples.drain(..n));
        self.shared.cond.notify_all();
        Ok(n / 2)
    }

    fn seek(&mut self, ms: u64) -> Result<u64, AudioError> {
        self.shared.flush(None);
        (self.seek)(ms);
        // Laisse au lecteur le temps de reprendre à la nouvelle position.
        let _ = self.shared.wait_ready(Duration::from_secs(10));
        Ok(ms)
    }

    fn take_info_update(&mut self) -> Option<TrackInfo> {
        let mut st = self.shared.st.lock();
        if st.info_changed {
            st.info_changed = false;
            if let Some(i) = &st.info {
                self.info = i.clone();
                return Some(i.clone());
            }
        }
        None
    }

    fn take_position_reset(&mut self) -> Option<u64> {
        self.shared.st.lock().position_reset.take()
    }

    fn ready(&self) -> bool {
        let st = self.shared.st.lock();
        st.ended || st.error.is_some() || st.samples.len() >= LOW_WATER.min(4096)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_read_and_end() {
        let sh = Shared::new();
        let mut src = BufferSource::new(sh.clone(), TrackInfo::default(), Box::new(|_| {}), Box::new(|| {}));
        assert!(!src.ready());
        let mut sink = BufferSink(sh.clone());
        let mut conv = Converter::new(None);
        sink.write(AudioPacket::Samples(vec![0.5; 10_000]), &mut conv).unwrap();
        assert!(src.ready());
        let mut out = Vec::new();
        assert_eq!(src.read(&mut out).unwrap(), 4096);
        sh.set_ended();
        let mut total = 4096;
        loop {
            let n = src.read(&mut out).unwrap();
            if n == 0 {
                break;
            }
            total += n;
        }
        assert_eq!(total, 5000);
        sh.set_info(TrackInfo { title: Some("T".into()), ..Default::default() });
        assert_eq!(src.take_info_update().and_then(|i| i.title).as_deref(), Some("T"));
    }
}
