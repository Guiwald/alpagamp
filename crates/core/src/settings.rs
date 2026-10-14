//! Réglages persistants (fichier TOML dans le dossier de configuration de l'utilisateur).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::eq::EqSettings;

/// Mode du visualiseur intégré.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VisMode {
    #[default]
    Analyzer,
    Oscilloscope,
    Off,
}

impl VisMode {
    /// Clic sur le visualiseur : Analyseur → Oscilloscope → Éteint → Analyseur.
    pub fn cycle(self) -> VisMode {
        match self {
            VisMode::Analyzer => VisMode::Oscilloscope,
            VisMode::Oscilloscope => VisMode::Off,
            VisMode::Off => VisMode::Analyzer,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AnalyzerStyle {
    #[default]
    Normal,
    Fire,
    Line,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScopeStyle {
    Dots,
    #[default]
    Lines,
    Solid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VisSettings {
    pub mode: VisMode,
    pub analyzer_style: AnalyzerStyle,
    pub scope_style: ScopeStyle,
    /// Barres épaisses (19) ou fines (75).
    pub thick_bands: bool,
    pub peaks: bool,
    /// Vitesse de chute des barres (0 = lente … 4 = rapide).
    pub falloff: u8,
    /// Vitesse de chute des crêtes (0 … 4).
    pub peak_falloff: u8,
    /// Rafraîchissement : 0 = 70 i/s, 1 = 35, 2 = 18, 3 = 9.
    pub refresh: u8,
}

impl Default for VisSettings {
    fn default() -> Self {
        Self {
            mode: VisMode::Analyzer,
            analyzer_style: AnalyzerStyle::Normal,
            scope_style: ScopeStyle::Lines,
            thick_bands: true,
            peaks: true,
            falloff: 2,
            peak_falloff: 1,
            refresh: 1,
        }
    }
}

/// Normalisation du volume (ReplayGain).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReplayGainMode {
    #[default]
    Off,
    Track,
    Album,
}

/// Réglages de lecture.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaybackSettings {
    pub replay_gain: ReplayGainMode,
    /// Préampli ReplayGain (dB).
    pub rg_preamp_db: f32,
    /// Gain des fichiers sans information ReplayGain (dB).
    pub rg_untagged_db: f32,
    /// Limiter le gain grâce aux crêtes connues.
    pub rg_prevent_clipping: bool,
    /// Fondu enchaîné entre deux morceaux (0 = enchaînement sans blanc).
    pub crossfade_ms: u32,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self { replay_gain: ReplayGainMode::Off, rg_preamp_db: 0.0, rg_untagged_db: 0.0, rg_prevent_clipping: true, crossfade_ms: 0 }
    }
}

/// Fenêtre de visualisation (presets MilkDrop).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MilkSettings {
    /// Secondes entre deux changements automatiques de preset (0 = jamais).
    pub auto_secs: u32,
    /// Durée du fondu entre deux presets, en dixièmes de seconde (0 = coupe franche).
    pub blend_tenths: u32,
    /// Presets dans le désordre.
    pub shuffle: bool,
    /// Garder le preset affiché.
    pub locked: bool,
    /// Dernier preset affiché (dossier/nom).
    pub last_preset: Option<String>,
    pub window: Option<WindowState>,
    /// Téléphone : tout l'écran est composé par la carte graphique (nécessaire à la visualisation). Désactivable
    /// en cas de souci d'affichage.
    pub gpu: bool,
}

impl Default for MilkSettings {
    fn default() -> Self {
        Self { auto_secs: 20, blend_tenths: 27, shuffle: true, locked: false, last_preset: None, window: None, gpu: true }
    }
}

/// Plugins WebAssembly (ADR 0020).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PluginSettings {
    /// Effets audio et plugins généraux activés (identifiants : nom du fichier sans `.wasm`).
    pub enabled: Vec<String>,
    /// Visualisation de plugin affichée à la place de MilkDrop (`None` : MilkDrop).
    pub vis: Option<String>,
    /// Options choisies : plugin → (option → valeur).
    pub options: std::collections::BTreeMap<String, std::collections::BTreeMap<String, i32>>,
}

impl PluginSettings {
    pub fn is_enabled(&self, id: &str) -> bool {
        self.enabled.iter().any(|e| e == id) || self.vis.as_deref() == Some(id)
    }
}

/// Position et état d'une fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub open: bool,
    pub shade: bool,
    /// Taille (playlist seulement), en pixels skin.
    pub w: u32,
    pub h: u32,
    /// Taille de la fenêtre à l'enregistrement, en unités de disposition (0 : inconnue, réglages d'avant la 0.6.1).
    /// Sert à retrouver au démarrage quelles fenêtres étaient collées, même si l'agrandissement a changé.
    #[serde(default)]
    pub lw: i32,
    #[serde(default)]
    pub lh: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Volume 0..=255 (défaut 200).
    pub volume: u8,
    /// Balance −127..=127.
    pub balance: i8,
    pub shuffle: bool,
    pub repeat: bool,
    pub manual_advance: bool,
    pub time_remaining: bool,
    /// Ancienne « double taille » (fenêtre principale et égaliseur) ; remplacée par `window_zoom`.
    pub double_size: bool,
    /// Ancien agrandissement entier (0.5 à 0.6.0) : 1, 2 ou 3 (0 = selon l'écran). Relu s'il est seul ; tenu à jour
    /// (partie entière) pour les anciennes versions.
    pub window_zoom: u8,
    /// Agrandissement de toutes les fenêtres, en pour cent : 100, 150, 200, 250 ou 300 (0 = selon l'écran).
    pub window_scale: u16,
    /// Unités des positions enregistrées : 0 = jusqu'à la 0.6.0 (points sur macOS), 1 = depuis la 0.6.1 (demi-points
    /// sur macOS). Pixels physiques ailleurs dans les deux cas.
    pub layout_units: u8,
    pub always_on_top: bool,
    /// Chemin du skin choisi (`None` = skin par défaut).
    pub skin: Option<PathBuf>,
    pub eq: EqSettings,
    pub vis: VisSettings,
    pub playback: PlaybackSettings,
    pub milkdrop: MilkSettings,
    pub plugins: PluginSettings,
    pub main_window: Option<WindowState>,
    pub eq_window: Option<WindowState>,
    pub playlist_window: Option<WindowState>,
    /// Langue forcée (`None` = langue du système).
    pub language: Option<String>,
    /// Dernier dossier ouvert.
    pub last_dir: Option<PathBuf>,
    /// Morceau courant et position à la fermeture.
    pub resume_index: Option<usize>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: 200,
            balance: 0,
            shuffle: false,
            repeat: false,
            manual_advance: false,
            time_remaining: false,
            double_size: false,
            window_zoom: 0,
            window_scale: 0,
            layout_units: 0,
            always_on_top: false,
            skin: None,
            eq: EqSettings::default(),
            vis: VisSettings::default(),
            playback: PlaybackSettings::default(),
            milkdrop: MilkSettings::default(),
            plugins: PluginSettings::default(),
            main_window: None,
            eq_window: None,
            playlist_window: None,
            language: None,
            last_dir: None,
            resume_index: None,
        }
    }
}

impl Settings {
    /// Agrandissement des fenêtres, en pour cent (100 à 300 par demi-crans) ; `auto` : celui que l'écran suggère.
    pub fn scale(&self, auto: u32) -> u32 {
        let pct = match self.window_scale {
            0 => match self.window_zoom {
                0 if self.double_size => 200,
                0 => auto,
                z => z as u32 * 100,
            },
            p => p as u32,
        };
        snap_scale(pct)
    }

    /// Choisit l'agrandissement (0 = selon l'écran).
    pub fn set_scale(&mut self, pct: u32) {
        let pct = if pct == 0 { 0 } else { snap_scale(pct) };
        self.window_scale = pct as u16;
        // Les versions 0.5 à 0.6.0 ne lisent que ce champ : la partie entière (×1,5 → ×1).
        self.window_zoom = (pct / 100) as u8;
        self.double_size = false;
    }

    /// Agrandissement suggéré par l'écran : largeur en points, et `half_crisp` quand ×1,5 y tombe sur des pixels
    /// entiers (écran Retina). ×1 sous 1 400 points ; ×1,5 jusqu'à 2 100 sur Retina (×2 sinon) ; ×2 jusqu'à 2 800 ;
    /// ×3 au-delà.
    pub fn auto_scale(screen_width_points: f64, half_crisp: bool) -> u32 {
        if screen_width_points < 1400.0 {
            100
        } else if screen_width_points < 2100.0 && half_crisp {
            150
        } else if screen_width_points < 2800.0 {
            200
        } else {
            300
        }
    }

    /// Agrandissement qu'avaient les versions 0.5 à 0.6.0, pour relire les positions qu'elles ont enregistrées.
    pub fn legacy_scale(&self, screen_width_points: f64) -> u32 {
        match self.window_zoom {
            0 if self.double_size => 200,
            0 if screen_width_points < 1400.0 => 100,
            0 if screen_width_points < 2800.0 => 200,
            0 => 300,
            z => (z as u32).clamp(1, 3) * 100,
        }
    }

    /// Charge les réglages ; renvoie les valeurs par défaut si le fichier manque ou est illisible.
    pub fn load(path: &Path) -> Settings {
        match std::fs::read_to_string(path) {
            Ok(t) => toml::from_str(&t).unwrap_or_else(|e| {
                log::warn!("réglages illisibles ({e}) : valeurs par défaut");
                Settings::default()
            }),
            Err(_) => Settings::default(),
        }
    }

    /// Enregistre de façon atomique (fichier temporaire puis renommage).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, path)
    }
}

/// Agrandissements proposés, en pour cent.
pub const SCALES: [u32; 5] = [100, 150, 200, 250, 300];

/// Ramène un agrandissement au demi-cran le plus proche, entre ×1 et ×3.
pub fn snap_scale(pct: u32) -> u32 {
    ((pct.clamp(100, 300) + 25) / 50) * 50
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_partial_file() {
        let dir = std::env::temp_dir().join(format!("alpagamp-test-{}", std::process::id()));
        let path = dir.join("settings.toml");
        let mut s = Settings::default();
        s.volume = 123;
        s.vis.mode = VisMode::Oscilloscope;
        s.milkdrop.last_preset = Some("Dancer/Comet/x".into());
        s.milkdrop.window = Some(WindowState { x: 1, y: 2, open: true, shade: false, w: 300, h: 232, lw: 600, lh: 464 });
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        std::fs::write(&path, "volume = 7\n").unwrap();
        let p = Settings::load(&path);
        assert_eq!(p.volume, 7);
        assert_eq!(p.balance, 0);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn window_scale() {
        let mut s = Settings::default();
        // Selon l'écran.
        assert_eq!(s.scale(150), 150);
        assert_eq!(Settings::auto_scale(1512.0, true), 150);
        assert_eq!(Settings::auto_scale(1512.0, false), 200);
        assert_eq!(Settings::auto_scale(1280.0, true), 100);
        assert_eq!(Settings::auto_scale(2560.0, true), 200);
        assert_eq!(Settings::auto_scale(3840.0, false), 300);
        // Réglages des versions 0.5 à 0.6.0.
        s.double_size = true;
        assert_eq!(s.scale(100), 200);
        s.window_zoom = 3;
        assert_eq!(s.scale(100), 300);
        assert_eq!(s.legacy_scale(1512.0), 300);
        s.window_zoom = 0;
        s.double_size = false;
        assert_eq!(s.legacy_scale(1512.0), 200);
        // Demi-crans.
        s.set_scale(150);
        assert_eq!((s.window_scale, s.window_zoom, s.scale(300)), (150, 1, 150));
        s.set_scale(260);
        assert_eq!(s.scale(100), 250);
        s.set_scale(0);
        assert_eq!((s.window_zoom, s.scale(200)), (0, 200));
        assert_eq!(snap_scale(40), 100);
        assert_eq!(snap_scale(999), 300);
    }

    #[test]
    fn reads_window_state_without_layout_size() {
        let s: Settings = toml::from_str("[main_window]\nx = 10\ny = 20\nopen = true\nshade = false\nw = 275\nh = 116\n").unwrap();
        let w = s.main_window.unwrap();
        assert_eq!((w.x, w.lw, w.lh), (10, 0, 0));
    }
}
