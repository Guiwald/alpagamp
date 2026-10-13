//! Décodeur BMP tolérant, écrit pour les skins classiques.
//!
//! Les skins d'époque sont surtout en 8 bits, souvent compressés RLE8, parfois mal formés (taille de
//! fichier fausse, données tronquées, décalage des pixels à zéro). Winamp les chargeait via l'API
//! Windows, assez permissive : on fait de même et on remplit ce qui manque en noir plutôt que d'échouer.
//!
//! Gère : en-têtes CORE (12 octets), INFO (40), V2/V3 (52/56), V4 (108), V5 (124) ; 1, 4, 8, 16, 24 et
//! 32 bits ; non compressé, RLE4, RLE8, BITFIELDS ; images de haut en bas ou de bas en haut.
//! L'alpha est ignoré (comme GDI) : toutes les images décodées sont opaques.

use crate::image::{Image, rgb};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BmpError {
    #[error("pas un fichier BMP (signature absente)")]
    NotBmp,
    #[error("en-tête BMP tronqué")]
    Truncated,
    #[error("en-tête BMP non pris en charge ({0} octets)")]
    UnsupportedHeader(u32),
    #[error("profondeur de couleur non prise en charge ({0} bits)")]
    UnsupportedDepth(u16),
    #[error("compression non prise en charge ({0})")]
    UnsupportedCompression(u32),
    #[error("dimensions invalides ({0}x{1})")]
    BadDimensions(i64, i64),
}

const BI_RGB: u32 = 0;
const BI_RLE8: u32 = 1;
const BI_RLE4: u32 = 2;
const BI_BITFIELDS: u32 = 3;
const BI_ALPHABITFIELDS: u32 = 6;

/// Taille maximale acceptée (garde-fou contre les fichiers corrompus).
const MAX_DIM: i64 = 8192;

fn u16le(d: &[u8], o: usize) -> Option<u16> {
    d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
}
fn u32le(d: &[u8], o: usize) -> Option<u32> {
    d.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn i32le(d: &[u8], o: usize) -> Option<i32> {
    u32le(d, o).map(|v| v as i32)
}

/// Un masque de bits et sa conversion vers 8 bits.
#[derive(Clone, Copy)]
struct Mask {
    mask: u32,
    shift: u32,
    bits: u32,
}

impl Mask {
    fn new(mask: u32) -> Self {
        if mask == 0 {
            return Mask { mask: 0, shift: 0, bits: 0 };
        }
        let shift = mask.trailing_zeros();
        let bits = (mask >> shift).count_ones();
        Mask { mask, shift, bits }
    }
    fn extract(&self, v: u32) -> u8 {
        if self.bits == 0 {
            return 0;
        }
        let x = (v & self.mask) >> self.shift;
        if self.bits >= 8 {
            (x >> (self.bits - 8)) as u8
        } else {
            // Étale la valeur sur 8 bits (ex. 5 bits : 31 → 255).
            let max = (1u32 << self.bits) - 1;
            ((x * 255 + max / 2) / max) as u8
        }
    }
}

/// Décode un fichier BMP complet.
pub fn decode(data: &[u8]) -> Result<Image, BmpError> {
    if data.len() < 18 || &data[0..2] != b"BM" {
        return Err(BmpError::NotBmp);
    }
    let declared_offset = u32le(data, 10).ok_or(BmpError::Truncated)? as usize;
    let hsize = u32le(data, 14).ok_or(BmpError::Truncated)?;
    let h = 14usize;

    let (width, height, bitcount, compression, colors_used, palette_entry_size);
    let mut masks: Option<(u32, u32, u32)> = None;
    let header_end;

    match hsize {
        12 => {
            width = u16le(data, h + 4).ok_or(BmpError::Truncated)? as i64;
            height = u16le(data, h + 6).ok_or(BmpError::Truncated)? as i16 as i64;
            bitcount = u16le(data, h + 10).ok_or(BmpError::Truncated)?;
            compression = BI_RGB;
            colors_used = 0;
            palette_entry_size = 3;
            header_end = h + 12;
        }
        40 | 52 | 56 | 64 | 108 | 124 => {
            width = i32le(data, h + 4).ok_or(BmpError::Truncated)? as i64;
            height = i32le(data, h + 8).ok_or(BmpError::Truncated)? as i64;
            bitcount = u16le(data, h + 14).ok_or(BmpError::Truncated)?;
            compression = u32le(data, h + 16).ok_or(BmpError::Truncated)?;
            colors_used = u32le(data, h + 32).unwrap_or(0);
            palette_entry_size = 4;
            let mut end = h + hsize as usize;
            if compression == BI_BITFIELDS || compression == BI_ALPHABITFIELDS {
                if hsize >= 52 {
                    masks = Some((
                        u32le(data, h + 40).unwrap_or(0),
                        u32le(data, h + 44).unwrap_or(0),
                        u32le(data, h + 48).unwrap_or(0),
                    ));
                } else {
                    // En-tête INFO : les masques suivent l'en-tête.
                    masks = Some((
                        u32le(data, end).unwrap_or(0),
                        u32le(data, end + 4).unwrap_or(0),
                        u32le(data, end + 8).unwrap_or(0),
                    ));
                    end += if compression == BI_ALPHABITFIELDS { 16 } else { 12 };
                }
            }
            header_end = end;
        }
        other => return Err(BmpError::UnsupportedHeader(other)),
    }

    if width <= 0 || height == 0 || width > MAX_DIM || height.abs() > MAX_DIM {
        return Err(BmpError::BadDimensions(width, height));
    }
    let top_down = height < 0;
    let width = width as u32;
    let height = height.unsigned_abs() as u32;

    // Palette.
    let mut palette: Vec<u32> = Vec::new();
    if bitcount <= 8 {
        let max = 1usize << bitcount;
        let n = if colors_used == 0 || colors_used as usize > max { max } else { colors_used as usize };
        for i in 0..n {
            let o = header_end + i * palette_entry_size;
            match data.get(o..o + 3) {
                Some(b) => palette.push(rgb(b[2], b[1], b[0])),
                None => palette.push(rgb(0, 0, 0)),
            }
        }
        while palette.len() < max {
            palette.push(rgb(0, 0, 0));
        }
    }

    // Début des pixels : celui déclaré s'il est plausible, sinon juste après la palette.
    let computed_offset = header_end + palette.len() * palette_entry_size;
    let offset = if declared_offset >= header_end && declared_offset < data.len() {
        declared_offset
    } else {
        computed_offset.min(data.len())
    };
    let pixels = &data[offset..];

    let mut img = Image::new(width, height, rgb(0, 0, 0));
    let row_index = |r: u32| if top_down { r } else { height - 1 - r };

    match (compression, bitcount) {
        (BI_RLE8, 8) => decode_rle(pixels, &palette, &mut img, top_down, false),
        (BI_RLE4, 4) => decode_rle(pixels, &palette, &mut img, top_down, true),
        (BI_RGB, 1 | 4 | 8) => {
            let stride = ((width as usize * bitcount as usize + 31) / 32) * 4;
            for r in 0..height {
                let y = row_index(r);
                let row = &pixels[(r as usize * stride).min(pixels.len())..];
                for x in 0..width as usize {
                    let idx = match bitcount {
                        8 => row.get(x).copied().unwrap_or(0) as usize,
                        4 => {
                            let b = row.get(x / 2).copied().unwrap_or(0);
                            (if x % 2 == 0 { b >> 4 } else { b & 0x0F }) as usize
                        }
                        _ => {
                            let b = row.get(x / 8).copied().unwrap_or(0);
                            ((b >> (7 - (x % 8))) & 1) as usize
                        }
                    };
                    img.set(x as i32, y as i32, palette.get(idx).copied().unwrap_or(rgb(0, 0, 0)));
                }
            }
        }
        (BI_RGB | BI_BITFIELDS | BI_ALPHABITFIELDS, 16 | 24 | 32) => {
            let bpp = bitcount as usize / 8;
            let stride = ((width as usize * bitcount as usize + 31) / 32) * 4;
            let (mr, mg, mb) = match (masks, bitcount) {
                (Some(m), 16 | 32) => (Mask::new(m.0), Mask::new(m.1), Mask::new(m.2)),
                (_, 16) => (Mask::new(0x7C00), Mask::new(0x03E0), Mask::new(0x001F)),
                _ => (Mask::new(0x00FF_0000), Mask::new(0x0000_FF00), Mask::new(0x0000_00FF)),
            };
            for r in 0..height {
                let y = row_index(r);
                let base = r as usize * stride;
                for x in 0..width as usize {
                    let o = base + x * bpp;
                    let c = match bitcount {
                        24 => match pixels.get(o..o + 3) {
                            Some(b) => rgb(b[2], b[1], b[0]),
                            None => rgb(0, 0, 0),
                        },
                        16 => {
                            let v = pixels.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as u32).unwrap_or(0);
                            rgb(mr.extract(v), mg.extract(v), mb.extract(v))
                        }
                        _ => {
                            let v = pixels.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).unwrap_or(0);
                            rgb(mr.extract(v), mg.extract(v), mb.extract(v))
                        }
                    };
                    img.set(x as i32, y as i32, c);
                }
            }
        }
        (BI_RGB | BI_RLE8 | BI_RLE4 | BI_BITFIELDS | BI_ALPHABITFIELDS, d) => {
            return Err(BmpError::UnsupportedDepth(d));
        }
        (c, _) => return Err(BmpError::UnsupportedCompression(c)),
    }
    Ok(img)
}

/// Décompression RLE8 / RLE4. Les zones non couvertes gardent l'index 0 de la palette.
fn decode_rle(data: &[u8], palette: &[u32], img: &mut Image, top_down: bool, four_bits: bool) {
    let width = img.width as i32;
    let height = img.height as i32;
    let bg = palette.first().copied().unwrap_or(rgb(0, 0, 0));
    img.pixels.fill(bg);
    let color = |i: u8| palette.get(i as usize).copied().unwrap_or(rgb(0, 0, 0));
    let mut x: i32 = 0;
    let mut r: i32 = 0; // ligne dans l'ordre du fichier
    let put = |img: &mut Image, x: i32, r: i32, c: u32| {
        if x < width && r < height {
            let y = if top_down { r } else { height - 1 - r };
            img.set(x, y, c);
        }
    };
    let mut i = 0usize;
    while i + 1 < data.len() {
        let count = data[i];
        let value = data[i + 1];
        i += 2;
        if count > 0 {
            for k in 0..count as i32 {
                let idx = if four_bits {
                    if k % 2 == 0 { value >> 4 } else { value & 0x0F }
                } else {
                    value
                };
                put(img, x, r, color(idx));
                x += 1;
            }
            continue;
        }
        match value {
            0 => {
                x = 0;
                r += 1;
            }
            1 => return,
            2 => {
                let (dx, dy) = match data.get(i..i + 2) {
                    Some(b) => (b[0] as i32, b[1] as i32),
                    None => return,
                };
                i += 2;
                x += dx;
                r += dy;
            }
            n => {
                let n = n as usize;
                if four_bits {
                    let bytes = n.div_ceil(2);
                    for k in 0..n {
                        let b = data.get(i + k / 2).copied().unwrap_or(0);
                        let idx = if k % 2 == 0 { b >> 4 } else { b & 0x0F };
                        put(img, x, r, color(idx));
                        x += 1;
                    }
                    i += bytes + (bytes & 1);
                } else {
                    for k in 0..n {
                        let b = data.get(i + k).copied().unwrap_or(0);
                        put(img, x, r, color(b));
                        x += 1;
                    }
                    i += n + (n & 1);
                }
            }
        }
        if r >= height {
            return;
        }
    }
}

/// Encode une image en BMP 24 bits non compressé (pour exporter le skin par défaut en .wsz).
pub fn encode_24(img: &Image) -> Vec<u8> {
    let stride = ((img.width as usize * 3 + 3) / 4) * 4;
    let data_size = stride * img.height as usize;
    let offset = 14 + 40;
    let mut out = Vec::with_capacity(offset + data_size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((offset + data_size) as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(offset as u32).to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(img.width as i32).to_le_bytes());
    out.extend_from_slice(&(img.height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&BI_RGB.to_le_bytes());
    out.extend_from_slice(&(data_size as u32).to_le_bytes());
    out.extend_from_slice(&2835i32.to_le_bytes());
    out.extend_from_slice(&2835i32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for r in (0..img.height).rev() {
        let start = out.len();
        for x in 0..img.width {
            let p = img.pixels[(r * img.width + x) as usize];
            out.extend_from_slice(&[p as u8, (p >> 8) as u8, (p >> 16) as u8]);
        }
        while out.len() - start < stride {
            out.push(0);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(width: i32, height: i32, bitcount: u16, compression: u32, palette: &[[u8; 4]], data: &[u8]) -> Vec<u8> {
        let offset = 14 + 40 + palette.len() * 4;
        let mut v = Vec::new();
        v.extend_from_slice(b"BM");
        v.extend_from_slice(&((offset + data.len()) as u32).to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&(offset as u32).to_le_bytes());
        v.extend_from_slice(&40u32.to_le_bytes());
        v.extend_from_slice(&width.to_le_bytes());
        v.extend_from_slice(&height.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&bitcount.to_le_bytes());
        v.extend_from_slice(&compression.to_le_bytes());
        v.extend_from_slice(&(data.len() as u32).to_le_bytes());
        v.extend_from_slice(&[0; 8]);
        v.extend_from_slice(&(palette.len() as u32).to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        for p in palette {
            v.extend_from_slice(p);
        }
        v.extend_from_slice(data);
        v
    }

    const PAL: [[u8; 4]; 3] = [[0, 0, 0, 0], [0, 0, 255, 0], [255, 0, 0, 0]]; // noir, rouge, bleu (BGR)

    #[test]
    fn rle8_bottom_up() {
        // 4x2 : ligne du bas = 4 rouges ; ligne du haut = bleu, absolu [1,2], bleu.
        let data = [4, 1, 0, 0, 1, 2, 0, 3, 1, 2, 2, 0, 0, 1];
        // ligne 0 (bas) : 4x index1 ; fin de ligne ; ligne 1 : 1x index2, absolu 3 : [1,2,2] (+padding), fin
        let bmp = header(4, 2, 8, BI_RLE8, &PAL, &data);
        let img = decode(&bmp).unwrap();
        let red = rgb(255, 0, 0);
        let blue = rgb(0, 0, 255);
        assert_eq!(&img.pixels[4..8], &[red, red, red, red]);
        assert_eq!(&img.pixels[0..4], &[blue, red, blue, blue]);
    }

    #[test]
    fn eight_bit_top_down_with_padding() {
        // 3x2 haut-en-bas, lignes alignées sur 4 octets.
        let data = [1, 2, 0, 0, 2, 1, 0, 0];
        let bmp = header(3, -2, 8, BI_RGB, &PAL, &data);
        let img = decode(&bmp).unwrap();
        assert_eq!(img.get(0, 0), Some(rgb(255, 0, 0)));
        assert_eq!(img.get(1, 0), Some(rgb(0, 0, 255)));
        assert_eq!(img.get(0, 1), Some(rgb(0, 0, 255)));
        assert_eq!(img.get(2, 1), Some(rgb(0, 0, 0)));
    }

    #[test]
    fn twenty_four_bit_roundtrip() {
        let mut img = Image::new(5, 3, rgb(10, 20, 30));
        img.set(4, 2, rgb(200, 100, 50));
        img.set(0, 0, rgb(1, 2, 3));
        let decoded = decode(&encode_24(&img)).unwrap();
        assert_eq!(decoded, img);
    }

    #[test]
    fn truncated_pixels_are_black() {
        let bmp = header(4, 4, 8, BI_RGB, &PAL, &[1, 1]);
        let img = decode(&bmp).unwrap();
        assert_eq!(img.get(0, 3), Some(rgb(255, 0, 0)));
        assert_eq!(img.get(3, 0), Some(rgb(0, 0, 0)));
    }

    #[test]
    fn rejects_non_bmp() {
        assert_eq!(decode(b"PNG...............").unwrap_err(), BmpError::NotBmp);
    }
}
