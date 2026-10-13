//! Le son donné aux visualisations : spectre (FFT de 1152 trames, fenêtre de Hann) et forme d'onde.

use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

use crate::host::Sound;

const N: usize = 1152;

pub struct Analyzer {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buf: Vec<Complex<f32>>,
    left: Vec<f32>,
    right: Vec<f32>,
    pub sound: Sound,
}

impl Default for Analyzer {
    fn default() -> Self {
        let window = (0..N).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (N - 1) as f32).cos()).collect();
        Analyzer {
            fft: FftPlanner::new().plan_fft_forward(N),
            window,
            buf: vec![Complex::default(); N],
            left: vec![0.0; N],
            right: vec![0.0; N],
            sound: Sound::default(),
        }
    }
}

impl Analyzer {
    /// Analyse les dernières trames jouées (`fill` remplit gauche et droite, 1152 trames chacune).
    pub fn update(&mut self, fill: impl FnOnce(&mut [f32], &mut [f32])) {
        fill(&mut self.left, &mut self.right);
        // Amplitude d'un sinus pleine échelle ≈ 1 (fenêtre de Hann : gain cohérent 0,5).
        let norm = 4.0 / N as f32;
        for (ch, src) in [&self.left, &self.right].into_iter().enumerate() {
            for (i, c) in self.buf.iter_mut().enumerate() {
                *c = Complex::new(src[i] * self.window[i], 0.0);
            }
            self.fft.process(&mut self.buf);
            for k in 0..576 {
                self.sound.spectrum[ch * 576 + k] = (self.buf[k].norm() * norm).min(1.0);
            }
            self.sound.wave[ch * 576..(ch + 1) * 576].copy_from_slice(&src[N - 576..]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sine_shows_in_its_band() {
        let mut a = Analyzer::default();
        // Bande 26 exactement : 26 × 44100 / 1152 Hz.
        let hz = 26.0 * 44_100.0 / 1152.0;
        a.update(|l, r| {
            for i in 0..N {
                let s = (std::f32::consts::TAU * hz * i as f32 / 44_100.0).sin();
                l[i] = s;
                r[i] = s * 0.5;
            }
        });
        let s = &a.sound.spectrum;
        assert!((s[26] - 1.0).abs() < 0.05, "{}", s[26]);
        assert!((s[576 + 26] - 0.5).abs() < 0.05);
        assert!(s[100] < 0.01);
        assert!(a.sound.wave.iter().any(|&w| w > 0.9));
    }
}
