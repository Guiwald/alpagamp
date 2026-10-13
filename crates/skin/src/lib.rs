//! Skins classiques (`.wsz`) et skin par défaut d'Alpagamp.
//!
//! Règles de chargement (fidèles à Winamp, voir docs/reference/winamp.md §4) :
//! - un `.wsz` est un zip ; les sous-dossiers sont aplatis, la casse est ignorée, la dernière entrée
//!   d'un même nom l'emporte ;
//! - un zip qui contient `skin.xml` est un skin moderne : refusé ;
//! - chaque planche absente ou illisible est prise dans le skin par défaut, fichier par fichier ;
//! - `nums_ex.bmp` passe avant `numbers.bmp` ; sans `balance.bmp`, on prend `volume.bmp` du skin.

pub mod bmp;
pub mod config;
pub mod default_skin;
pub use default_skin::app_icon;
pub mod image;
pub mod sprites;
pub mod text;

use std::collections::HashMap;
use std::io::Read;

pub use config::{PlEditConfig, RegionWindow, Regions, VisColors};

/// Les 22 couleurs de `genex.bmp` (fenêtres génériques : listes, champs, boutons).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenExColors(pub [u32; 22]);

impl GenExColors {
    pub fn item_bg(&self) -> u32 {
        self.0[0]
    }
    pub fn item_fg(&self) -> u32 {
        self.0[1]
    }
    pub fn window_bg(&self) -> u32 {
        self.0[2]
    }
    pub fn button_text(&self) -> u32 {
        self.0[3]
    }
    pub fn window_text(&self) -> u32 {
        self.0[4]
    }
    pub fn divider(&self) -> u32 {
        self.0[5]
    }
    pub fn playlist_selection(&self) -> u32 {
        self.0[6]
    }
    pub fn scrollbar(&self) -> (u32, u32) {
        (self.0[13], self.0[14])
    }
    pub fn scrollbar_pressed(&self) -> (u32, u32) {
        (self.0[15], self.0[16])
    }
    pub fn scrollbar_dead(&self) -> u32 {
        self.0[17]
    }
    pub fn list_selected(&self) -> (u32, u32) {
        (self.0[20], self.0[21])
    }
}
pub use image::{Image, rgb};
pub use sprites::{Sheet, Sprite};

#[derive(Debug, thiserror::Error)]
pub enum SkinError {
    #[error("archive illisible : {0}")]
    Zip(String),
    #[error("skin moderne (.wal / skin.xml) : non pris en charge")]
    ModernSkin,
    #[error("aucun fichier de skin classique dans l'archive")]
    Empty,
}

/// Un skin chargé, toutes planches résolues.
#[derive(Clone)]
pub struct Skin {
    pub name: String,
    sheets: HashMap<Sheet, Image>,
    pub vis: VisColors,
    pub pledit: PlEditConfig,
    pub regions: Regions,
    pub readme: Option<String>,
    /// `true` si les chiffres viennent de `nums_ex.bmp` (signe moins en 9x13), sinon `numbers.bmp`.
    pub uses_nums_ex: bool,
    /// `true` si le skin fournit `eq_ex.bmp` (sinon, Winamp ne change pas l'aspect du bouton déplier).
    pub has_eq_ex: bool,
    /// Planches réellement fournies par le skin (les autres viennent du skin par défaut).
    pub provided: Vec<Sheet>,
}

impl std::fmt::Debug for Skin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Skin").field("name", &self.name).field("provided", &self.provided).finish()
    }
}

impl Skin {
    /// La planche demandée (toujours présente).
    pub fn sheet(&self, sheet: Sheet) -> &Image {
        &self.sheets[&sheet]
    }

    /// La planche des chiffres effectivement utilisée.
    /// Couleurs des fenêtres génériques, lues sur la ligne 0 de `genex.bmp`.
    pub fn genex_colors(&self) -> GenExColors {
        let img = self.sheet(Sheet::GenEx);
        let mut c = [0xFF00_0000u32; 22];
        for (i, v) in c.iter_mut().enumerate() {
            *v = img.get(48 + 2 * i as i32, 0).unwrap_or(0xFF00_0000) | 0xFF00_0000;
        }
        GenExColors(c)
    }

    /// Lettres A–Z du titre des fenêtres génériques (`gen.bmp`, lignes 88 et 96) : (x, largeur) pour la
    /// fenêtre active puis inactive. Les lettres sont séparées par la couleur du pixel (0, ligne).
    pub fn gen_letters(&self) -> [[(i32, i32); 26]; 2] {
        let img = self.sheet(Sheet::Gen);
        let mut out = [[(0, 0); 26]; 2];
        for (k, y) in [88, 96].into_iter().enumerate() {
            let bg = img.get(0, y);
            let mut x = 1;
            for slot in out[k].iter_mut() {
                let mut end = x;
                while end < img.width as i32 && img.get(end, y) != bg {
                    end += 1;
                }
                *slot = (x, end - x);
                x = end + 1;
            }
        }
        out
    }

    pub fn digits_sheet(&self) -> Sheet {
        if self.uses_nums_ex { Sheet::NumsEx } else { Sheet::Numbers }
    }

    /// Le skin par défaut (original, dessiné par le programme).
    pub fn default_skin() -> Skin {
        default_skin::build()
    }

    /// Construit un skin à partir de planches déjà décodées (utilisé par le skin par défaut).
    pub fn from_parts(name: &str, sheets: HashMap<Sheet, Image>, vis: VisColors, pledit: PlEditConfig) -> Skin {
        Skin {
            name: name.to_string(),
            provided: sheets.keys().copied().collect(),
            sheets,
            vis,
            pledit,
            regions: Regions::default(),
            readme: None,
            uses_nums_ex: true,
            has_eq_ex: true,
        }
    }

    /// Charge un `.wsz` (zip) en complétant ce qui manque avec `fallback`.
    pub fn load_wsz(name: &str, bytes: &[u8], fallback: &Skin) -> Result<Skin, SkinError> {
        let files = read_zip(bytes)?;
        Self::from_files(name, &files, fallback)
    }

    /// Charge un skin depuis un ensemble de fichiers (nom en minuscules, sans dossier → contenu).
    pub fn from_files(name: &str, files: &HashMap<String, Vec<u8>>, fallback: &Skin) -> Result<Skin, SkinError> {
        if files.contains_key("skin.xml") {
            return Err(SkinError::ModernSkin);
        }
        let mut sheets = HashMap::new();
        let mut provided = Vec::new();
        for sheet in Sheet::ALL {
            if let Some(data) = files.get(sheet.file_name()) {
                match bmp::decode(data) {
                    Ok(img) => {
                        sheets.insert(sheet, pad_to_reference(img, sheet));
                        provided.push(sheet);
                    }
                    Err(e) => log::warn!("skin {name} : {} illisible ({e}), remplacé par le skin par défaut", sheet.file_name()),
                }
            }
        }
        let has = |s: Sheet| provided.contains(&s);
        if provided.is_empty() && !files.contains_key("viscolor.txt") && !files.contains_key("pledit.txt") {
            return Err(SkinError::Empty);
        }
        // Chiffres : nums_ex d'abord, puis numbers, sinon ceux du skin par défaut.
        let uses_nums_ex = if has(Sheet::NumsEx) {
            true
        } else if has(Sheet::Numbers) {
            false
        } else {
            fallback.uses_nums_ex
        };
        // Balance absente : on reprend le volume du skin lui-même s'il existe.
        if !has(Sheet::Balance) && has(Sheet::Volume) {
            let vol = sheets[&Sheet::Volume].clone();
            sheets.insert(Sheet::Balance, vol);
        }
        for sheet in Sheet::ALL {
            sheets.entry(sheet).or_insert_with(|| fallback.sheet(sheet).clone());
        }
        let text = |f: &str| files.get(f).map(|b| decode_text(b));
        let vis = text("viscolor.txt").map(|t| VisColors::parse(&t, &fallback.vis)).unwrap_or(fallback.vis);
        let pledit = text("pledit.txt").map(|t| PlEditConfig::parse(&t, &fallback.pledit)).unwrap_or_else(|| fallback.pledit.clone());
        let regions = text("region.txt").map(|t| Regions::parse(&t)).unwrap_or_default();
        let readme = ["readme.txt", "read me.txt"]
            .iter()
            .find_map(|f| text(f))
            .or_else(|| {
                let mut others: Vec<&String> = files
                    .keys()
                    .filter(|k| k.ends_with(".txt") && !["pledit.txt", "viscolor.txt", "region.txt"].contains(&k.as_str()))
                    .collect();
                others.sort();
                others.first().and_then(|k| text(k))
            });
        Ok(Skin {
            name: name.to_string(),
            sheets,
            vis,
            pledit,
            regions,
            readme,
            uses_nums_ex,
            has_eq_ex: has(Sheet::EqEx),
            provided,
        })
    }

    /// Exporte le skin en `.wsz` (BMP 24 bits + fichiers texte).
    pub fn to_wsz(&self) -> Vec<u8> {
        use std::io::Write;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            for sheet in Sheet::ALL {
                if sheet == Sheet::Numbers && self.uses_nums_ex {
                    continue;
                }
                zip.start_file(sheet.file_name(), opts).expect("zip");
                zip.write_all(&bmp::encode_24(self.sheet(sheet))).expect("zip");
            }
            zip.start_file("viscolor.txt", opts).expect("zip");
            let mut v = String::new();
            for c in self.vis.0 {
                let (r, g, b) = image::split_rgb(c);
                v.push_str(&format!("{r},{g},{b}\r\n"));
            }
            zip.write_all(v.as_bytes()).expect("zip");
            zip.start_file("pledit.txt", opts).expect("zip");
            let hex = |c: u32| format!("#{:06X}", c & 0xFF_FFFF);
            let p = &self.pledit;
            let pl = format!(
                "[Text]\r\nNormal={}\r\nCurrent={}\r\nNormalBG={}\r\nSelectedBG={}\r\nmbFG={}\r\nmbBG={}\r\nFont={}\r\n",
                hex(p.normal),
                hex(p.current),
                hex(p.normal_bg),
                hex(p.selected_bg),
                hex(p.mb_fg),
                hex(p.mb_bg),
                p.font
            );
            zip.write_all(pl.as_bytes()).expect("zip");
            if let Some(r) = &self.readme {
                zip.start_file("readme.txt", opts).expect("zip");
                zip.write_all(r.as_bytes()).expect("zip");
            }
            zip.finish().expect("zip");
        }
        buf.into_inner()
    }
}

/// Complète une planche plus petite que la référence avec du noir (Winamp lisait hors de l'image).
fn pad_to_reference(img: Image, sheet: Sheet) -> Image {
    let (rw, rh) = sheet.reference_size();
    if img.width >= rw && img.height >= rh {
        return img;
    }
    let mut out = Image::new(img.width.max(rw), img.height.max(rh), rgb(0, 0, 0));
    out.blit(&img, 0, 0, img.width as i32, img.height as i32, 0, 0);
    out
}

/// Texte d'un skin : UTF-8 si valide, sinon Latin-1 (Windows-1252 approximé).
fn decode_text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.trim_start_matches('\u{FEFF}').to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

/// Lit les fichiers utiles d'un zip : `.bmp`, `.txt`, `.cur`, `.ini`, `.xml` (détection moderne).
/// Clé = nom de fichier en minuscules sans dossier ; la dernière entrée l'emporte.
pub fn read_zip(bytes: &[u8]) -> Result<HashMap<String, Vec<u8>>, SkinError> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| SkinError::Zip(e.to_string()))?;
    let mut files = HashMap::new();
    for i in 0..archive.len() {
        let mut f = match archive.by_index(i) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("entrée {i} du zip illisible : {e}");
                continue;
            }
        };
        if f.is_dir() {
            continue;
        }
        let full = f.name().map(|n| n.to_string()).unwrap_or_default().replace('\\', "/");
        let depth = full.trim_matches('/').matches('/').count();
        let base = full.rsplit('/').next().unwrap_or("").to_ascii_lowercase();
        let wanted = base.ends_with(".bmp") || base.ends_with(".txt") || base.ends_with(".cur") || base.ends_with(".ini");
        let is_modern_marker = base == "skin.xml" && depth <= 1;
        if !wanted && !is_modern_marker {
            continue;
        }
        if f.size() > 16 * 1024 * 1024 {
            continue;
        }
        let mut data = Vec::with_capacity(f.size() as usize);
        if f.read_to_end(&mut data).is_ok() {
            files.insert(base, data);
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::SimpleFileOptions::default();
            for (n, d) in entries {
                z.start_file(*n, o).unwrap();
                z.write_all(d).unwrap();
            }
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn flatten_case_and_last_wins_and_fallback() {
        let def = Skin::default_skin();
        let red = Image::new(275, 116, rgb(255, 0, 0));
        let blue = Image::new(275, 116, rgb(0, 0, 255));
        let zip = make_zip(&[
            ("MySkin/MAIN.BMP", &bmp::encode_24(&red)),
            ("other/Main.bmp", &bmp::encode_24(&blue)),
            ("MySkin/VISCOLOR.TXT", b"1,2,3\n"),
        ]);
        let skin = Skin::load_wsz("test", &zip, &def).unwrap();
        assert_eq!(skin.sheet(Sheet::Main).get(0, 0), Some(rgb(0, 0, 255)));
        assert_eq!(skin.provided, vec![Sheet::Main]);
        assert_eq!(skin.sheet(Sheet::Cbuttons), def.sheet(Sheet::Cbuttons));
        assert_eq!(skin.vis.0[0], rgb(1, 2, 3));
        assert_eq!(skin.vis.0[5], def.vis.0[5]);
    }

    #[test]
    fn modern_skin_refused() {
        let def = Skin::default_skin();
        let zip = make_zip(&[("skin.xml", b"<WinampAbstractionLayer/>"), ("main.bmp", b"x")]);
        assert!(matches!(Skin::load_wsz("m", &zip, &def), Err(SkinError::ModernSkin)));
    }

    #[test]
    fn numbers_and_balance_rules() {
        let def = Skin::default_skin();
        let nums = Image::new(99, 13, rgb(9, 9, 9));
        let vol = Image::new(68, 433, rgb(7, 7, 7));
        let zip = make_zip(&[("numbers.bmp", &bmp::encode_24(&nums)), ("volume.bmp", &bmp::encode_24(&vol))]);
        let skin = Skin::load_wsz("n", &zip, &def).unwrap();
        assert!(!skin.uses_nums_ex);
        assert_eq!(skin.sheet(Sheet::Balance).get(20, 20), Some(rgb(7, 7, 7)));
    }

    #[test]
    fn default_skin_roundtrips_through_wsz() {
        let def = Skin::default_skin();
        let wsz = def.to_wsz();
        let again = Skin::load_wsz("rt", &wsz, &def).unwrap();
        for sheet in Sheet::ALL {
            if sheet == Sheet::Numbers {
                continue;
            }
            assert_eq!(again.sheet(sheet), def.sheet(sheet), "{sheet:?}");
        }
        assert_eq!(again.vis, def.vis);
        assert_eq!(again.pledit, def.pledit);
    }

    #[test]
    fn gen_letters_have_widths() {
        let def = Skin::default_skin();
        let l = def.gen_letters();
        for k in 0..2 {
            assert!(l[k].iter().all(|&(_, w)| (2..=6).contains(&w)), "{:?}", l[k]);
            assert!(l[k][25].0 + l[k][25].1 < 194);
        }
        assert_ne!(def.genex_colors().item_bg(), def.genex_colors().item_fg());
    }
}
