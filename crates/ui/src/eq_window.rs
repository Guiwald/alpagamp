//! La fenêtre de l'égaliseur (275x116, ou 275x14 en bandeau).

use alpagamp_core::eq::{BAND_LABELS, CENTER, EqSettings, slider_to_db};
use alpagamp_skin::sprites::{self, *};
use alpagamp_skin::{Image, Skin};

use crate::paint::Painter;
use crate::{Action, Mods};

pub const WIDTH: u32 = 275;
pub const HEIGHT: u32 = 116;

/// Ce que l'égaliseur affiche.
#[derive(Clone, Debug, Default)]
pub struct EqView {
    pub eq: EqSettings,
    pub active: bool,
    pub volume: u8,
    pub balance: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Zone {
    Close,
    Shade,
    On,
    Auto,
    Presets,
    Preamp,
    Band(usize),
    Quick,
    TitleBar,
    Background,
    ShadeVolume,
    ShadeBalance,
}

const BAND_X0: i32 = 78;
const BAND_DX: i32 = 18;
const SLIDER_Y: i32 = 38;

fn inside(x: i32, y: i32, rx: i32, ry: i32, rw: i32, rh: i32) -> bool {
    x >= rx && y >= ry && x < rx + rw && y < ry + rh
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    Slider(Zone),
    ShadeVolume,
    ShadeBalance,
}

pub struct EqWindow {
    pub shade: bool,
    pressed: Option<Zone>,
    pressed_inside: bool,
    drag: Option<Drag>,
    /// Message pour le titre défilant de la fenêtre principale (« EQ: 60HZ: +3.0 DB »).
    pub status: Option<String>,
}

impl Default for EqWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl EqWindow {
    pub fn new() -> Self {
        Self { shade: false, pressed: None, pressed_inside: false, drag: None, status: None }
    }

    pub fn size(&self) -> (u32, u32) {
        (WIDTH, if self.shade { 14 } else { HEIGHT })
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.is_some() || self.pressed.is_some()
    }

    fn zone_at(&self, x: i32, y: i32) -> Zone {
        if self.shade {
            return if inside(x, y, 254, 3, 9, 9) {
                Zone::Shade
            } else if inside(x, y, 264, 3, 9, 9) {
                Zone::Close
            } else if inside(x, y, 61, 3, 97, 9) {
                Zone::ShadeVolume
            } else if inside(x, y, 164, 3, 43, 9) {
                Zone::ShadeBalance
            } else {
                Zone::TitleBar
            };
        }
        if y < 14 {
            return if inside(x, y, 254, 3, 9, 9) {
                Zone::Shade
            } else if inside(x, y, 264, 3, 9, 9) {
                Zone::Close
            } else {
                Zone::TitleBar
            };
        }
        if inside(x, y, 14, 18, 26, 12) {
            return Zone::On;
        }
        if inside(x, y, 40, 18, 32, 12) {
            return Zone::Auto;
        }
        if inside(x, y, 217, 18, 44, 12) {
            return Zone::Presets;
        }
        if inside(x, y, 21, SLIDER_Y, 14, 63) {
            return Zone::Preamp;
        }
        for i in 0..10 {
            if inside(x, y, BAND_X0 + BAND_DX * i as i32, SLIDER_Y, 14, 63) {
                return Zone::Band(i);
            }
        }
        if inside(x, y, 42, 36, 28, 66) {
            return Zone::Quick;
        }
        Zone::Background
    }

    /// Valeur du curseur (0 = +12 dB … 63 = −12 dB) pour une ordonnée.
    fn value_at(y: i32, mods: Mods) -> u8 {
        let v = ((y - SLIDER_Y - 5) as f32 * 63.0 / 51.0).round().clamp(0.0, 63.0) as u8;
        if !mods.shift && (30..=32).contains(&v) { CENTER } else { v }
    }

    fn band_status(i: usize, v: u8) -> String {
        format!("EQ: {}HZ: {:+.1} DB", BAND_LABELS[i], slider_to_db(v))
    }

    pub fn mouse_down(&mut self, x: i32, y: i32, mods: Mods, view: &EqView) -> Vec<Action> {
        let z = self.zone_at(x, y);
        match z {
            Zone::Close | Zone::Shade | Zone::On | Zone::Auto | Zone::Presets => {
                self.pressed = Some(z);
                self.pressed_inside = true;
                Vec::new()
            }
            Zone::Preamp | Zone::Band(_) => {
                self.drag = Some(Drag::Slider(z));
                self.drag_to(x, y, mods, view)
            }
            Zone::Quick => {
                let v = if y < 58 { 0 } else if y < 80 { CENTER } else { 63 };
                vec![Action::SetEqAllBands(v)]
            }
            Zone::ShadeVolume => {
                self.drag = Some(Drag::ShadeVolume);
                self.drag_to(x, y, mods, view)
            }
            Zone::ShadeBalance => {
                self.drag = Some(Drag::ShadeBalance);
                self.drag_to(x, y, mods, view)
            }
            Zone::TitleBar | Zone::Background => vec![Action::StartMove],
        }
    }

    pub fn mouse_move(&mut self, x: i32, y: i32, mods: Mods, view: &EqView) -> Vec<Action> {
        if let Some(p) = self.pressed {
            self.pressed_inside = self.zone_at(x, y) == p;
            return Vec::new();
        }
        if self.drag.is_some() {
            return self.drag_to(x, y, mods, view);
        }
        Vec::new()
    }

    pub fn mouse_up(&mut self, x: i32, y: i32, _mods: Mods, view: &EqView) -> Vec<Action> {
        self.drag = None;
        self.status = None;
        let Some(p) = self.pressed.take() else { return Vec::new() };
        if self.zone_at(x, y) != p {
            return Vec::new();
        }
        match p {
            Zone::Close => vec![Action::Close],
            Zone::Shade => vec![Action::ToggleShade],
            Zone::On => vec![Action::ToggleEqEnabled],
            Zone::Auto => vec![Action::ToggleEqAuto],
            Zone::Presets => vec![Action::EqPresetsMenu],
            _ => {
                let _ = view;
                Vec::new()
            }
        }
    }

    pub fn double_click(&mut self, x: i32, y: i32) -> Vec<Action> {
        match self.zone_at(x, y) {
            Zone::TitleBar => vec![Action::ToggleShade],
            _ => Vec::new(),
        }
    }

    fn drag_to(&mut self, x: i32, y: i32, mods: Mods, view: &EqView) -> Vec<Action> {
        match self.drag {
            Some(Drag::Slider(z)) => {
                // En balayant d'une colonne à l'autre, on passe à la bande voisine.
                let z = match (z, self.zone_at(x, y.clamp(SLIDER_Y, SLIDER_Y + 62))) {
                    (Zone::Band(_), Zone::Band(j)) => Zone::Band(j),
                    _ => z,
                };
                self.drag = Some(Drag::Slider(z));
                let v = Self::value_at(y, mods);
                match z {
                    Zone::Preamp => {
                        self.status = Some(format!("EQ: PREAMP: {:+.1} DB", slider_to_db(v)));
                        vec![Action::SetPreamp(v)]
                    }
                    Zone::Band(i) => {
                        self.status = Some(Self::band_status(i, v));
                        vec![Action::SetEqBand(i, v)]
                    }
                    _ => Vec::new(),
                }
            }
            Some(Drag::ShadeVolume) => {
                let v = ((x - 61 - 1) as f32 * 255.0 / 94.0).round().clamp(0.0, 255.0) as u8;
                self.status = Some(format!("VOLUME: {}%", (v as u32 * 100 + 127) / 255));
                vec![Action::SetVolume(v)]
            }
            Some(Drag::ShadeBalance) => {
                let b = ((x - 164 - 20) as f32 * 127.0 / 20.0).round().clamp(-127.0, 127.0) as i32;
                let b = if !mods.shift && b.abs() < 12 { 0 } else { b };
                let _ = view;
                vec![Action::SetBalance(b as i8)]
            }
            None => Vec::new(),
        }
    }

    fn down(&self, z: Zone) -> bool {
        self.pressed == Some(z) && self.pressed_inside
    }

    pub fn render(&mut self, fb: &mut Image, skin: &Skin, view: &EqView) {
        let (w, h) = self.size();
        if fb.width != w || fb.height != h {
            *fb = Image::new(w, h, 0xFF00_0000);
        }
        let mut p = Painter::new(fb, skin);
        if self.shade {
            p.sprite(if view.active { EQ_SHADE_ACTIVE } else { EQ_SHADE_INACTIVE }, 0, 0);
            if self.down(Zone::Shade) {
                p.sprite(EQ_MAXIMIZE_DOWN, 254, 3);
            }
            p.sprite(if self.down(Zone::Close) { EQ_SHADE_CLOSE_DOWN } else { EQ_SHADE_CLOSE }, 264, 3);
            let third = |f: f32, s: &[Sprite; 3]| if f < 1.0 / 3.0 { s[0] } else if f < 2.0 / 3.0 { s[1] } else { s[2] };
            let fv = view.volume as f32 / 255.0;
            p.sprite(third(fv, &EQ_SHADE_VOLUME_THUMB), 61 + (fv * 94.0).round() as i32, 4);
            let fb_ = (view.balance as f32 + 127.0) / 254.0;
            p.sprite(third(fb_, &EQ_SHADE_BALANCE_THUMB), 164 + (fb_ * 40.0).round() as i32, 4);
            return;
        }
        p.sprite(EQ_BACKGROUND, 0, 0);
        p.sprite(if view.active { EQ_TITLE_ACTIVE } else { EQ_TITLE_INACTIVE }, 0, 0);
        if self.down(Zone::Shade) {
            if skin.has_eq_ex {
                p.sprite(EQ_MINIMIZE_DOWN, 254, 3);
            }
        }
        p.sprite(if self.down(Zone::Close) { EQ_CLOSE_DOWN } else { EQ_CLOSE }, 264, 3);
        let idx = |on: bool, down: bool| on as usize + 2 * down as usize;
        p.sprite(EQ_ON[idx(view.eq.enabled, self.down(Zone::On))], 14, 18);
        p.sprite(EQ_AUTO[idx(view.eq.auto, self.down(Zone::Auto))], 40, 18);
        p.sprite(if self.down(Zone::Presets) { EQ_PRESETS_DOWN } else { EQ_PRESETS }, 217, 18);
        self.draw_graph(&mut p, view);
        let dragging = |z: Zone| self.drag == Some(Drag::Slider(z));
        Self::slider(&mut p, 21, view.eq.preamp, dragging(Zone::Preamp));
        for i in 0..10 {
            Self::slider(&mut p, BAND_X0 + BAND_DX * i as i32, view.eq.bands[i], dragging(Zone::Band(i)));
        }
    }

    fn slider(p: &mut Painter, x: i32, v: u8, pressed: bool) {
        let n = 27 - (v as i32 * 28 / 64);
        p.sprite(sprites::eq_slider_background(n.clamp(0, 27)), x, SLIDER_Y);
        let ty = SLIDER_Y + (v as f32 * 51.0 / 63.0).round() as i32;
        p.sprite(if pressed { EQ_SLIDER_THUMB_DOWN } else { EQ_SLIDER_THUMB }, x + 1, ty);
    }

    /// Courbe de réponse : spline passant par les 10 bandes, colorée ligne par ligne.
    fn draw_graph(&self, p: &mut Painter, view: &EqView) {
        p.sprite(EQ_GRAPH_BACKGROUND, 86, 17);
        let colors = p.skin.sheet(EQ_GRAPH_LINE_COLORS.sheet);
        let color_at = |row: i32| colors.get(EQ_GRAPH_LINE_COLORS.x, EQ_GRAPH_LINE_COLORS.y + row.clamp(0, 18)).unwrap_or(0xFF00_FF00);
        // Ligne du préampli.
        let py = (view.eq.preamp as f32 * 18.0 / 63.0).round() as i32;
        p.sprite(EQ_PREAMP_LINE, 86, 17 + py);
        let ys: Vec<f32> = view.eq.bands.iter().map(|&b| b as f32 * 18.0 / 63.0).collect();
        let curve = spline(&ys, 113);
        let mut prev: Option<i32> = None;
        for (x, y) in curve.iter().enumerate() {
            let y = y.round().clamp(0.0, 18.0) as i32;
            let (a, b) = match prev {
                Some(pv) if pv < y => (pv + 1, y),
                Some(pv) if pv > y => (y, pv - 1),
                _ => (y, y),
            };
            for yy in a..=b {
                p.px(86 + x as i32, 17 + yy, color_at(yy));
            }
            prev = Some(y);
        }
    }
}

/// Spline cubique naturelle passant par `ys` (points équidistants), échantillonnée sur `n` pixels.
pub fn spline(ys: &[f32], n: usize) -> Vec<f32> {
    let k = ys.len();
    if k < 2 {
        return vec![ys.first().copied().unwrap_or(0.0); n];
    }
    // Résolution du système tridiagonal pour les dérivées secondes (pas = 1).
    let mut m = vec![0f32; k];
    let mut c = vec![0f32; k];
    let mut d = vec![0f32; k];
    for i in 1..k - 1 {
        let rhs = 6.0 * (ys[i + 1] - 2.0 * ys[i] + ys[i - 1]);
        let denom = 4.0 - c[i - 1];
        c[i] = 1.0 / denom;
        d[i] = (rhs - d[i - 1]) / denom;
    }
    for i in (1..k - 1).rev() {
        m[i] = d[i] - c[i] * m[i + 1];
    }
    (0..n)
        .map(|px| {
            let t = px as f32 * (k - 1) as f32 / (n - 1) as f32;
            let i = (t.floor() as usize).min(k - 2);
            let u = t - i as f32;
            let a = 1.0 - u;
            a * ys[i] + u * ys[i + 1] + ((a * a * a - a) * m[i] + (u * u * u - u) * m[i + 1]) / 6.0
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spline_passes_through_points() {
        let ys = [0.0, 5.0, 2.0, 9.0];
        let s = spline(&ys, 4);
        for (a, b) in s.iter().zip(ys) {
            assert!((a - b).abs() < 1e-4);
        }
    }

    #[test]
    fn slider_drag_and_snap() {
        let mut w = EqWindow::new();
        let v = EqView::default();
        let a = w.mouse_down(BAND_X0 + 3, SLIDER_Y + 5, Mods::default(), &v);
        assert_eq!(a, vec![Action::SetEqBand(0, 0)]);
        let a = w.mouse_move(BAND_X0 + BAND_DX * 3 + 3, SLIDER_Y + 30, Mods::default(), &v);
        assert_eq!(a, vec![Action::SetEqBand(3, CENTER)], "aimanté au centre et passé à la bande 3");
        assert!(w.status.as_deref().unwrap().starts_with("EQ: 600HZ"));
        w.mouse_up(0, 0, Mods::default(), &v);
        assert_eq!(w.mouse_down(20, 22, Mods::default(), &v), Vec::<Action>::new());
        assert_eq!(w.mouse_up(20, 22, Mods::default(), &v), vec![Action::ToggleEqEnabled]);
    }
}
