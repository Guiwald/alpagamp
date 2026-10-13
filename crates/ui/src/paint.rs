//! Dessin à partir des planches du skin.

use alpagamp_skin::sprites::{self, Sheet, Sprite};
use alpagamp_skin::text::{CHAR_H, CHAR_W, glyph};
use alpagamp_skin::{Image, Skin};

pub struct Painter<'a> {
    pub fb: &'a mut Image,
    pub skin: &'a Skin,
}

impl<'a> Painter<'a> {
    pub fn new(fb: &'a mut Image, skin: &'a Skin) -> Self {
        Self { fb, skin }
    }

    /// Copie un sprite en (x, y).
    pub fn sprite(&mut self, s: Sprite, x: i32, y: i32) {
        let src = self.skin.sheet(s.sheet);
        self.fb.blit(src, s.x, s.y, s.w, s.h, x, y);
    }

    /// Copie une partie d'un sprite (largeur `w`, hauteur `h`).
    pub fn sprite_part(&mut self, s: Sprite, x: i32, y: i32, w: i32, h: i32) {
        let src = self.skin.sheet(s.sheet);
        self.fb.blit(src, s.x, s.y, w.min(s.w), h.min(s.h), x, y);
    }

    /// Écrit un texte avec la police `text.bmp`, en coupant au-delà de `max_w` pixels.
    /// `offset` décale le texte vers la gauche (défilement).
    pub fn text(&mut self, x: i32, y: i32, text: &str, max_w: i32, offset: i32) {
        let src = self.skin.sheet(Sheet::Text);
        let mut cx = x - offset;
        for ch in text.chars() {
            if cx >= x + max_w {
                break;
            }
            if cx + CHAR_W > x {
                let (col, row) = glyph(ch);
                // Découpe à gauche et à droite.
                let left_cut = (x - cx).max(0);
                let right_cut = ((cx + CHAR_W) - (x + max_w)).max(0);
                let w = CHAR_W - left_cut - right_cut;
                if w > 0 {
                    self.fb.blit(src, col * CHAR_W + left_cut, row * CHAR_H, w, CHAR_H, cx + left_cut, y);
                }
            }
            cx += CHAR_W;
        }
    }

    /// Chiffre 9x13 de la planche des chiffres du skin.
    pub fn digit(&mut self, d: u8, x: i32, y: i32) {
        let sheet = self.skin.digits_sheet();
        self.sprite(sprites::digit(sheet, d as i32), x, y);
    }

    /// Remplit un rectangle d'une couleur.
    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        self.fb.fill_rect(x, y, w, h, c);
    }

    pub fn px(&mut self, x: i32, y: i32, c: u32) {
        self.fb.set(x, y, c);
    }
}

/// Largeur en pixels d'un texte en police `text.bmp`.
pub fn text_width(text: &str) -> i32 {
    text.chars().count() as i32 * CHAR_W
}
