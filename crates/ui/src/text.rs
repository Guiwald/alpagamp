//! Texte en police vectorielle (playlist, fenêtres génériques), dessiné à la résolution de l'écran.
//!
//! Les fenêtres dessinent leurs sprites à l'échelle 1 et décrivent le texte sous forme de [`TextRun`] ; le texte est
//! ensuite rasterisé par-dessus l'image agrandie, pour rester net sur un écran Retina ou en double taille.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

static FONT_DATA: &[u8] = include_bytes!("../../../assets/fonts/LiberationSans-Regular.ttf");

fn font() -> &'static fontdue::Font {
    static F: OnceLock<fontdue::Font> = OnceLock::new();
    F.get_or_init(|| fontdue::Font::from_bytes(FONT_DATA, fontdue::FontSettings::default()).expect("police intégrée"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    /// `x` est le bord droit.
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }
}

/// Un morceau de texte à dessiner (coordonnées en pixels skin).
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub x: f32,
    /// Haut de la ligne.
    pub y: f32,
    pub text: String,
    pub color: u32,
    /// Taille de la police en pixels skin.
    pub size: f32,
    pub clip: Rect,
    pub align: Align,
}

/// Largeur d'avance d'un texte, en pixels skin.
pub fn measure(text: &str, size: f32) -> f32 {
    let f = font();
    text.chars().map(|c| f.metrics(c, size).advance_width).sum()
}

/// Coupe un texte trop long et ajoute « … ».
pub fn ellipsize(text: &str, size: f32, max_w: f32) -> String {
    if measure(text, size) <= max_w {
        return text.to_string();
    }
    let ell = measure("…", size);
    let mut w = 0.0;
    let mut out = String::new();
    for c in text.chars() {
        let a = font().metrics(c, size).advance_width;
        if w + a + ell > max_w {
            break;
        }
        w += a;
        out.push(c);
    }
    out.push('…');
    out
}

type GlyphKey = (char, u32);

thread_local! {
    static CACHE: RefCell<HashMap<GlyphKey, (fontdue::Metrics, Vec<u8>)>> = RefCell::new(HashMap::new());
}

/// Dessine des morceaux de texte dans un tampon 32 bits (`0x??RRGGBB`) de `w`x`h`, agrandi `zoom` fois.
pub fn draw_runs(buf: &mut [u32], w: u32, h: u32, zoom: f32, runs: &[TextRun]) {
    draw_runs_at(buf, w, h, zoom, (0.0, 0.0), runs);
}

/// Comme [`draw_runs`], la fenêtre étant placée en `origin` (pixels physiques) dans le tampon.
pub fn draw_runs_at(buf: &mut [u32], w: u32, h: u32, zoom: f32, origin: (f32, f32), runs: &[TextRun]) {
    let f = font();
    let (ox, oy) = origin;
    for run in runs {
        let px = (run.size * zoom).round().max(4.0);
        let lm = f.horizontal_line_metrics(px);
        let ascent = lm.map(|m| m.ascent).unwrap_or(px * 0.8);
        let width = measure(&run.text, run.size) * zoom;
        let mut pen_x = ox + run.x * zoom;
        if run.align == Align::Right {
            pen_x -= width;
        }
        let baseline = oy + run.y * zoom + ascent;
        let clip_x0 = (ox + (run.clip.x as f32) * zoom).max(0.0) as i32;
        let clip_y0 = (oy + (run.clip.y as f32) * zoom).max(0.0) as i32;
        let clip_x1 = (ox + ((run.clip.x + run.clip.w) as f32) * zoom).min(w as f32) as i32;
        let clip_y1 = (oy + ((run.clip.y + run.clip.h) as f32) * zoom).min(h as f32) as i32;
        let (cr, cg, cb) = ((run.color >> 16) & 0xFF, (run.color >> 8) & 0xFF, run.color & 0xFF);
        for ch in run.text.chars() {
            CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                let (m, bmp) = cache.entry((ch, px as u32)).or_insert_with(|| f.rasterize(ch, px));
                let gx = pen_x.round() as i32 + m.xmin;
                let gy = (baseline - m.height as f32 - m.ymin as f32).round() as i32;
                for row in 0..m.height as i32 {
                    let y = gy + row;
                    if y < clip_y0 || y >= clip_y1 {
                        continue;
                    }
                    for col in 0..m.width as i32 {
                        let x = gx + col;
                        if x < clip_x0 || x >= clip_x1 {
                            continue;
                        }
                        let a = bmp[(row as usize) * m.width + col as usize] as u32;
                        if a == 0 {
                            continue;
                        }
                        let i = (y as u32 * w + x as u32) as usize;
                        let d = buf[i];
                        let (dr, dg, db) = ((d >> 16) & 0xFF, (d >> 8) & 0xFF, d & 0xFF);
                        let mixc = |s: u32, d: u32| (s * a + d * (255 - a)) / 255;
                        buf[i] = (d & 0xFF00_0000) | (mixc(cr, dr) << 16) | (mixc(cg, dg) << 8) | mixc(cb, db);
                    }
                }
                pen_x += m.advance_width;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_and_draws() {
        let w = measure("Alpagamp", 10.0);
        assert!(w > 30.0 && w < 60.0, "{w}");
        assert!(ellipsize("Un titre beaucoup trop long pour la place", 10.0, 50.0).ends_with('…'));
        let mut buf = vec![0u32; 100 * 20];
        draw_runs(&mut buf, 100, 20, 1.0, &[TextRun { x: 2.0, y: 2.0, text: "Hi".into(), color: 0xFFFFFF, size: 12.0, clip: Rect::new(0, 0, 100, 20), align: Align::Left }]);
        assert!(buf.iter().any(|&p| p & 0xFF > 128), "rien de dessiné");
    }
}
