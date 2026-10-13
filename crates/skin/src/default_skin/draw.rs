//! Primitives de dessin pour le skin par défaut.

use crate::image::{Image, rgb, split_rgb};

/// Mélange linéaire de deux couleurs (t = 0 → a, t = 1 → b).
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let (ar, ag, ab) = split_rgb(a);
    let (br, bg, bb) = split_rgb(b);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)).round() as u8;
    rgb(l(ar, br), l(ag, bg), l(ab, bb))
}

/// Éclaircit (k > 0) ou assombrit (k < 0) une couleur de k unités sur chaque composante.
pub fn shade(c: u32, k: i32) -> u32 {
    let (r, g, b) = split_rgb(c);
    let f = |x: u8| (x as i32 + k).clamp(0, 255) as u8;
    rgb(f(r), f(g), f(b))
}

/// Bruit déterministe (pour le métal brossé), dans [-1, 1].
pub fn noise(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263) ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0xFFFF) as f32 / 32767.5 - 1.0
}

pub fn px(img: &mut Image, x: i32, y: i32, c: u32) {
    img.set(x, y, c);
}

pub fn rect(img: &mut Image, x: i32, y: i32, w: i32, h: i32, c: u32) {
    img.fill_rect(x, y, w, h, c);
}

pub fn hline(img: &mut Image, x: i32, y: i32, w: i32, c: u32) {
    img.fill_rect(x, y, w, 1, c);
}

pub fn vline(img: &mut Image, x: i32, y: i32, h: i32, c: u32) {
    img.fill_rect(x, y, 1, h, c);
}

/// Contour d'un rectangle.
pub fn frame(img: &mut Image, x: i32, y: i32, w: i32, h: i32, c: u32) {
    hline(img, x, y, w, c);
    hline(img, x, y + h - 1, w, c);
    vline(img, x, y, h, c);
    vline(img, x + w - 1, y, h, c);
}

/// Biseau : haut et gauche en `hi`, bas et droite en `lo`.
pub fn bevel(img: &mut Image, x: i32, y: i32, w: i32, h: i32, hi: u32, lo: u32) {
    hline(img, x, y, w - 1, hi);
    vline(img, x, y, h - 1, hi);
    hline(img, x + 1, y + h - 1, w - 1, lo);
    vline(img, x + w - 1, y + 1, h - 1, lo);
}

/// Dégradé vertical.
pub fn vgrad(img: &mut Image, x: i32, y: i32, w: i32, h: i32, top: u32, bottom: u32) {
    for row in 0..h {
        let t = if h > 1 { row as f32 / (h - 1) as f32 } else { 0.0 };
        hline(img, x, y + row, w, mix(top, bottom, t));
    }
}



/// Métal brossé : dégradé vertical + stries horizontales discrètes.
pub fn brushed(img: &mut Image, x: i32, y: i32, w: i32, h: i32, top: u32, bottom: u32, seed: u32, amount: f32) {
    for row in 0..h {
        let t = if h > 1 { row as f32 / (h - 1) as f32 } else { 0.0 };
        let base = mix(top, bottom, t);
        let row_n = noise(0, y + row, seed) * amount * 0.6;
        for col in 0..w {
            // Stries : bruit qui varie lentement en x.
            let streak = noise((x + col) / 7, y + row, seed ^ 0x9E37) * amount * 0.5;
            let grain = noise(x + col, y + row, seed ^ 0x51ED) * amount * 0.35;
            px(img, x + col, y + row, shade(base, (row_n + streak + grain).round() as i32));
        }
    }
}

/// Dessine un motif ASCII : chaque caractère présent dans `palette` est peint de sa couleur,
/// les autres (ex. '.') sont transparents.
pub fn pattern(img: &mut Image, x: i32, y: i32, rows: &[&str], palette: &[(char, u32)]) {
    for (r, line) in rows.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            if let Some((_, color)) = palette.iter().find(|(k, _)| *k == ch) {
                px(img, x + c as i32, y + r as i32, *color);
            }
        }
    }
}

/// Peint un glyphe (lignes de '#' et '.') d'une seule couleur.
pub fn glyph(img: &mut Image, x: i32, y: i32, rows: &[&str], c: u32) {
    pattern(img, x, y, rows, &[('#', c)]);
}

/// Panneau creusé (écran LCD) : bord sombre en haut à gauche, clair en bas à droite.
pub fn recess(img: &mut Image, x: i32, y: i32, w: i32, h: i32, face: u32, edge_dark: u32, edge_light: u32) {
    rect(img, x, y, w, h, face);
    bevel(img, x, y, w, h, edge_dark, edge_light);
}

/// Touche en relief.
pub fn key(img: &mut Image, x: i32, y: i32, w: i32, h: i32, face_top: u32, face_bottom: u32, hi: u32, lo: u32, outline: u32) {
    vgrad(img, x, y, w, h, face_top, face_bottom);
    frame(img, x, y, w, h, outline);
    bevel(img, x + 1, y + 1, w - 2, h - 2, hi, lo);
}
