//! Sortie audio : file d'échantillons partagée, rappel cpal (ou sortie muette sans périphérique),
//! volume, balance, prélèvement pour la visualisation.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use parking_lot::Mutex;

/// File d'échantillons stéréo entrelacés entre le fil de décodage et le rappel audio.
#[derive(Default)]
pub struct Queue {
    pub samples: VecDeque<f32>,
    /// Trames poussées depuis le démarrage du moteur.
    pub pushed: u64,
    /// Trames effectivement jouées (ou jetées lors d'un vidage).
    pub played: u64,
}

impl Queue {
    pub fn queued_frames(&self) -> usize {
        self.samples.len() / 2
    }

    /// Vide la file : ce qui était en attente est considéré comme joué.
    pub fn flush(&mut self) {
        let n = self.samples.len() / 2;
        self.samples.clear();
        self.played += n as u64;
    }
}

/// Anneau des derniers échantillons joués (avant volume), pour les visualisations.
pub struct VisRing {
    buf: Vec<f32>,
    write: usize,
    /// Nombre total de trames écrites.
    pub total: u64,
}

pub const VIS_RING_FRAMES: usize = 4096;

impl Default for VisRing {
    fn default() -> Self {
        Self { buf: vec![0.0; VIS_RING_FRAMES * 2], write: 0, total: 0 }
    }
}

impl VisRing {
    fn push(&mut self, samples: &[f32]) {
        for f in samples.chunks_exact(2) {
            self.buf[self.write * 2] = f[0];
            self.buf[self.write * 2 + 1] = f[1];
            self.write = (self.write + 1) % VIS_RING_FRAMES;
        }
        self.total += (samples.len() / 2) as u64;
    }

    /// Copie les `left.len()` dernières trames (gauche et droite séparées).
    pub fn latest(&self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len()).min(VIS_RING_FRAMES);
        for i in 0..n {
            let idx = (self.write + VIS_RING_FRAMES - n + i) % VIS_RING_FRAMES;
            left[i] = self.buf[idx * 2];
            right[i] = self.buf[idx * 2 + 1];
        }
    }

    pub fn clear(&mut self) {
        self.buf.fill(0.0);
    }
}

/// État partagé entre le moteur et la sortie.
pub struct Shared {
    pub queue: Mutex<Queue>,
    pub vis: Mutex<VisRing>,
    volume: AtomicU32,
    balance: AtomicU32,
    pub paused: AtomicBool,
    pub out_rate: AtomicU32,
    pub has_device: AtomicBool,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            queue: Mutex::new(Queue::default()),
            vis: Mutex::new(VisRing::default()),
            volume: AtomicU32::new(1.0f32.to_bits()),
            balance: AtomicU32::new(0.0f32.to_bits()),
            paused: AtomicBool::new(false),
            out_rate: AtomicU32::new(44_100),
            has_device: AtomicBool::new(false),
        }
    }
}

impl Shared {
    pub fn set_volume(&self, v: f32) {
        self.volume.store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }
    pub fn set_balance(&self, b: f32) {
        self.balance.store(b.clamp(-1.0, 1.0).to_bits(), Ordering::Relaxed);
    }
    pub fn balance(&self) -> f32 {
        f32::from_bits(self.balance.load(Ordering::Relaxed))
    }

    /// Prend jusqu'à `frames` trames dans la file (rien en pause) et les recopie pour la visualisation.
    fn take(&self, frames: usize, scratch: &mut Vec<f32>) {
        scratch.clear();
        if self.paused.load(Ordering::Relaxed) {
            return;
        }
        {
            let mut q = self.queue.lock();
            let n = q.queued_frames().min(frames);
            scratch.extend(q.samples.drain(..n * 2));
            q.played += n as u64;
        }
        if !scratch.is_empty() {
            self.vis.lock().push(scratch);
        }
    }

    /// Gains gauche/droite (volume × balance, comme la sortie « linéaire » de Winamp).
    fn gains(&self) -> (f32, f32) {
        let v = self.volume();
        let b = self.balance();
        let gl = if b > 0.0 { 1.0 - b } else { 1.0 };
        let gr = if b < 0.0 { 1.0 + b } else { 1.0 };
        (v * gl, v * gr)
    }
}

/// Poignée de la sortie : la garder vivante garde le son.
pub struct Output {
    stop: crossbeam_channel::Sender<()>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Output {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Démarre la sortie sur le périphérique par défaut ; sans périphérique, une sortie muette consomme
/// la file en temps réel (la position avance, les visualisations bougent).
pub fn start(shared: Arc<Shared>) -> Output {
    let (stop_tx, stop_rx) = crossbeam_channel::bounded::<()>(1);
    let (ready_tx, ready_rx) = crossbeam_channel::bounded::<bool>(1);
    let sh = shared.clone();
    let thread = std::thread::Builder::new()
        .name("alpagamp-sortie".into())
        .spawn(move || {
            match open_device(sh.clone()) {
                Ok(stream) => {
                    sh.has_device.store(true, Ordering::Relaxed);
                    let _ = ready_tx.send(true);
                    let _ = stop_rx.recv();
                    drop(stream);
                }
                Err(e) => {
                    log::warn!("pas de sortie audio ({e}) : sortie muette");
                    sh.has_device.store(false, Ordering::Relaxed);
                    sh.out_rate.store(44_100, Ordering::Relaxed);
                    let _ = ready_tx.send(false);
                    null_output(sh, stop_rx);
                }
            }
        })
        .expect("fil de sortie");
    let _ = ready_rx.recv_timeout(Duration::from_secs(5));
    Output { stop: stop_tx, thread: Some(thread) }
}

fn open_device(shared: Arc<Shared>) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("aucun périphérique de sortie")?;
    let config = device.default_output_config().map_err(|e| e.to_string())?;
    let rate = config.sample_rate();
    let fmt = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();
    shared.out_rate.store(rate, Ordering::Relaxed);
    let stream = match fmt {
        cpal::SampleFormat::F32 => build::<f32>(&device, stream_config, shared),
        cpal::SampleFormat::I16 => build::<i16>(&device, stream_config, shared),
        cpal::SampleFormat::I32 => build::<i32>(&device, stream_config, shared),
        cpal::SampleFormat::U16 => build::<u16>(&device, stream_config, shared),
        cpal::SampleFormat::F64 => build::<f64>(&device, stream_config, shared),
        other => return Err(format!("format d'échantillon non géré : {other}")),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

fn build<T>(device: &cpal::Device, config: cpal::StreamConfig, shared: Arc<Shared>) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let ch = config.channels as usize;
    let mut scratch: Vec<f32> = Vec::with_capacity(16_384);
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let frames = data.len() / ch.max(1);
                shared.take(frames, &mut scratch);
                let (gl, gr) = shared.gains();
                for (i, frame) in data.chunks_mut(ch.max(1)).enumerate() {
                    let (l, r) = if i * 2 + 1 < scratch.len() { (scratch[i * 2] * gl, scratch[i * 2 + 1] * gr) } else { (0.0, 0.0) };
                    match frame.len() {
                        1 => frame[0] = T::from_sample((l + r) * 0.5),
                        _ => {
                            frame[0] = T::from_sample(l);
                            frame[1] = T::from_sample(r);
                            for s in &mut frame[2..] {
                                *s = T::from_sample(0.0f32);
                            }
                        }
                    }
                }
            },
            |e| log::warn!("erreur de sortie audio : {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}

fn null_output(shared: Arc<Shared>, stop: crossbeam_channel::Receiver<()>) {
    let rate = shared.out_rate.load(Ordering::Relaxed) as f64;
    let start = Instant::now();
    let mut consumed: u64 = 0;
    let mut scratch = Vec::new();
    loop {
        if stop.recv_timeout(Duration::from_millis(10)).is_ok() {
            return;
        }
        let due = (start.elapsed().as_secs_f64() * rate) as u64;
        let frames = (due - consumed) as usize;
        consumed = due;
        shared.take(frames, &mut scratch);
    }
}
