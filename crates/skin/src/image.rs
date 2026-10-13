//! Image RGBA en mémoire, au format 0xAARRGGBB.

/// Une image en pixels 32 bits `0xAARRGGBB`, ligne par ligne, de haut en bas.
#[derive(Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Image({}x{})", self.width, self.height)
    }
}

impl Image {
    /// Image remplie d'une couleur.
    pub fn new(width: u32, height: u32, fill: u32) -> Self {
        Self { width, height, pixels: vec![fill; (width * height) as usize] }
    }

    /// Pixel en (x, y) ; `None` hors de l'image.
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Option<u32> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some(self.pixels[(y as u32 * self.width + x as u32) as usize])
    }

    /// Écrit un pixel (ignoré hors de l'image).
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, c: u32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        self.pixels[(y as u32 * self.width + x as u32) as usize] = c;
    }

    /// Copie le rectangle `(sx, sy, w, h)` de `src` en `(dx, dy)`, en ignorant ce qui déborde.
    /// Les pixels source sont copiés tels quels (pas de mélange alpha).
    pub fn blit(&mut self, src: &Image, sx: i32, sy: i32, w: i32, h: i32, dx: i32, dy: i32) {
        for row in 0..h {
            let syy = sy + row;
            let dyy = dy + row;
            if syy < 0 || syy >= src.height as i32 || dyy < 0 || dyy >= self.height as i32 {
                continue;
            }
            for col in 0..w {
                let sxx = sx + col;
                let dxx = dx + col;
                if sxx < 0 || sxx >= src.width as i32 || dxx < 0 || dxx >= self.width as i32 {
                    continue;
                }
                let p = src.pixels[(syy as u32 * src.width + sxx as u32) as usize];
                self.pixels[(dyy as u32 * self.width + dxx as u32) as usize] = p;
            }
        }
    }

    /// Extrait un sous-rectangle (les zones hors de la source restent transparentes).
    pub fn crop(&self, x: i32, y: i32, w: u32, h: u32) -> Image {
        let mut out = Image::new(w, h, 0);
        out.blit(self, x, y, w as i32, h as i32, 0, 0);
        out
    }

    /// Remplit un rectangle.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        for yy in y.max(0)..(y + h).min(self.height as i32) {
            for xx in x.max(0)..(x + w).min(self.width as i32) {
                self.pixels[(yy as u32 * self.width + xx as u32) as usize] = c;
            }
        }
    }
}

/// Construit une couleur opaque.
#[inline]
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

/// Composantes (r, g, b) d'une couleur.
#[inline]
pub const fn split_rgb(c: u32) -> (u8, u8, u8) {
    ((c >> 16) as u8, (c >> 8) as u8, c as u8)
}
