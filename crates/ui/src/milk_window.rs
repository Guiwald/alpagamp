//! Fenêtre de visualisation : un cadre générique skinné (`gen.bmp`) autour de l'image des presets MilkDrop,
//! redimensionnable par le coin (pas de 25×29), plein écran par double-clic.
//!
//! La fenêtre se dessine dans une image dont la zone d'affichage est **transparente** (alpha 0) : l'application
//! compose l'image de la visualisation dessous, sur le GPU.

use alpagamp_skin::{Image, Skin};

use crate::Action;
use crate::gen_window::draw_frame;
use crate::paint::Painter;
use crate::playlist_window::snap_size;
use crate::text::{self, Align, Rect, TextRun};

const FONT: f32 = 9.0;
/// Fond des messages dans l'image (noir, un peu transparent).
const BOX: u32 = 0xD800_0000;

/// Ce que la fenêtre affiche en plus de l'image.
pub struct MilkView<'a> {
    pub active: bool,
    pub title: &'a str,
    /// Message passager en haut à gauche (nom du preset qui arrive…).
    pub message: Option<&'a str>,
    /// État durable en bas (téléchargement en cours, pas de GPU…).
    pub status: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Zone {
    Close,
    TitleBar,
    Resize,
    Viewport,
    Frame,
}

pub struct MilkWindow {
    pub w: u32,
    pub h: u32,
    pressed: Option<Zone>,
    pressed_inside: bool,
    resize: Option<(u32, u32, i32, i32)>,
    pub text: Vec<TextRun>,
}

impl Default for MilkWindow {
    fn default() -> Self {
        Self::new(275, 116 + 29 * 4)
    }
}

fn inside(x: i32, y: i32, rx: i32, ry: i32, rw: i32, rh: i32) -> bool {
    x >= rx && y >= ry && x < rx + rw && y < ry + rh
}

impl MilkWindow {
    pub fn new(w: u32, h: u32) -> Self {
        let (w, h) = snap_size(w as i32, h as i32);
        Self { w, h, pressed: None, pressed_inside: false, resize: None, text: Vec::new() }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.w, self.h)
    }

    pub fn is_dragging(&self) -> bool {
        self.pressed.is_some() || self.resize.is_some()
    }

    /// Zone d'affichage de l'image, en pixels skin.
    pub fn viewport(&self) -> Rect {
        Rect::new(11, 20, self.w as i32 - 19, self.h as i32 - 34)
    }

    fn zone_at(&self, x: i32, y: i32) -> Zone {
        let (w, h) = (self.w as i32, self.h as i32);
        if inside(x, y, w - 11, 3, 9, 9) {
            return Zone::Close;
        }
        if inside(x, y, w - 20, h - 14, 20, 14) || inside(x, y, w - 8, h - 24, 8, 10) {
            return Zone::Resize;
        }
        if y < 20 {
            return Zone::TitleBar;
        }
        let v = self.viewport();
        if inside(x, y, v.x, v.y, v.w, v.h) {
            return Zone::Viewport;
        }
        Zone::Frame
    }

    /// Le pointeur est-il sur l'image (pour le clic droit, la molette) ?
    pub fn in_viewport(&self, x: i32, y: i32) -> bool {
        self.zone_at(x, y) == Zone::Viewport
    }

    pub fn mouse_down(&mut self, x: i32, y: i32) -> Vec<Action> {
        match self.zone_at(x, y) {
            Zone::Close => {
                self.pressed = Some(Zone::Close);
                self.pressed_inside = true;
                Vec::new()
            }
            Zone::Resize => {
                self.resize = Some((self.w, self.h, x, y));
                Vec::new()
            }
            // On déplace la fenêtre en la saisissant n'importe où (comme les fenêtres de visualisation de Winamp).
            Zone::TitleBar | Zone::Frame | Zone::Viewport => vec![Action::StartMove],
        }
    }

    pub fn mouse_move(&mut self, x: i32, y: i32) -> Vec<Action> {
        if let Some((sw, sh, sx, sy)) = self.resize {
            let (nw, nh) = snap_size(sw as i32 + x - sx, sh as i32 + y - sy);
            if (nw, nh) != (self.w, self.h) {
                self.w = nw;
                self.h = nh;
                return vec![Action::MilkResize(nw, nh)];
            }
            return Vec::new();
        }
        if let Some(p) = self.pressed {
            self.pressed_inside = self.zone_at(x, y) == p;
        }
        Vec::new()
    }

    pub fn mouse_up(&mut self, x: i32, y: i32) -> Vec<Action> {
        self.resize = None;
        let Some(p) = self.pressed.take() else { return Vec::new() };
        if p == Zone::Close && self.zone_at(x, y) == Zone::Close { vec![Action::Close] } else { Vec::new() }
    }

    pub fn double_click(&mut self, x: i32, y: i32) -> Vec<Action> {
        match self.zone_at(x, y) {
            Zone::Viewport => vec![Action::MilkFullscreen],
            _ => Vec::new(),
        }
    }

    /// Clé de l'image dessinée : si elle n'a pas changé, inutile de redessiner et de renvoyer le cadre au GPU.
    pub fn chrome_key(&self, v: &MilkView) -> (u32, u32, bool, bool, String, String, String) {
        (
            self.w,
            self.h,
            v.active,
            self.pressed == Some(Zone::Close) && self.pressed_inside,
            v.title.to_string(),
            v.message.unwrap_or("").to_string(),
            v.status.unwrap_or("").to_string(),
        )
    }

    pub fn render(&mut self, fb: &mut Image, skin: &Skin, v: &MilkView) {
        let (w, h) = (self.w as i32, self.h as i32);
        if fb.width != self.w || fb.height != self.h {
            *fb = Image::new(self.w, self.h, 0xFF00_0000);
        }
        self.text.clear();
        let mut p = Painter::new(fb, skin);
        draw_frame(&mut p, w, h, v.title, v.active, self.pressed == Some(Zone::Close) && self.pressed_inside);
        let vp = self.viewport();
        p.fill(vp.x, vp.y, vp.w, vp.h, 0);
        if let Some(m) = v.message {
            message_box(&mut p, &mut self.text, vp, m, false);
        }
        if let Some(s) = v.status {
            message_box(&mut p, &mut self.text, vp, s, true);
        }
    }
}

/// Un message dans un cadre noir, en haut à gauche (ou en bas) de `area`. Sert aussi au plein écran.
pub fn message_box(p: &mut Painter, out: &mut Vec<TextRun>, area: Rect, msg: &str, bottom: bool) {
    let max = (area.w - 8) as f32;
    let shown = text::ellipsize(msg, FONT, max);
    let tw = text::measure(&shown, FONT).ceil() as i32;
    let (bw, bh) = (tw + 8, 15);
    let bx = area.x + 2;
    let by = if bottom { area.y + area.h - bh - 2 } else { area.y + 2 };
    p.fill(bx, by, bw.min(area.w - 4), bh, BOX);
    out.push(TextRun {
        x: (bx + 4) as f32,
        y: (by + 2) as f32,
        text: shown,
        color: 0xFFE8_E8E8,
        size: FONT,
        clip: Rect::new(bx, by, bw, bh),
        align: Align::Left,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_close_and_fullscreen() {
        let mut m = MilkWindow::default();
        assert_eq!(m.size(), (275, 232));
        assert!(m.mouse_down(270, 225).is_empty());
        assert_eq!(m.mouse_move(300, 260), vec![Action::MilkResize(300, 261)]);
        m.mouse_up(300, 260);
        assert_eq!(m.mouse_down(m.w as i32 - 6, 6), Vec::<Action>::new());
        assert_eq!(m.mouse_up(m.w as i32 - 6, 6), vec![Action::Close]);
        assert_eq!(m.double_click(100, 100), vec![Action::MilkFullscreen]);
        assert_eq!(m.mouse_down(100, 5), vec![Action::StartMove]);
    }

    #[test]
    fn viewport_is_transparent() {
        let mut m = MilkWindow::default();
        let mut fb = Image::new(1, 1, 0);
        let v = MilkView { active: true, title: "Visualisation", message: Some("Alpagamp - Néon"), status: None };
        m.render(&mut fb, &Skin::default_skin(), &v);
        let px = |x: u32, y: u32| fb.pixels[(y * fb.width + x) as usize];
        assert_eq!(px(150, 150) >> 24, 0);
        assert_eq!(px(1, 1) >> 24, 0xFF);
        assert_eq!(px(14, 23), BOX);
        assert_eq!(m.text.len(), 1);
    }
}
