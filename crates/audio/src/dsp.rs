//! Traitements : préampli, ReplayGain, égaliseur 10 bandes, limiteur.
//!
//! Égaliseur : 10 filtres en cloche (« peaking », formules RBJ) en série, Q ≈ 1,4. Écrit à partir des formules
//! standard ; aucun code tiers.

use alpagamp_core::eq::{BAND_FREQS, EqSettings, slider_to_db};

#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    // État par canal : x1, x2, y1, y2 (gauche puis droite).
    s: [[f32; 4]; 2],
}

impl Biquad {
    fn peaking(fs: f32, f0: f32, q: f32, db: f32) -> Biquad {
        let a = 10f32.powf(db / 40.0);
        let w0 = std::f32::consts::TAU * f0 / fs;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let a0 = 1.0 + alpha / a;
        Biquad {
            b0: (1.0 + alpha * a) / a0,
            b1: (-2.0 * cos) / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: (-2.0 * cos) / a0,
            a2: (1.0 - alpha / a) / a0,
            s: [[0.0; 4]; 2],
        }
    }

    /// Change les coefficients en gardant l'état (pas de clic quand on bouge un curseur).
    fn retune(&mut self, other: &Biquad) {
        self.b0 = other.b0;
        self.b1 = other.b1;
        self.b2 = other.b2;
        self.a1 = other.a1;
        self.a2 = other.a2;
    }

    #[inline]
    fn run(&mut self, ch: usize, x: f32) -> f32 {
        let s = &mut self.s[ch];
        let y = self.b0 * x + self.b1 * s[0] + self.b2 * s[1] - self.a1 * s[2] - self.a2 * s[3];
        s[1] = s[0];
        s[0] = x;
        s[3] = s[2];
        s[2] = y;
        y
    }
}

const Q: f32 = 1.4;

/// Chaîne de traitement appliquée dans le fil de décodage (avant la sortie).
pub struct Dsp {
    rate: u32,
    eq: EqSettings,
    bands: Vec<(usize, Biquad)>,
    /// Gain linéaire (préampli × ReplayGain).
    gain: f32,
    replay_gain_db: f32,
    limiter_gain: f32,
    pub limiter: bool,
}

impl Dsp {
    pub fn new(rate: u32) -> Self {
        let mut d = Dsp { rate, eq: EqSettings::default(), bands: Vec::new(), gain: 1.0, replay_gain_db: 0.0, limiter_gain: 1.0, limiter: true };
        d.rebuild(false);
        d
    }

    pub fn set_rate(&mut self, rate: u32) {
        if rate != self.rate {
            self.rate = rate;
            self.rebuild(false);
        }
    }

    pub fn set_eq(&mut self, eq: EqSettings) {
        self.eq = eq;
        self.rebuild(true);
    }

    pub fn set_replay_gain_db(&mut self, db: f32) {
        self.replay_gain_db = db;
        self.rebuild(true);
    }

    pub fn reset_state(&mut self) {
        for (_, b) in &mut self.bands {
            b.s = [[0.0; 4]; 2];
        }
        self.limiter_gain = 1.0;
    }

    fn rebuild(&mut self, keep_state: bool) {
        let active = self.eq.enabled && !self.eq.is_flat();
        let preamp_db = if active { slider_to_db(self.eq.preamp) } else { 0.0 };
        self.gain = 10f32.powf((preamp_db + self.replay_gain_db) / 20.0);
        let mut new_bands = Vec::new();
        if active {
            for (i, &f) in BAND_FREQS.iter().enumerate() {
                let db = slider_to_db(self.eq.bands[i]);
                if db.abs() < 0.01 || f >= 0.49 * self.rate as f32 {
                    continue;
                }
                new_bands.push((i, Biquad::peaking(self.rate as f32, f, Q, db)));
            }
        }
        if keep_state {
            for (i, nb) in new_bands.iter_mut() {
                if let Some((_, old)) = self.bands.iter().find(|(j, _)| j == i) {
                    let mut kept = *old;
                    kept.retune(nb);
                    *nb = kept;
                }
            }
        }
        self.bands = new_bands;
    }

    /// Traite un bloc stéréo entrelacé, en place.
    pub fn process(&mut self, buf: &mut [f32]) {
        let gain = self.gain;
        if self.bands.is_empty() && (gain - 1.0).abs() < 1e-6 && !self.limiter {
            return;
        }
        for frame in buf.chunks_exact_mut(2) {
            let mut l = frame[0] * gain;
            let mut r = frame[1] * gain;
            for (_, b) in &mut self.bands {
                l = b.run(0, l);
                r = b.run(1, r);
            }
            if self.limiter {
                // Limiteur de crête : attaque immédiate, relâchement ~0,5 s.
                let peak = l.abs().max(r.abs()) * self.limiter_gain;
                const THRESH: f32 = 0.95;
                if peak > THRESH {
                    self.limiter_gain *= THRESH / peak;
                } else {
                    let release = 1.0 / (0.5 * self.rate as f32);
                    self.limiter_gain = (self.limiter_gain + release).min(1.0);
                }
                l *= self.limiter_gain;
                r *= self.limiter_gain;
            }
            frame[0] = l;
            frame[1] = r;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alpagamp_core::eq::db_to_slider;

    fn rms_after(dsp: &mut Dsp, freq: f32) -> f32 {
        let rate = 44_100.0;
        let mut buf: Vec<f32> = (0..44_100).flat_map(|i| {
            let s = (i as f32 / rate * freq * std::f32::consts::TAU).sin() * 0.1;
            [s, s]
        }).collect();
        dsp.process(&mut buf);
        let tail = &buf[44_100..];
        (tail.iter().map(|x| x * x).sum::<f32>() / tail.len() as f32).sqrt()
    }

    #[test]
    fn band_gain_at_center_frequency() {
        let mut dsp = Dsp::new(44_100);
        dsp.limiter = false;
        let mut eq = EqSettings { enabled: true, ..Default::default() };
        eq.bands[4] = db_to_slider(12.0); // 1 kHz
        dsp.set_eq(eq);
        let base = 0.1 / 2f32.sqrt();
        let boosted = rms_after(&mut dsp, 1000.0);
        let db = 20.0 * (boosted / base).log10();
        assert!((db - 12.0).abs() < 0.5, "gain à 1 kHz : {db:.2} dB");
        let far = rms_after(&mut dsp, 60.0);
        let db_far = 20.0 * (far / base).log10();
        assert!(db_far.abs() < 1.5, "gain à 60 Hz : {db_far:.2} dB");
    }

    #[test]
    fn disabled_eq_is_transparent() {
        let mut dsp = Dsp::new(48_000);
        dsp.limiter = false;
        let mut eq = EqSettings::default();
        eq.bands = [0; 10];
        dsp.set_eq(eq);
        let mut buf = vec![0.3, -0.3, 0.2, 0.1];
        dsp.process(&mut buf);
        assert_eq!(buf, vec![0.3, -0.3, 0.2, 0.1]);
    }

    #[test]
    fn limiter_caps_peaks() {
        let mut dsp = Dsp::new(48_000);
        let mut buf = vec![2.0f32; 2000];
        dsp.process(&mut buf);
        assert!(buf.iter().all(|s| s.abs() <= 0.951));
    }
}
