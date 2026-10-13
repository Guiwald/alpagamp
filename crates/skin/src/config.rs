//! Fichiers texte d'un skin : `viscolor.txt`, `pledit.txt`, `region.txt`.

use crate::image::rgb;

/// Les 24 couleurs du visualiseur.
///
/// 0 = fond, 1 = points de la grille, 2..=17 = dégradé de l'analyseur (2 en haut, 17 en bas),
/// 18..=22 = oscilloscope (18 au centre), 23 = crêtes de l'analyseur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisColors(pub [u32; 24]);

impl VisColors {
    /// Lit `viscolor.txt` comme Winamp : chaque ligne commence par trois entiers séparés par des
    /// espaces, virgules ou tabulations ; **la première ligne qui ne commence pas par un nombre arrête
    /// la lecture**, et les couleurs non lues gardent la valeur de `defaults`.
    pub fn parse(text: &str, defaults: &VisColors) -> VisColors {
        let mut out = *defaults;
        for (i, line) in text.lines().take(24).enumerate() {
            let mut nums = [0u32; 3];
            let mut rest = line.trim_start_matches([' ', ',', '\t']);
            let mut ok = true;
            for n in nums.iter_mut() {
                rest = rest.trim_start_matches([' ', ',', '\t']);
                let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                if digits.is_empty() {
                    ok = false;
                    break;
                }
                rest = &rest[digits.len()..];
                *n = digits.parse::<u32>().unwrap_or(0).min(255);
            }
            if !ok {
                break;
            }
            out.0[i] = rgb(nums[0] as u8, nums[1] as u8, nums[2] as u8);
        }
        out
    }
}

/// Couleurs et police de la playlist (`pledit.txt`, section `[Text]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlEditConfig {
    pub normal: u32,
    pub current: u32,
    pub normal_bg: u32,
    pub selected_bg: u32,
    pub mb_fg: u32,
    pub mb_bg: u32,
    pub font: String,
}

impl Default for PlEditConfig {
    fn default() -> Self {
        Self {
            normal: rgb(0x00, 0xFF, 0x00),
            current: rgb(0xFF, 0xFF, 0xFF),
            normal_bg: rgb(0x00, 0x00, 0x00),
            selected_bg: rgb(0x00, 0x00, 0xC6),
            mb_fg: rgb(0x00, 0xFF, 0x00),
            mb_bg: rgb(0x00, 0x00, 0x00),
            font: "Arial".to_string(),
        }
    }
}

impl PlEditConfig {
    pub fn parse(text: &str, defaults: &PlEditConfig) -> PlEditConfig {
        let mut out = defaults.clone();
        let ini = Ini::parse(text);
        let color = |key: &str| ini.get("text", key).and_then(parse_hex_color);
        if let Some(c) = color("normal") {
            out.normal = c;
        }
        if let Some(c) = color("current") {
            out.current = c;
        }
        if let Some(c) = color("normalbg") {
            out.normal_bg = c;
        }
        if let Some(c) = color("selectedbg") {
            out.selected_bg = c;
        }
        if let Some(c) = color("mbfg") {
            out.mb_fg = c;
        }
        if let Some(c) = color("mbbg") {
            out.mb_bg = c;
        }
        if let Some(f) = ini.get("text", "font") {
            let f = f.trim();
            if !f.is_empty() {
                out.font = f.to_string();
            }
        }
        out
    }
}

/// Lit `#RRGGBB` ou `RRGGBB` (on garde les 24 bits de poids faible, comme Winamp).
fn parse_hex_color(v: &str) -> Option<u32> {
    let v = v.trim().trim_start_matches('#');
    let hex: String = v.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
    if hex.is_empty() {
        return None;
    }
    let n = u64::from_str_radix(&hex[hex.len().saturating_sub(8)..], 16).ok()? as u32;
    Some(0xFF00_0000 | (n & 0x00FF_FFFF))
}

/// Fenêtres découpées par `region.txt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegionWindow {
    Main,
    MainShade,
    Equalizer,
    EqualizerShade,
}

/// Une liste de polygones (coordonnées de bords de pixels, sens horaire).
pub type Polygons = Vec<Vec<(i32, i32)>>;

/// Le contenu de `region.txt`. Une fenêtre absente reste rectangulaire.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Regions {
    pub main: Option<Polygons>,
    pub main_shade: Option<Polygons>,
    pub equalizer: Option<Polygons>,
    pub equalizer_shade: Option<Polygons>,
}

impl Regions {
    pub fn parse(text: &str) -> Regions {
        let ini = Ini::parse(text);
        let section = |name: &str| -> Option<Polygons> {
            let counts = parse_numbers(ini.get(name, "numpoints")?);
            let points = parse_numbers(ini.get(name, "pointlist")?);
            if counts.is_empty() || points.len() % 2 != 0 {
                return None;
            }
            let total: i64 = counts.iter().map(|&c| c as i64).sum();
            if total * 2 != points.len() as i64 || counts.iter().any(|&c| c < 0) {
                return None;
            }
            let mut polys = Vec::new();
            let mut it = points.chunks(2).map(|p| (p[0], p[1]));
            for &c in &counts {
                let poly: Vec<(i32, i32)> = it.by_ref().take(c as usize).collect();
                if poly.len() >= 3 {
                    polys.push(poly);
                }
            }
            if polys.is_empty() { None } else { Some(polys) }
        };
        Regions {
            main: section("normal"),
            main_shade: section("windowshade"),
            equalizer: section("equalizer"),
            equalizer_shade: section("equalizerws"),
        }
    }

    pub fn get(&self, w: RegionWindow) -> Option<&Polygons> {
        match w {
            RegionWindow::Main => self.main.as_ref(),
            RegionWindow::MainShade => self.main_shade.as_ref(),
            RegionWindow::Equalizer => self.equalizer.as_ref(),
            RegionWindow::EqualizerShade => self.equalizer_shade.as_ref(),
        }
    }

    /// Masque de visibilité (`true` = pixel visible) pour une fenêtre de `w`x`h`.
    /// Règle « pair-impair » sur l'union des polygones, testée au centre des pixels.
    pub fn mask(polys: &Polygons, w: u32, h: u32) -> Vec<bool> {
        let mut out = vec![false; (w * h) as usize];
        for y in 0..h {
            let cy = y as f64 + 0.5;
            for x in 0..w {
                let cx = x as f64 + 0.5;
                let inside = polys.iter().any(|p| point_in_polygon(p, cx, cy));
                out[(y * w + x) as usize] = inside;
            }
        }
        out
    }
}

fn point_in_polygon(poly: &[(i32, i32)], x: f64, y: f64) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (poly[i].0 as f64, poly[i].1 as f64);
        let (xj, yj) = (poly[j].0 as f64, poly[j].1 as f64);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Lit une liste de nombres séparés par des virgules, espaces ou tabulations ; s'arrête au premier
/// élément non numérique (les commentaires `;` en fin de ligne sont donc sans effet).
fn parse_numbers(v: &str) -> Vec<i32> {
    let mut out = Vec::new();
    for tok in v.split([',', ' ', '\t']).filter(|t| !t.is_empty()) {
        match tok.parse::<i32>() {
            Ok(n) => out.push(n),
            Err(_) => break,
        }
    }
    out
}

/// Lecteur INI minimal, insensible à la casse des sections et des clés.
pub struct Ini {
    entries: Vec<(String, String, String)>,
}

impl Ini {
    pub fn parse(text: &str) -> Ini {
        let mut entries = Vec::new();
        let mut section = String::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') && !line.contains('=') {
                continue;
            }
            if line.starts_with('[') {
                if let Some(end) = line.find(']') {
                    section = line[1..end].trim().to_ascii_lowercase();
                }
                continue;
            }
            if let Some(eq) = line.find('=') {
                let key = line[..eq].trim().to_ascii_lowercase();
                let val = line[eq + 1..].trim().to_string();
                entries.push((section.clone(), key, val));
            }
        }
        Ini { entries }
    }

    /// Dernière valeur de `section.key` (insensible à la casse).
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        let section = section.to_ascii_lowercase();
        let key = key.to_ascii_lowercase();
        self.entries.iter().rev().find(|(s, k, _)| *s == section && *k == key).map(|(_, _, v)| v.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viscolor_stops_at_first_non_numeric_line() {
        let defaults = VisColors([rgb(1, 1, 1); 24]);
        let v = VisColors::parse("0,0,0 // fond\n 24, 33,41\nhello\n9,9,9\n", &defaults);
        assert_eq!(v.0[0], rgb(0, 0, 0));
        assert_eq!(v.0[1], rgb(24, 33, 41));
        assert_eq!(v.0[2], rgb(1, 1, 1));
        assert_eq!(v.0[3], rgb(1, 1, 1));
    }

    #[test]
    fn pledit_case_insensitive() {
        let p = PlEditConfig::parse("[text]\nNORMAL=#ff0000\nCurrent=00FF00\nFont=Tahoma\n", &PlEditConfig::default());
        assert_eq!(p.normal, rgb(255, 0, 0));
        assert_eq!(p.current, rgb(0, 255, 0));
        assert_eq!(p.font, "Tahoma");
        assert_eq!(p.selected_bg, PlEditConfig::default().selected_bg);
    }

    #[test]
    fn region_parse_and_mask() {
        let r = Regions::parse("[Normal]\nNumPoints=4\nPointList=0,0, 275,0, 275,116, 0,116 ; tout\n[Equalizer]\nNumPoints=3\nPointList=0,0,1\n");
        let main = r.main.as_ref().unwrap();
        assert_eq!(main[0].len(), 4);
        assert!(r.equalizer.is_none(), "liste impaire : section ignorée");
        let m = Regions::mask(&vec![vec![(1, 0), (3, 0), (3, 2), (1, 2)]], 4, 3);
        assert_eq!(m, vec![false, true, true, false, false, true, true, false, false, false, false, false]);
    }
}
