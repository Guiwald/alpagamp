//! Rééchantillonnage stéréo vers la fréquence du périphérique de sortie (rubato, FFT synchrone).

use audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Indexing, Resampler};

const CHUNK: usize = 1024;

pub struct StereoResampler {
    inner: Option<Fft<f32>>,
    pub in_rate: u32,
    pub out_rate: u32,
    pending: Vec<f32>,
    out_buf: Vec<f32>,
    /// Trames de retard du filtre restant à jeter en début de flux.
    skip: usize,
    total_in: u64,
    total_out: u64,
}

impl StereoResampler {
    pub fn new(in_rate: u32, out_rate: u32) -> Self {
        let inner = if in_rate == out_rate || in_rate == 0 {
            None
        } else {
            match Fft::<f32>::new(in_rate as usize, out_rate as usize, CHUNK, 2, FixedSync::Input) {
                Ok(r) => Some(r),
                Err(e) => {
                    log::warn!("rééchantillonneur {in_rate}→{out_rate} impossible : {e}");
                    None
                }
            }
        };
        let skip = inner.as_ref().map(|r| r.output_delay()).unwrap_or(0);
        let out_max = inner.as_ref().map(|r| r.output_frames_max()).unwrap_or(0);
        Self { inner, in_rate, out_rate, pending: Vec::new(), out_buf: vec![0.0; out_max * 2], skip, total_in: 0, total_out: 0 }
    }

    pub fn is_passthrough(&self) -> bool {
        self.inner.is_none()
    }

    /// Ajoute des échantillons stéréo entrelacés et pousse dans `out` ce qui est prêt.
    pub fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        let Some(r) = self.inner.as_mut() else {
            out.extend_from_slice(input);
            return;
        };
        self.total_in += (input.len() / 2) as u64;
        self.pending.extend_from_slice(input);
        loop {
            let need = r.input_frames_next();
            if self.pending.len() / 2 < need {
                break;
            }
            let (used, produced) = {
                let inp = InterleavedSlice::new(&self.pending[..need * 2], 2, need).expect("taille");
                let cap = self.out_buf.len() / 2;
                let mut outp = InterleavedSlice::new_mut(&mut self.out_buf[..], 2, cap).expect("taille");
                match r.process_into_buffer(&inp, &mut outp, None) {
                    Ok(x) => x,
                    Err(e) => {
                        log::warn!("rééchantillonnage : {e}");
                        (need, 0)
                    }
                }
            };
            self.pending.drain(..used * 2);
            Self::emit(&self.out_buf[..produced * 2], &mut self.skip, &mut self.total_out, out);
        }
    }

    fn emit(samples: &[f32], skip: &mut usize, total_out: &mut u64, out: &mut Vec<f32>) {
        let frames = samples.len() / 2;
        let s = (*skip).min(frames);
        *skip -= s;
        out.extend_from_slice(&samples[s * 2..]);
        *total_out += (frames - s) as u64;
    }

    /// Vide le filtre en fin de morceau : produit exactement la durée attendue.
    pub fn flush(&mut self, out: &mut Vec<f32>) {
        let Some(r) = self.inner.as_mut() else { return };
        let expected = (self.total_in as f64 * self.out_rate as f64 / self.in_rate as f64).round() as u64;
        let mut guard = 0;
        while self.total_out < expected && guard < 16 {
            guard += 1;
            let need = r.input_frames_next();
            let have = self.pending.len() / 2;
            let partial = have.min(need);
            let mut chunk = vec![0.0f32; need * 2];
            chunk[..partial * 2].copy_from_slice(&self.pending[..partial * 2]);
            self.pending.drain(..partial * 2);
            let indexing = Indexing { input_offset: 0, output_offset: 0, partial_len: Some(partial), active_channels_mask: None };
            let produced = {
                let inp = InterleavedSlice::new(&chunk[..], 2, need).expect("taille");
                let cap = self.out_buf.len() / 2;
                let mut outp = InterleavedSlice::new_mut(&mut self.out_buf[..], 2, cap).expect("taille");
                match r.process_into_buffer(&inp, &mut outp, Some(&indexing)) {
                    Ok((_, p)) => p,
                    Err(_) => break,
                }
            };
            let remaining = (expected - self.total_out) as usize;
            let take = produced.min(remaining + self.skip);
            Self::emit(&self.out_buf[..take * 2], &mut self.skip, &mut self.total_out, out);
        }
        self.reset();
    }

    pub fn reset(&mut self) {
        if let Some(r) = self.inner.as_mut() {
            r.reset();
            self.skip = r.output_delay();
        }
        self.pending.clear();
        self.total_in = 0;
        self.total_out = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_and_frequency_preserved() {
        let (fin, fout) = (44_100u32, 48_000u32);
        let mut r = StereoResampler::new(fin, fout);
        let n = 44_100;
        let input: Vec<f32> =
            (0..n).flat_map(|i| {
                let s = (i as f32 / fin as f32 * 1000.0 * std::f32::consts::TAU).sin() * 0.5;
                [s, s]
            }).collect();
        let mut out = Vec::new();
        for c in input.chunks(2 * 777) {
            r.push(c, &mut out);
        }
        r.flush(&mut out);
        assert_eq!(out.len() / 2, 48_000);
        // Fréquence : compter les passages par zéro sur la partie stable.
        let mid: Vec<f32> = out.iter().step_by(2).skip(4800).take(38_400).copied().collect();
        let zc = mid.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        assert!((zc as i32 - 800).abs() <= 2, "{zc} passages pour 0,8 s à 1 kHz");
    }

    #[test]
    fn passthrough() {
        let mut r = StereoResampler::new(48_000, 48_000);
        let mut out = Vec::new();
        r.push(&[1.0, 2.0], &mut out);
        assert_eq!(out, vec![1.0, 2.0]);
    }
}
