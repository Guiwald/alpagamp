//! Le visualiseur intégré de la fenêtre principale : analyseur de spectre et oscilloscope.
//!
//! Comportement repris de Winamp (voir docs/reference/winamp.md §6) : 75 colonnes, barres épaisses
//! (19 × 3 px) ou fines, chute réglable, crêtes qui retombent en accélérant, trois styles de couleur.

use std::sync::Arc;

use alpagamp_core::settings::{AnalyzerStyle, ScopeStyle, VisMode, VisSettings};
use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

use crate::paint::Painter;

pub const WINDOW: usize = 576;
const FFT_N: usize = 512;
const COLS: usize = 75;
/// Pas de simulation (s) : Winamp tournait à ~60 mises à jour par seconde.
const STEP: f32 = 1.0 / 60.0;

const FALLOFF: [f32; 5] = [3.0 / 16.0, 6.0 / 16.0, 12.0 / 16.0, 1.0, 2.0];
const PEAK_FALLOFF: [f32; 5] = [1.05, 1.1, 1.2, 1.4, 1.6];

pub struct ClassicVis {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buf: Vec<Complex<f32>>,
    bin_edges: Vec<f32>,
    target: [f32; COLS],
    bars: [f32; COLS],
    peaks: [f32; COLS],
    peak_speed: [f32; COLS],
    scope: [i32; COLS],
    shown_bars: [f32; COLS],
    shown_peaks: [f32; COLS],
    shown_scope: [i32; COLS],
    acc: f32,
    updates: u32,
}

impl Default for ClassicVis {
    fn default() -> Self {
        Self::new()
    }
}

impl ClassicVis {
    pub fn new() -> Self {
        let fft = FftPlanner::new().plan_fft_forward(FFT_N);
        let window = (0..FFT_N).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (FFT_N - 1) as f32).cos()).collect();
        // Bornes des 75 bandes sur une échelle exponentielle (≈ demi-tons), en index de bin (1..256).
        let k = 2f32.powf(COLS as f32 / 12.0) - 1.0;
        let bin_edges = (0..=COLS).map(|x| (2f32.powf(x as f32 / 12.0) - 1.0) * 255.0 / k + 1.0).collect();
        Self {
            fft,
            window,
            buf: vec![Complex::default(); FFT_N],
            bin_edges,
            target: [0.0; COLS],
            bars: [0.0; COLS],
            peaks: [0.0; COLS],
            peak_speed: [0.0; COLS],
            scope: [8; COLS],
            shown_bars: [0.0; COLS],
            shown_peaks: [0.0; COLS],
            shown_scope: [8; COLS],
            acc: 0.0,
            updates: 0,
        }
    }

    /// Analyse les 576 dernières trames (gauche, droite).
    pub fn analyze(&mut self, left: &[f32], right: &[f32]) {
        let n = left.len().min(right.len());
        if n == 0 {
            self.target = [0.0; COLS];
            self.scope = [8; COLS];
            return;
        }
        // Spectre sur les 512 dernières trames, en mono.
        let start = n.saturating_sub(FFT_N);
        for i in 0..FFT_N {
            let j = start + i;
            let s = if j < n { (left[j] + right[j]) * 0.5 } else { 0.0 };
            self.buf[i] = Complex::new(s * self.window[i], 0.0);
        }
        self.fft.process(&mut self.buf);
        let norm = 4.0 / FFT_N as f32; // amplitude d'un sinus pleine échelle ≈ 1 (fenêtre de Hann)
        let mags: Vec<f32> = self.buf[..FFT_N / 2].iter().map(|c| c.norm() * norm).collect();
        for x in 0..COLS {
            let lo = self.bin_edges[x];
            let hi = self.bin_edges[x + 1].max(lo + 1.0);
            let mut m = 0f32;
            let mut b = lo.floor() as usize;
            while (b as f32) < hi && b < mags.len() {
                m = m.max(mags[b]);
                b += 1;
            }
            // Interpolation pour les bandes plus étroites qu'un bin.
            if hi - lo < 1.0 || m == 0.0 {
                let i = (lo as usize).min(mags.len() - 2);
                let t = lo.fract();
                m = m.max(mags[i] * (1.0 - t) + mags[i + 1] * t);
            }
            // Légère pente (+3 dB/octave) : les aigus ont moins d'énergie.
            let tilt = 1.0 + x as f32 / COLS as f32 * 2.0;
            let db = 20.0 * (m * tilt + 1e-9).log10();
            self.target[x] = ((db + 60.0) / 60.0 * 15.0).clamp(0.0, 15.0);
        }
        // Oscilloscope : 75 échantillons pris dans la fenêtre de 576.
        let wstart = n.saturating_sub(WINDOW);
        let wlen = n - wstart;
        for i in 0..COLS {
            let j = wstart + i * wlen / COLS;
            let s = (left[j] + right[j]) * 0.5;
            self.scope[i] = ((s * 16.0).floor() as i32 + 8).clamp(0, 15);
        }
    }

    /// Fait avancer chutes et crêtes de `dt` secondes ; met à jour l'image affichée selon le
    /// rafraîchissement choisi.
    pub fn advance(&mut self, dt: f32, settings: &VisSettings) {
        self.acc = (self.acc + dt).min(0.25);
        let fall = FALLOFF[settings.falloff.min(4) as usize];
        let pfall = PEAK_FALLOFF[settings.peak_falloff.min(4) as usize];
        let every = [1u32, 2, 4, 8][settings.refresh.min(3) as usize];
        while self.acc >= STEP {
            self.acc -= STEP;
            for x in 0..COLS {
                let t = self.target[x];
                if t > self.bars[x] {
                    self.bars[x] = t;
                } else {
                    self.bars[x] = (self.bars[x] - fall).max(t).max(0.0);
                }
                if self.bars[x] >= self.peaks[x] {
                    self.peaks[x] = self.bars[x];
                    self.peak_speed[x] = 3.0 / 256.0;
                } else {
                    self.peaks[x] = (self.peaks[x] - self.peak_speed[x]).max(0.0);
                    self.peak_speed[x] *= pfall;
                }
            }
            self.updates = self.updates.wrapping_add(1);
            if self.updates % every == 0 {
                self.shown_bars = self.bars;
                self.shown_peaks = self.peaks;
                self.shown_scope = self.scope;
            }
        }
    }

    /// Remet à zéro (arrêt).
    pub fn reset(&mut self) {
        self.target = [0.0; COLS];
        self.bars = [0.0; COLS];
        self.peaks = [0.0; COLS];
        self.shown_bars = [0.0; COLS];
        self.shown_peaks = [0.0; COLS];
        self.scope = [8; COLS];
        self.shown_scope = [8; COLS];
    }

    /// Dessine dans la zone 76x16 en (x, y).
    pub fn draw(&self, p: &mut Painter, x: i32, y: i32, settings: &VisSettings) {
        let c = p.skin.vis.0;
        p.fill(x, y, 76, 16, c[0]);
        for gy in (1..16).step_by(2) {
            for gx in (0..76).step_by(2) {
                p.px(x + gx, y + gy, c[1]);
            }
        }
        match settings.mode {
            VisMode::Off => {}
            VisMode::Analyzer => {
                let heights = self.column_heights(settings.thick_bands, &self.shown_bars);
                let peaks = self.column_heights(settings.thick_bands, &self.shown_peaks);
                for col in 0..COLS {
                    if settings.thick_bands && col % 4 == 3 {
                        continue;
                    }
                    let h = heights[col].round() as i32;
                    for r in 0..h.clamp(0, 15) {
                        let color = match settings.analyzer_style {
                            AnalyzerStyle::Normal => 17 - r,
                            AnalyzerStyle::Fire => (h + 2 - r).clamp(2, 17),
                            AnalyzerStyle::Line => (17 - h).clamp(2, 17),
                        };
                        p.px(x + col as i32, y + 15 - r, c[color as usize]);
                    }
                    if settings.peaks {
                        let pk = peaks[col].round() as i32;
                        if pk >= 1 {
                            p.px(x + col as i32, y + 15 - pk.min(15), c[23]);
                        }
                    }
                }
            }
            VisMode::Oscilloscope => {
                let mut prev = self.shown_scope[0];
                for col in 0..COLS {
                    let v = self.shown_scope[col];
                    let color = |v: i32| c[(18 + (v / 2 - 4).abs()).clamp(18, 22) as usize];
                    match settings.scope_style {
                        ScopeStyle::Dots => p.px(x + col as i32, y + v, color(v)),
                        ScopeStyle::Lines => {
                            let (a, b) = if prev < v { (prev, v) } else { (v, prev) };
                            for yy in a..=b {
                                p.px(x + col as i32, y + yy, color(yy));
                            }
                        }
                        ScopeStyle::Solid => {
                            let (a, b) = if v < 8 { (v, 7) } else { (8, v) };
                            for yy in a..=b {
                                p.px(x + col as i32, y + yy, color(yy));
                            }
                        }
                    }
                    prev = v;
                }
            }
        }
    }

    /// Version réduite (mode bandeau) : 38x5.
    pub fn draw_small(&self, p: &mut Painter, x: i32, y: i32, settings: &VisSettings) {
        let c = p.skin.vis.0;
        p.fill(x, y, 38, 5, c[0]);
        match settings.mode {
            VisMode::Off => {}
            VisMode::Analyzer => {
                for col in 0..37 {
                    let h = (self.shown_bars[col * 2].max(self.shown_bars[(col * 2 + 1).min(COLS - 1)]) * 5.0 / 15.0).round() as i32;
                    for r in 0..h.clamp(0, 5) {
                        let color = 17 - r * 3;
                        p.px(x + col as i32, y + 4 - r, c[color as usize]);
                    }
                }
            }
            VisMode::Oscilloscope => {
                for col in 0..38 {
                    let v = self.shown_scope[(col * COLS / 38).min(COLS - 1)] * 5 / 16;
                    p.px(x + col as i32, y + v.clamp(0, 4), c[18]);
                }
            }
        }
    }

    fn column_heights(&self, thick: bool, src: &[f32; COLS]) -> [f32; COLS] {
        if !thick {
            return *src;
        }
        let mut out = [0f32; COLS];
        for group in 0..19 {
            let cols = group * 4..(group * 4 + 3).min(COLS);
            let avg = cols.clone().map(|i| src[i]).sum::<f32>() / cols.len() as f32;
            for i in cols {
                out[i] = avg;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_peaks_in_the_right_band() {
        let mut v = ClassicVis::new();
        let rate = 44_100.0;
        let f = 1000.0;
        let s: Vec<f32> = (0..WINDOW).map(|i| (i as f32 / rate * f * std::f32::consts::TAU).sin() * 0.5).collect();
        v.analyze(&s, &s);
        let (imax, _) = v.target.iter().enumerate().fold((0, 0f32), |(bi, bv), (i, &t)| if t > bv { (i, t) } else { (bi, bv) });
        // 1 kHz ≈ bin 11,6 ; la bande qui contient ce bin.
        let bin = f / (rate / FFT_N as f32);
        let expected = v.bin_edges.windows(2).position(|w| bin >= w[0].floor() && bin < w[1].max(w[0] + 1.0)).unwrap();
        assert!((imax as i32 - expected as i32).abs() <= 2, "bande max {imax}, attendue {expected}");
        assert!(v.target[imax] > 10.0);
    }

    #[test]
    fn bars_fall_and_peaks_linger() {
        let mut v = ClassicVis::new();
        v.target = [15.0; COLS];
        let s = VisSettings::default();
        v.advance(STEP * 1.01, &s);
        assert_eq!(v.bars[0], 15.0);
        v.target = [0.0; COLS];
        v.advance(STEP * 10.0, &s);
        assert!(v.bars[0] < 15.0 && v.bars[0] > 5.0, "{}", v.bars[0]);
        assert!(v.peaks[0] > v.bars[0]);
    }
}
