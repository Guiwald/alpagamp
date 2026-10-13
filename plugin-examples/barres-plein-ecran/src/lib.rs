//! **Barres plein écran** : un gros analyseur de spectre rétro, en diodes, aux couleurs du skin (ADR 0013).

use alpagamp_plugin::{Image, Plugin, Sound, Vis, host, mix, rgb};

const INFO: &str = r#"{
  "name": {"en": "Fullscreen Bars", "fr": "Barres plein écran", "es": "Barras a pantalla completa", "de": "Vollbild-Balken", "it": "Barre a schermo intero", "ca": "Barres a pantalla completa", "ga": "Barraí lánscáileáin"},
  "kind": "vis",
  "author": "Alpagamp",
  "version": "1.0.0",
  "description": {
    "en": "A big retro spectrum analyzer made of LEDs, in the colors of your skin.",
    "fr": "Un gros analyseur de spectre rétro en diodes, aux couleurs de ton skin.",
    "es": "Un gran analizador de espectro retro hecho de LEDs, con los colores de tu skin.",
    "de": "Ein großer Retro-Spektrumanalysator aus LEDs, in den Farben deines Skins.",
    "it": "Un grande analizzatore di spettro retrò a LED, con i colori della tua skin.",
    "ca": "Un gran analitzador d'espectre retro fet de LEDs, amb els colors de la teua aparença.",
    "ga": "Anailíseoir speictrim mór retro déanta as LEDanna, i ndathanna do chraicinn."
  },
  "options": [
    {"id": "bars", "label": {"en": "Bars", "fr": "Barres", "es": "Barras", "de": "Balken", "it": "Barre", "ca": "Barres", "ga": "Barraí"},
     "values": [{"value": 16, "label": "16"}, {"value": 24, "label": "24"}, {"value": 32, "label": "32"}, {"value": 48, "label": "48"}],
     "default": 32},
    {"id": "colors", "label": {"en": "Colors", "fr": "Couleurs", "es": "Colores", "de": "Farben", "it": "Colori", "ca": "Colors", "ga": "Dathanna"},
     "values": [
       {"value": 0, "label": {"en": "Skin", "fr": "Skin", "es": "Skin", "de": "Skin", "it": "Skin", "ca": "Aparença", "ga": "Craiceann"}},
       {"value": 1, "label": {"en": "Fire", "fr": "Feu", "es": "Fuego", "de": "Feuer", "it": "Fuoco", "ca": "Foc", "ga": "Tine"}},
       {"value": 2, "label": {"en": "Ice", "fr": "Glace", "es": "Hielo", "de": "Eis", "it": "Ghiaccio", "ca": "Gel", "ga": "Oighear"}},
       {"value": 3, "label": {"en": "Rainbow", "fr": "Arc-en-ciel", "es": "Arcoíris", "de": "Regenbogen", "it": "Arcobaleno", "ca": "Arc de Sant Martí", "ga": "Bogha báistí"}}],
     "default": 0},
    {"id": "peaks", "label": {"en": "Peaks", "fr": "Crêtes", "es": "Picos", "de": "Spitzen", "it": "Picchi", "ca": "Pics", "ga": "Buaicí"},
     "values": [
       {"value": 1, "label": {"en": "Yes", "fr": "Oui", "es": "Sí", "de": "Ja", "it": "Sì", "ca": "Sí", "ga": "Tá"}},
       {"value": 0, "label": {"en": "No", "fr": "Non", "es": "No", "de": "Nein", "it": "No", "ca": "No", "ga": "Níl"}}],
     "default": 1},
    {"id": "falloff", "label": {"en": "Falloff", "fr": "Chute", "es": "Caída", "de": "Abfall", "it": "Caduta", "ca": "Caiguda", "ga": "Titim"},
     "values": [
       {"value": 0, "label": {"en": "Slow", "fr": "Lente", "es": "Lenta", "de": "Langsam", "it": "Lenta", "ca": "Lenta", "ga": "Mall"}},
       {"value": 1, "label": {"en": "Normal", "fr": "Normale", "es": "Normal", "de": "Normal", "it": "Normale", "ca": "Normal", "ga": "Gnáth"}},
       {"value": 2, "label": {"en": "Fast", "fr": "Rapide", "es": "Rápida", "de": "Schnell", "it": "Veloce", "ca": "Ràpida", "ga": "Tapa"}}],
     "default": 1}
  ]
}"#;

/// Fréquence supposée (l'hôte ne la donne pas aux visualisations : à 48 kHz, tout glisse de 9 %, invisible).
const RATE: f32 = 44_100.0;
const LOW_HZ: f32 = 40.0;
const HIGH_HZ: f32 = 16_000.0;
const MAX_BARS: usize = 48;

pub struct Barres {
    bars: usize,
    colors: i32,
    peaks_on: bool,
    falloff: f32,
    level: [f32; MAX_BARS],
    peak: [f32; MAX_BARS],
    peak_hold: [f32; MAX_BARS],
    peak_speed: [f32; MAX_BARS],
    last_ms: Option<u32>,
}

impl Plugin for Barres {
    const INFO: &'static str = INFO;

    fn new() -> Self {
        Barres {
            bars: 32,
            colors: 0,
            peaks_on: true,
            falloff: 1.2,
            level: [0.0; MAX_BARS],
            peak: [0.0; MAX_BARS],
            peak_hold: [0.0; MAX_BARS],
            peak_speed: [0.0; MAX_BARS],
            last_ms: None,
        }
    }

    fn option(&mut self, index: u32, value: i32) {
        match index {
            0 => self.bars = (value.clamp(4, MAX_BARS as i32)) as usize,
            1 => self.colors = value,
            2 => self.peaks_on = value != 0,
            3 => self.falloff = [0.6, 1.2, 2.4][value.clamp(0, 2) as usize],
            _ => {}
        }
    }
}

/// Niveau (0–1) de chaque barre : bandes réparties en échelle logarithmique, en décibels.
pub fn levels(sound: &Sound, bars: usize, out: &mut [f32]) {
    let bin = |hz: f32| hz * 1152.0 / RATE;
    let ratio = (HIGH_HZ / LOW_HZ).powf(1.0 / bars as f32);
    for (x, o) in out.iter_mut().enumerate().take(bars) {
        let lo = bin(LOW_HZ * ratio.powi(x as i32));
        let hi = bin(LOW_HZ * ratio.powi(x as i32 + 1)).max(lo + 1.0);
        let mut m = 0f32;
        for ch in sound.spectrum {
            // Bandes plus étroites qu'une bande de l'analyse : interpolation.
            let i = (lo as usize).min(574);
            let t = lo - i as f32;
            m = m.max(ch[i] * (1.0 - t) + ch[i + 1] * t);
            let mut b = lo.ceil() as usize;
            while (b as f32) < hi && b < 576 {
                m = m.max(ch[b]);
                b += 1;
            }
        }
        // Les aigus ont moins d'énergie : +3 dB par octave environ.
        let tilt = 1.0 + 2.0 * x as f32 / bars as f32;
        let db = 20.0 * (m * tilt + 1e-9).log10();
        *o = ((db + 54.0) / 54.0).clamp(0.0, 1.0);
    }
}

fn hue(h: f32, v: f32) -> u32 {
    let h = (h.rem_euclid(1.0)) * 6.0;
    let f = h - h.floor();
    let (r, g, b) = match h as i32 {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    };
    rgb((r * v * 255.0) as u8, (g * v * 255.0) as u8, (b * v * 255.0) as u8)
}

fn gradient(stops: &[u32], t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (t as usize).min(stops.len() - 2);
    mix(stops[i], stops[i + 1], t - i as f32)
}

impl Barres {
    /// Couleur d'une diode allumée : barre `x`, hauteur `f` (0 en bas, 1 en haut).
    fn color(&self, skin: &[u32; 24], x: usize, f: f32) -> u32 {
        match self.colors {
            1 => gradient(&[rgb(90, 0, 0), rgb(220, 30, 0), rgb(255, 140, 0), rgb(255, 230, 60), rgb(255, 255, 220)], f),
            2 => gradient(&[rgb(0, 20, 90), rgb(0, 90, 220), rgb(0, 200, 255), rgb(200, 250, 255)], f),
            3 => hue(x as f32 / self.bars as f32 * 0.83, 0.45 + 0.55 * f),
            // Couleurs du skin : 17 (bas) à 2 (haut).
            _ => skin[17 - (f * 15.0).round().clamp(0.0, 15.0) as usize],
        }
    }

    fn advance(&mut self, target: &[f32], dt: f32) {
        for x in 0..self.bars {
            let t = target[x];
            self.level[x] = if t > self.level[x] { t } else { (self.level[x] - self.falloff * dt).max(t) };
            if self.level[x] >= self.peak[x] {
                self.peak[x] = self.level[x];
                self.peak_hold[x] = 0.4;
                self.peak_speed[x] = 0.0;
            } else if self.peak_hold[x] > 0.0 {
                self.peak_hold[x] -= dt;
            } else {
                self.peak_speed[x] += 2.5 * dt;
                self.peak[x] = (self.peak[x] - self.peak_speed[x] * dt).max(0.0);
            }
        }
    }
}

impl Vis for Barres {
    fn render(&mut self, img: &mut Image, time_ms: u32, sound: &Sound) {
        let dt = match self.last_ms {
            Some(l) => (time_ms.wrapping_sub(l) as f32 / 1000.0).clamp(0.0, 0.1),
            None => 0.0,
        };
        self.last_ms = Some(time_ms);
        let mut target = [0f32; MAX_BARS];
        levels(sound, self.bars, &mut target);
        self.advance(&target, dt);

        let mut skin = [0u32; 24];
        for (i, c) in skin.iter_mut().enumerate() {
            *c = host::skin_color(i as u32);
        }
        let bg = if self.colors == 0 { skin[0] } else { rgb(0, 0, 0) };
        let peak_color = if self.colors == 0 { skin[23] } else { rgb(255, 255, 255) };
        img.fill(bg);

        let (w, h) = (img.width as i32, img.height as i32);
        let n = self.bars as i32;
        // Diodes : hauteur et écart.
        let seg = (h / 36).max(2);
        let gap_y = if seg >= 4 { 2 } else { 1 };
        let segs = ((h - 2) / seg).max(1);
        let gap_x = (w / (n * 6)).max(1);
        let bar_w = ((w - gap_x * (n + 1)) / n).max(1);
        let left = (w - (bar_w * n + gap_x * (n - 1))) / 2;
        let bottom = h - 1 - (h - 2 - segs * seg) / 2;

        for x in 0..self.bars {
            let bx = left + x as i32 * (bar_w + gap_x);
            let lit = (self.level[x] * segs as f32).round() as i32;
            for s in 0..segs {
                let f = s as f32 / (segs - 1).max(1) as f32;
                let c = self.color(&skin, x, f);
                let c = if s < lit { c } else { mix(bg, c, 0.13) };
                let y = bottom - (s + 1) * seg + 1;
                img.rect(bx, y, bar_w, seg - gap_y, c);
            }
            if self.peaks_on && self.peak[x] > 0.01 {
                let s = ((self.peak[x] * segs as f32).round() as i32 - 1).clamp(0, segs - 1);
                let y = bottom - (s + 1) * seg + 1;
                img.rect(bx, y, bar_w, seg - gap_y, peak_color);
            }
        }
    }
}

alpagamp_plugin::vis_plugin!(Barres);

#[cfg(test)]
mod tests {
    use super::*;

    fn sound_with(bin: usize, amp: f32) -> (Vec<f32>, Vec<f32>) {
        let mut s = vec![0f32; 1152];
        s[bin] = amp;
        s[576 + bin] = amp;
        (s, vec![0f32; 1152])
    }

    #[test]
    fn a_tone_lights_its_bar() {
        // 1 kHz ≈ bande 26.
        let (s, w) = sound_with(26, 0.8);
        let sound = Sound { spectrum: [&s[..576], &s[576..]], wave: [&w[..576], &w[576..]] };
        let mut out = [0f32; 32];
        levels(&sound, 32, &mut out);
        let best = out.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap().0;
        assert!((14..=20).contains(&best), "barre {best}");
        assert!(out[0] < 0.01 && out[31] < 0.01);
    }

    #[test]
    fn renders_and_falls() {
        let mut b = Barres::new();
        let (s, w) = sound_with(26, 1.0);
        let sound = Sound { spectrum: [&s[..576], &s[576..]], wave: [&w[..576], &w[576..]] };
        let mut px = vec![0u32; 320 * 180];
        let mut img = Image { width: 320, height: 180, pixels: &mut px };
        b.render(&mut img, 0, &sound);
        let peak = b.level.iter().copied().fold(0.0, f32::max);
        assert!(peak > 0.9);
        let silent = vec![0f32; 1152];
        let quiet = Sound { spectrum: [&silent[..576], &silent[576..]], wave: [&w[..576], &w[576..]] };
        b.render(&mut img, 100, &quiet);
        let after = b.level.iter().copied().fold(0.0, f32::max);
        assert!(after < peak && after > 0.5, "{after}");
        assert!(px.iter().any(|&p| p != px[0]));
    }
}
