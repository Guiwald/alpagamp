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
    pub double_size: bool,
    pub always_on_top: bool,
    /// Chemin du skin choisi (`None` = skin par défaut).
    pub skin: Option<PathBuf>,
    pub eq: EqSettings,
    pub vis: VisSettings,
    pub playback: PlaybackSettings,
    pub milkdrop: MilkSettings,
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
            always_on_top: false,
            skin: None,
            eq: EqSettings::default(),
            vis: VisSettings::default(),
            playback: PlaybackSettings::default(),
            milkdrop: MilkSettings::default(),
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
        s.milkdrop.window = Some(WindowState { x: 1, y: 2, open: true, shade: false, w: 300, h: 232 });
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        std::fs::write(&path, "volume = 7\n").unwrap();
        let p = Settings::load(&path);
        assert_eq!(p.volume, 7);
        assert_eq!(p.balance, 0);
        std::fs::remove_dir_all(dir).ok();
    }
}
