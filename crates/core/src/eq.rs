//! Réglages d'égaliseur, presets et fichiers `.eqf` / bibliothèques.
//!
//! Chaque curseur est stocké comme dans Winamp, sur 0..=63 : 0 = +12 dB, 31 = 0 dB, 63 = −12 dB.

use serde::{Deserialize, Serialize};

/// Fréquences affichées sur les curseurs (libellés classiques).
pub const BAND_LABELS: [&str; 10] = ["60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K"];
/// Fréquences centrales réelles des filtres, en Hz.
pub const BAND_FREQS: [f32; 10] = [60.0, 170.0, 310.0, 600.0, 1000.0, 3000.0, 6000.0, 12000.0, 14000.0, 16000.0];

pub const CENTER: u8 = 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EqSettings {
    pub enabled: bool,
    /// Charger automatiquement un preset propre à chaque morceau.
    pub auto: bool,
    pub preamp: u8,
    pub bands: [u8; 10],
}

impl Default for EqSettings {
    fn default() -> Self {
        Self { enabled: false, auto: false, preamp: CENTER, bands: [CENTER; 10] }
    }
}

impl EqSettings {
    /// Le filtre peut-il être court-circuité (tout à 0 dB) ?
    pub fn is_flat(&self) -> bool {
        self.preamp == CENTER && self.bands.iter().all(|&b| b == CENTER)
    }

    pub fn apply_preset(&mut self, p: &EqPreset) {
        self.preamp = p.preamp;
        self.bands = p.bands;
    }
}

/// Valeur de curseur (0..63) → gain en dB.
pub fn slider_to_db(v: u8) -> f32 {
    let v = v.min(63) as f32;
    if v <= 31.0 { 12.0 * (31.0 - v) / 31.0 } else { -12.0 * (v - 31.0) / 32.0 }
}

/// Gain en dB → valeur de curseur (0..63).
pub fn db_to_slider(db: f32) -> u8 {
    let db = db.clamp(-12.0, 12.0);
    let v = if db >= 0.0 { 31.0 - db * 31.0 / 12.0 } else { 31.0 - db * 32.0 / 12.0 };
    v.round().clamp(0.0, 63.0) as u8
}

/// Un preset nommé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EqPreset {
    pub name: String,
    pub preamp: u8,
    pub bands: [u8; 10],
}

impl EqPreset {
    pub fn from_db(name: &str, preamp_db: f32, bands_db: [f32; 10]) -> Self {
        Self { name: name.to_string(), preamp: db_to_slider(preamp_db), bands: bands_db.map(db_to_slider) }
    }
}

/// Presets d'usine d'Alpagamp (courbes originales ; noms usuels).
pub fn factory_presets() -> Vec<EqPreset> {
    let p = EqPreset::from_db;
    vec![
        p("Flat", 0.0, [0.0; 10]),
        p("Classical", 0.0, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -4.0, -4.0, -4.0, -6.0]),
        p("Club", 0.0, [0.0, 0.0, 3.0, 4.0, 4.0, 4.0, 2.0, 0.0, 0.0, 0.0]),
        p("Dance", 0.0, [6.0, 4.0, 2.0, 0.0, 0.0, -2.0, -3.0, -3.0, 0.0, 0.0]),
        p("Full Bass", -2.0, [6.0, 6.0, 6.0, 4.0, 1.0, -2.0, -4.0, -6.0, -6.0, -7.0]),
        p("Full Bass & Treble", -2.0, [5.0, 4.0, 0.0, -4.0, -3.0, 1.0, 5.0, 7.0, 8.0, 8.0]),
        p("Full Treble", -3.0, [-6.0, -6.0, -6.0, -2.0, 1.0, 6.0, 9.0, 9.0, 9.0, 10.0]),
        p("Headphones", -2.0, [3.0, 7.0, 3.0, -2.0, -1.0, 1.0, 3.0, 6.0, 8.0, 9.0]),
        p("Large Hall", 0.0, [6.0, 6.0, 3.0, 3.0, 0.0, -3.0, -3.0, -3.0, 0.0, 0.0]),
        p("Live", 0.0, [-3.0, 0.0, 2.0, 3.0, 3.0, 3.0, 2.0, 1.0, 1.0, 1.0]),
        p("Party", 0.0, [4.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 4.0, 4.0]),
        p("Pop", 0.0, [-1.0, 3.0, 4.0, 5.0, 3.0, 0.0, -1.0, -1.0, -1.0, -1.0]),
        p("Reggae", 0.0, [0.0, 0.0, 0.0, -3.0, 0.0, 4.0, 4.0, 0.0, 0.0, 0.0]),
        p("Rock", -1.0, [5.0, 3.0, -3.0, -5.0, -2.0, 2.0, 5.0, 6.0, 6.0, 6.0]),
        p("Ska", 0.0, [-1.0, -3.0, -2.0, 0.0, 2.0, 3.0, 5.0, 5.0, 6.0, 5.0]),
        p("Soft", 0.0, [3.0, 1.0, 0.0, -1.0, 0.0, 2.0, 5.0, 6.0, 7.0, 8.0]),
        p("Soft Rock", 0.0, [2.0, 2.0, 1.0, 0.0, -2.0, -3.0, -1.0, 0.0, 1.0, 5.0]),
        p("Techno", -1.0, [5.0, 3.0, 0.0, -3.0, -3.0, 0.0, 5.0, 6.0, 6.0, 5.0]),
    ]
}

/// En-tête des fichiers `.eqf` et des bibliothèques (`.q1`).
const EQF_HEADER: &[u8] = b"Winamp EQ library file v1.1\x1a!--";
const NAME_LEN: usize = 257;
const RECORD_LEN: usize = NAME_LEN + 11;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EqfError {
    #[error("en-tête de fichier EQ absent")]
    BadHeader,
}

/// Lit un `.eqf` ou une bibliothèque de presets.
pub fn read_eqf(data: &[u8]) -> Result<Vec<EqPreset>, EqfError> {
    if data.len() < EQF_HEADER.len() || &data[..27] != &EQF_HEADER[..27] {
        return Err(EqfError::BadHeader);
    }
    let mut out = Vec::new();
    let mut o = EQF_HEADER.len();
    while o + RECORD_LEN <= data.len() {
        let name_bytes = &data[o..o + NAME_LEN];
        let end = name_bytes.iter().position(|&b| b == 0).unwrap_or(NAME_LEN);
        let name: String = match std::str::from_utf8(&name_bytes[..end]) {
            Ok(s) => s.to_string(),
            Err(_) => name_bytes[..end].iter().map(|&b| b as char).collect(),
        };
        let mut bands = [CENTER; 10];
        for (i, b) in bands.iter_mut().enumerate() {
            *b = data[o + NAME_LEN + i].min(63);
        }
        let preamp = data[o + NAME_LEN + 10].min(63);
        out.push(EqPreset { name, preamp, bands });
        o += RECORD_LEN;
    }
    Ok(out)
}

/// Écrit un `.eqf` ou une bibliothèque.
pub fn write_eqf(presets: &[EqPreset]) -> Vec<u8> {
    let mut out = EQF_HEADER.to_vec();
    for p in presets {
        let mut name = [0u8; NAME_LEN];
        let bytes = p.name.as_bytes();
        let n = bytes.len().min(NAME_LEN - 1);
        name[..n].copy_from_slice(&bytes[..n]);
        out.extend_from_slice(&name);
        out.extend_from_slice(&p.bands);
        out.push(p.preamp);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_db_mapping() {
        assert_eq!(slider_to_db(31), 0.0);
        assert_eq!(slider_to_db(0), 12.0);
        assert_eq!(slider_to_db(63), -12.0);
        for v in 0..=63u8 {
            assert_eq!(db_to_slider(slider_to_db(v)), v);
        }
    }

    #[test]
    fn eqf_roundtrip() {
        let presets = factory_presets();
        let data = write_eqf(&presets);
        assert_eq!(data.len(), 31 + 268 * presets.len());
        assert_eq!(read_eqf(&data).unwrap(), presets);
        assert_eq!(read_eqf(b"nope").unwrap_err(), EqfError::BadHeader);
    }
}
