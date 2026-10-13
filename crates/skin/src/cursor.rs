//! Curseurs des skins (`*.cur`, format ICO de Windows) : image et point chaud.
//!
//! Un `.cur` contient une ou plusieurs images (DIB « BMP sans en-tête de fichier », ou PNG) ; on prend la plus
//! grande. Les DIB ont deux masques : la couleur (« XOR ») et la transparence (« AND », 1 bit par pixel).

/// Un curseur prêt à afficher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub width: u32,
    pub height: u32,
    /// Point chaud (le « bout » du curseur).
    pub hotspot: (u32, u32),
    /// Pixels `0xAARRGGBB`.
    pub pixels: Vec<u32>,
}

impl Cursor {
    /// Pixels RGBA (pour le système de fenêtres).
    pub fn rgba(&self) -> Vec<u8> {
        self.pixels.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, (p >> 24) as u8]).collect()
    }
}

fn u16le(b: &[u8], o: usize) -> Option<u32> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?) as u32)
}

fn u32le(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

/// Lit un fichier `.cur` (ou `.ico`).
pub fn parse_cur(b: &[u8]) -> Option<Cursor> {
    if u16le(b, 0)? != 0 || !matches!(u16le(b, 2)?, 1 | 2) {
        return None;
    }
    let count = u16le(b, 4)? as usize;
    let is_cur = u16le(b, 2)? == 2;
    // L'entrée la plus grande.
    let mut best: Option<(u32, usize)> = None;
    for i in 0..count.min(32) {
        let e = 6 + i * 16;
        let w = match *b.get(e)? {
            0 => 256,
            v => v as u32,
        };
        if best.is_none_or(|(bw, _)| w > bw) {
            best = Some((w, e));
        }
    }
    let (_, e) = best?;
    let (hx, hy) = if is_cur { (u16le(b, e + 4)?, u16le(b, e + 6)?) } else { (0, 0) };
    let size = u32le(b, e + 8)? as usize;
    let offset = u32le(b, e + 12)? as usize;
    let data = b.get(offset..offset.checked_add(size)?.min(b.len()))?;
    let mut c = if data.starts_with(b"\x89PNG") { None } else { parse_dib(data)? };
    if let Some(cur) = c.as_mut() {
        cur.hotspot = (hx.min(cur.width.saturating_sub(1)), hy.min(cur.height.saturating_sub(1)));
    }
    // Les curseurs PNG (rares dans les skins) sont ignorés : le curseur du système reste affiché.
    c.take()
}

fn parse_dib(d: &[u8]) -> Option<Option<Cursor>> {
    let header = u32le(d, 0)? as usize;
    if header < 40 {
        return None;
    }
    let w = u32le(d, 4)? as i32;
    let h2 = u32le(d, 8)? as i32;
    let bpp = u16le(d, 14)?;
    let compression = u32le(d, 16)?;
    if w <= 0 || w > 256 || h2 == 0 || compression != 0 {
        return None;
    }
    // La hauteur compte les deux masques.
    let h = (h2.unsigned_abs() / 2) as i32;
    let colors = match u32le(d, 32)? {
        0 if bpp <= 8 => 1u32 << bpp,
        n => n,
    } as usize;
    let palette_at = header;
    let pixels_at = palette_at + if bpp <= 8 { colors * 4 } else { 0 };
    let palette = |i: usize| -> u32 {
        let o = palette_at + i * 4;
        match d.get(o..o + 3) {
            Some(p) => 0xFF00_0000 | (p[2] as u32) << 16 | (p[1] as u32) << 8 | p[0] as u32,
            None => 0xFF00_0000,
        }
    };
    let row = |bits: u32| (((w as u32 * bits) + 31) / 32 * 4) as usize;
    let xor_row = row(bpp as u32);
    let and_at = pixels_at + xor_row * h as usize;
    let and_row = row(1);
    let mut px = vec![0u32; (w * h) as usize];
    let mut has_alpha = false;
    for y in 0..h as usize {
        // Lignes de bas en haut.
        let dy = h as usize - 1 - y;
        let base = pixels_at + y * xor_row;
        for x in 0..w as usize {
            let c = match bpp {
                32 => {
                    let p = d.get(base + x * 4..base + x * 4 + 4)?;
                    if p[3] != 0 {
                        has_alpha = true;
                    }
                    (p[3] as u32) << 24 | (p[2] as u32) << 16 | (p[1] as u32) << 8 | p[0] as u32
                }
                24 => {
                    let p = d.get(base + x * 3..base + x * 3 + 3)?;
                    0xFF00_0000 | (p[2] as u32) << 16 | (p[1] as u32) << 8 | p[0] as u32
                }
                8 => palette(*d.get(base + x)? as usize),
                4 => {
                    let v = *d.get(base + x / 2)?;
                    palette(if x % 2 == 0 { v >> 4 } else { v & 0x0F } as usize)
                }
                1 => {
                    let v = *d.get(base + x / 8)?;
                    palette(((v >> (7 - x % 8)) & 1) as usize)
                }
                _ => return None,
            };
            px[dy * w as usize + x] = c;
        }
    }
    // Masque de transparence (sauf images 32 bits qui ont leur propre alpha).
    if !has_alpha {
        for y in 0..h as usize {
            let dy = h as usize - 1 - y;
            let base = and_at + y * and_row;
            for x in 0..w as usize {
                let transparent = d.get(base + x / 8).is_some_and(|v| (v >> (7 - x % 8)) & 1 == 1);
                let p = &mut px[dy * w as usize + x];
                if transparent {
                    // Masque 1 et couleur non noire : « inverser l'écran ». On montre un trait sombre à la place.
                    *p = if *p & 0x00FF_FFFF != 0 { 0xC000_0000 } else { 0 };
                } else {
                    *p |= 0xFF00_0000;
                }
            }
        }
    }
    Some(Some(Cursor { width: w as u32, height: h as u32, hotspot: (0, 0), pixels: px }))
}

/// Curseurs que les skins classiques peuvent fournir (nom du fichier, en minuscules).
pub const CURSOR_FILES: &[&str] = &[
    "normal.cur", "close.cur", "mainmenu.cur", "min.cur", "titlebar.cur", "posbar.cur", "volbal.cur", "winbut.cur",
    "songname.cur", "eqnormal.cur", "eqclose.cur", "eqslid.cur", "eqtitle.cur", "pnormal.cur", "pclose.cur",
    "psize.cur", "ptbar.cur", "pvscroll.cur", "pwinbut.cur", "pwsnorm.cur", "pwssize.cur", "wsclose.cur",
    "wsmin.cur", "wsnormal.cur", "wsposbar.cur", "wswinbut.cur",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Un `.cur` 4×4 en 1 bit : pixel (0,0) blanc opaque, le reste transparent ; point chaud (1, 2).
    fn tiny_cur() -> Vec<u8> {
        let (w, h) = (4u32, 4u32);
        let mut dib = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&(w as i32).to_le_bytes());
        dib.extend_from_slice(&((h * 2) as i32).to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&[0u8; 24]);
        // Palette : noir, blanc.
        dib.extend_from_slice(&[0, 0, 0, 0, 255, 255, 255, 0]);
        // XOR (de bas en haut) : seule la ligne du haut (dernière) a le pixel 0 à 1.
        for y in 0..h {
            let v: u8 = if y == h - 1 { 0b1000_0000 } else { 0 };
            dib.extend_from_slice(&[v, 0, 0, 0]);
        }
        // AND : tout transparent sauf le pixel (0,0).
        for y in 0..h {
            let v: u8 = if y == h - 1 { 0b0111_1111 } else { 0xFF };
            dib.extend_from_slice(&[v, 0xFF, 0xFF, 0xFF]);
        }
        let mut f = Vec::new();
        f.extend_from_slice(&[0, 0, 2, 0, 1, 0]);
        f.extend_from_slice(&[w as u8, h as u8, 2, 0]);
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&2u16.to_le_bytes());
        f.extend_from_slice(&(dib.len() as u32).to_le_bytes());
        f.extend_from_slice(&22u32.to_le_bytes());
        f.extend_from_slice(&dib);
        f
    }

    #[test]
    fn parses_a_1bit_cursor() {
        let c = parse_cur(&tiny_cur()).unwrap();
        assert_eq!((c.width, c.height, c.hotspot), (4, 4, (1, 2)));
        assert_eq!(c.pixels[0], 0xFFFF_FFFF);
        assert_eq!(c.pixels[1] >> 24, 0);
        assert_eq!(c.rgba().len(), 64);
        assert!(parse_cur(b"pas un curseur").is_none());
    }
}

#[cfg(test)]
mod real_tests {
    /// Curseurs d'un vrai skin : `ALPAGAMP_TEST_WSZ=chemin.wsz cargo test -p alpagamp-skin real_cursors -- --ignored`.
    #[test]
    #[ignore]
    fn real_cursors() {
        let path = std::env::var("ALPAGAMP_TEST_WSZ").expect("ALPAGAMP_TEST_WSZ");
        for p in path.split(':') {
            let files = crate::read_zip(&std::fs::read(p).unwrap()).unwrap();
            let curs: Vec<_> = files.keys().filter(|k| k.ends_with(".cur")).cloned().collect();
            let ok: Vec<_> = curs.iter().filter(|k| super::parse_cur(&files[*k]).is_some()).collect();
            eprintln!("{p} : {} curseurs, {} lus", curs.len(), ok.len());
            for k in &curs {
                if let Some(c) = super::parse_cur(&files[k]) {
                    let opaque = c.pixels.iter().filter(|p| *p >> 24 == 0xFF).count();
                    eprintln!("  {k} {}x{} point chaud {:?}, {opaque} pixels opaques", c.width, c.height, c.hotspot);
                } else {
                    eprintln!("  {k} illisible");
                }
            }
        }
    }
}
