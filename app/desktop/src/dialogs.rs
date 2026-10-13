//! Boîtes de dialogue natives (macOS, Windows). Sous Linux (développement), elles sont absentes.

use std::path::{Path, PathBuf};

use alpagamp_core::formats::AUDIO_EXTENSIONS;
use alpagamp_core::i18n::t;

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn pick_files(dir: Option<&Path>) -> Vec<PathBuf> {
    let mut d = rfd::FileDialog::new()
        .add_filter(t("dialog.audio_files"), AUDIO_EXTENSIONS)
        .add_filter(t("dialog.playlists"), &["m3u", "m3u8", "pls"])
        .add_filter(t("dialog.all_files"), &["*"]);
    if let Some(dir) = dir {
        d = d.set_directory(dir);
    }
    d.pick_files().unwrap_or_default()
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn pick_folder(dir: Option<&Path>) -> Option<PathBuf> {
    let mut d = rfd::FileDialog::new();
    if let Some(dir) = dir {
        d = d.set_directory(dir);
    }
    d.pick_folder()
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn pick_skin() -> Option<PathBuf> {
    rfd::FileDialog::new().add_filter(t("dialog.skins"), &["wsz", "zip"]).pick_file()
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn message(title: &str, text: &str) {
    rfd::MessageDialog::new().set_title(title).set_description(text).set_level(rfd::MessageLevel::Info).show();
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn pick_files(_dir: Option<&Path>) -> Vec<PathBuf> {
    let _ = (AUDIO_EXTENSIONS, t("dialog.audio_files"));
    log::info!("boîte d'ouverture non disponible sur cette plateforme : passer les fichiers en argument");
    Vec::new()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn pick_folder(_dir: Option<&Path>) -> Option<PathBuf> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn pick_skin() -> Option<PathBuf> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn message(title: &str, text: &str) {
    log::info!("{title} : {text}");
}
