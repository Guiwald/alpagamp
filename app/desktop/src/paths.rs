//! Emplacements des fichiers de l'utilisateur.

use std::path::PathBuf;

pub struct Paths {
    pub config_dir: PathBuf,
    pub settings: PathBuf,
    pub playlist: PathBuf,
    pub skins_dir: PathBuf,
    /// Préréglages d'égaliseur automatiques (un par morceau, au format bibliothèque `.eqf`).
    pub eq_auto: PathBuf,
    pub bookmarks: PathBuf,
    /// Presets de visualisation : le pack téléchargé et ceux que l'on y dépose.
    pub presets_dir: PathBuf,
}

impl Paths {
    pub fn new() -> Paths {
        let config_dir = directories::ProjectDirs::from("fr", "Guiwald", "Alpagamp")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir().join("alpagamp"));
        Paths {
            settings: config_dir.join("settings.toml"),
            playlist: config_dir.join("playlist.m3u8"),
            skins_dir: config_dir.join("skins"),
            eq_auto: config_dir.join("eq-auto.eqf"),
            bookmarks: config_dir.join("bookmarks.m3u8"),
            presets_dir: config_dir.join("presets"),
            config_dir,
        }
    }
}
