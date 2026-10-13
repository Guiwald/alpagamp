//! **Enhancer** : élargisseur stéréo, renfort de basses et clarté, avec un limiteur pour ne jamais saturer
//! (ADR 0013, dans l'esprit des « enhancers » de l'époque).

use alpagamp_plugin::{Dsp, Plugin};

const INFO: &str = r#"{
  "name": "Enhancer",
  "kind": "dsp",
  "author": "Alpagamp",
  "version": "1.0.0",
  "description": {
    "en": "Wider stereo, deeper bass and a little more clarity. A limiter keeps the sound from clipping.",
    "fr": "Une stéréo plus large, des basses plus profondes et un peu plus de clarté. Un limiteur évite la saturation.",
    "es": "Estéreo más amplio, graves más profundos y algo más de claridad. Un limitador evita la saturación.",
    "de": "Breiteres Stereo, tieferer Bass und etwas mehr Klarheit. Ein Limiter verhindert Übersteuerung.",
    "it": "Stereo più ampio, bassi più profondi e un po' più di chiarezza. Un limitatore evita la saturazione.",
    "ca": "Un estèreo més ample, uns baixos més profunds i un poc més de claredat. Un limitador evita la saturació.",
    "ga": "Steiréafónach níos leithne, dord níos doimhne agus beagán níos mó soiléireachta. Coscann teorantóir an sáithiú."
  },
  "options": [
    {"id": "width", "label": {"en": "Stereo width", "fr": "Largeur stéréo", "es": "Amplitud estéreo", "de": "Stereobreite", "it": "Ampiezza stereo", "ca": "Amplària estèreo", "ga": "Leithead steiréa"},
     "values": [
       {"value": 0, "label": {"en": "Off", "fr": "Aucune", "es": "No", "de": "Aus", "it": "No", "ca": "Cap", "ga": "As"}},
       {"value": 25, "label": "+25 %"}, {"value": 50, "label": "+50 %"}, {"value": 75, "label": "+75 %"}, {"value": 100, "label": "+100 %"}],
     "default": 50},
    {"id": "bass", "label": {"en": "Bass", "fr": "Basses", "es": "Graves", "de": "Bass", "it": "Bassi", "ca": "Baixos", "ga": "Dord"},
     "values": [
       {"value": 0, "label": {"en": "Off", "fr": "Aucun renfort", "es": "No", "de": "Aus", "it": "No", "ca": "Cap", "ga": "As"}},
       {"value": 3, "label": "+3 dB"}, {"value": 6, "label": "+6 dB"}, {"value": 9, "label": "+9 dB"}],
     "default": 6},
    {"id": "clarity", "label": {"en": "Clarity", "fr": "Clarté", "es": "Claridad", "de": "Klarheit", "it": "Chiarezza", "ca": "Claredat", "ga": "Soiléireacht"},
     "values": [
       {"value": 0, "label": {"en": "Off", "fr": "Aucune", "es": "No", "de": "Aus", "it": "No", "ca": "Cap", "ga": "As"}},
       {"value": 2, "label": "+2 dB"}, {"value": 4, "label": "+4 dB"}],
     "default": 2}
  ]
}"#;

/// Filtre biquadratique (formules de R. Bristow-Johnson), une voie.
#[derive(Clone, Copy, Default)]
struct Biquad {
    b: [f32; 3],
    a: [f32; 2],
    x: [f32; 2],
    y: [f32; 2],
}

enum Shape {
    LowShelf,
    HighShelf,
    HighPass,
}

impl Biquad {
    fn new(shape: Shape, freq: f32, gain_db: f32, rate: f32) -> Biquad {
        let w0 = std::f32::consts::TAU * (freq / rate).min(0.45);
        let (sin, cos) = w0.sin_cos();
        let a = 10f32.powf(gain_db / 40.0);
        let sa = 2.0 * a.sqrt() * sin / 2.0 * std::f32::consts::SQRT_2;
        let (b0, b1, b2, a0, a1, a2) = match shape {
            Shape::LowShelf => (
                a * ((a + 1.0) - (a - 1.0) * cos + sa),
                2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                a * ((a + 1.0) - (a - 1.0) * cos - sa),
                (a + 1.0) + (a - 1.0) * cos + sa,
                -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                (a + 1.0) + (a - 1.0) * cos - sa,
            ),
            Shape::HighShelf => (
                a * ((a + 1.0) + (a - 1.0) * cos + sa),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                a * ((a + 1.0) + (a - 1.0) * cos - sa),
                (a + 1.0) - (a - 1.0) * cos + sa,
                2.0 * ((a - 1.0) - (a + 1.0) * cos),
                (a + 1.0) - (a - 1.0) * cos - sa,
            ),
            Shape::HighPass => {
                let alpha = sin / std::f32::consts::SQRT_2;
                ((1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha)
            }
        };
        Biquad { b: [b0 / a0, b1 / a0, b2 / a0], a: [a1 / a0, a2 / a0], x: [0.0; 2], y: [0.0; 2] }
    }

    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let y = self.b[0] * x + self.b[1] * self.x[0] + self.b[2] * self.x[1] - self.a[0] * self.y[0] - self.a[1] * self.y[1];
        self.x = [x, self.x[0]];
        // Pas de nombres dénormalisés (lents) quand le son s'éteint.
        let y = if y.abs() < 1e-20 { 0.0 } else { y };
        self.y = [y, self.y[0]];
        y
    }
}

pub struct Enhancer {
    width: f32,
    bass_db: f32,
    clarity_db: f32,
    rate: u32,
    side_hp: Biquad,
    bass: [Biquad; 2],
    clarity: [Biquad; 2],
    /// Enveloppe du limiteur.
    env: f32,
    release: f32,
}

/// Niveau au-delà duquel le limiteur agit.
const CEILING: f32 = 0.95;

impl Plugin for Enhancer {
    const INFO: &'static str = INFO;

    fn new() -> Self {
        let mut e = Enhancer {
            width: 0.5,
            bass_db: 6.0,
            clarity_db: 2.0,
            rate: 0,
            side_hp: Biquad::default(),
            bass: [Biquad::default(); 2],
            clarity: [Biquad::default(); 2],
            env: 0.0,
            release: 0.0,
        };
        e.setup(44_100);
        e
    }

    fn option(&mut self, index: u32, value: i32) {
        match index {
            0 => self.width = value.clamp(0, 100) as f32 / 100.0,
            1 => self.bass_db = value.clamp(0, 12) as f32,
            2 => self.clarity_db = value.clamp(0, 6) as f32,
            _ => return,
        }
        let r = self.rate;
        self.setup(r);
    }
}

impl Enhancer {
    fn setup(&mut self, rate: u32) {
        self.rate = rate;
        let r = rate as f32;
        self.side_hp = Biquad::new(Shape::HighPass, 180.0, 0.0, r);
        self.bass = [Biquad::new(Shape::LowShelf, 90.0, self.bass_db, r); 2];
        self.clarity = [Biquad::new(Shape::HighShelf, 6000.0, self.clarity_db, r); 2];
        // Le limiteur relâche en ~150 ms.
        self.release = (-1.0 / (0.15 * r)).exp();
    }
}

impl Dsp for Enhancer {
    fn process(&mut self, samples: &mut [f32], rate: u32) {
        if rate != self.rate {
            self.setup(rate);
        }
        let bass = self.bass_db > 0.0;
        let clarity = self.clarity_db > 0.0;
        for f in samples.chunks_exact_mut(2) {
            let (mut l, mut r) = (f[0], f[1]);
            if self.width > 0.0 {
                // Côtés élargis, sauf le grave (qui reste au centre, sans effet de phase).
                let mid = (l + r) * 0.5;
                let side = (l - r) * 0.5;
                let side = side + self.width * self.side_hp.run(side);
                l = mid + side;
                r = mid - side;
            }
            if bass {
                l = self.bass[0].run(l);
                r = self.bass[1].run(r);
            }
            if clarity {
                l = self.clarity[0].run(l);
                r = self.clarity[1].run(r);
            }
            // Limiteur : attaque immédiate, relâchement doux.
            let peak = l.abs().max(r.abs());
            self.env = if peak > self.env { peak } else { self.env * self.release + peak * (1.0 - self.release) };
            let g = if self.env > CEILING { CEILING / self.env } else { 1.0 };
            f[0] = l * g;
            f[1] = r * g;
        }
    }
}

alpagamp_plugin::dsp_plugin!(Enhancer);

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(hz: f32, amp: f32, frames: usize, phase_r: f32) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let t = i as f32 / 44_100.0;
                [amp * (std::f32::consts::TAU * hz * t).sin(), amp * (std::f32::consts::TAU * hz * t + phase_r).sin()]
            })
            .collect()
    }

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
    }

    #[test]
    fn off_is_transparent() {
        let mut e = Enhancer::new();
        for (i, v) in [0, 0, 0].into_iter().enumerate() {
            e.option(i as u32, v);
        }
        let input = sine(440.0, 0.3, 4096, 0.5);
        let mut s = input.clone();
        e.process(&mut s, 44_100);
        let diff = s.iter().zip(&input).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
        assert!(diff < 1e-4, "{diff}");
    }

    #[test]
    fn bass_is_boosted() {
        let mut e = Enhancer::new();
        e.option(0, 0);
        e.option(2, 0);
        e.option(1, 6);
        let mut s = sine(50.0, 0.1, 44_100, 0.0);
        e.process(&mut s, 44_100);
        let gain = rms(&s[44_100..]) / rms(&sine(50.0, 0.1, 44_100, 0.0)[44_100..]);
        assert!((1.8..2.1).contains(&gain), "{gain}");
    }

    #[test]
    fn stereo_gets_wider() {
        let mut e = Enhancer::new();
        let mut s = sine(1000.0, 0.3, 22_050, 1.0);
        let side = |s: &[f32]| rms(&s.chunks(2).map(|f| f[0] - f[1]).collect::<Vec<_>>());
        let before = side(&s);
        e.process(&mut s, 44_100);
        let k = side(&s[4410..]) / before;
        assert!((1.3..1.7).contains(&k), "{k}");
    }

    #[test]
    fn never_clips() {
        let mut e = Enhancer::new();
        e.option(1, 9);
        let mut s = sine(60.0, 0.99, 22_050, 2.0);
        e.process(&mut s, 44_100);
        assert!(s.iter().all(|x| x.abs() <= CEILING + 1e-4));
    }
}
