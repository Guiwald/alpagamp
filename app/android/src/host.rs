//! Ce que l'appli mobile demande au système : zone visible, clavier, permission, dossiers.

use std::path::PathBuf;

pub trait Host {
    /// Haut et bas (pixels) de la zone visible, hors barres du système et clavier.
    fn content_rect(&self) -> Option<(i32, i32)>;
    fn show_keyboard(&self, show: bool);
    fn has_music_permission(&self) -> bool;
    fn request_music_permission(&self);
    /// Fermer l'appli.
    fn finish(&self);
    /// Dossiers où chercher la musique.
    fn music_dirs(&self) -> Vec<PathBuf>;
    /// Dossier des réglages et de la playlist.
    fn data_dir(&self) -> PathBuf;
    /// Fonction qui ouvre une adresse dans le navigateur (connexion à Spotify).
    fn url_opener(&self) -> Box<dyn Fn(&str) + Send + Sync>;
}

#[cfg(target_os = "android")]
pub struct AndroidHost(pub winit::platform::android::activity::AndroidApp);

#[cfg(target_os = "android")]
impl Host for AndroidHost {
    fn content_rect(&self) -> Option<(i32, i32)> {
        let r = self.0.content_rect();
        Some((r.top, r.bottom))
    }
    fn show_keyboard(&self, show: bool) {
        // Demande explicite : une demande « implicite » n'ouvre souvent pas le clavier.
        if show { self.0.show_soft_input(false) } else { self.0.hide_soft_input(false) }
    }
    fn has_music_permission(&self) -> bool {
        crate::platform::has_music_permission(&self.0)
    }
    fn request_music_permission(&self) {
        crate::platform::request_music_permission(&self.0)
    }
    fn finish(&self) {
        crate::platform::finish(&self.0)
    }
    fn music_dirs(&self) -> Vec<PathBuf> {
        crate::platform::music_dirs()
    }
    fn data_dir(&self) -> PathBuf {
        crate::platform::data_dir(&self.0)
    }
    fn url_opener(&self) -> Box<dyn Fn(&str) + Send + Sync> {
        let app = self.0.clone();
        Box::new(move |u: &str| crate::platform::open_url(&app, u))
    }
}

/// Simulateur de téléphone sur ordinateur (développement) : `ALPAGAMP_SIM_MUSIC` = dossier de musique.
#[cfg(not(target_os = "android"))]
pub struct SimHost;

#[cfg(not(target_os = "android"))]
impl Host for SimHost {
    fn content_rect(&self) -> Option<(i32, i32)> {
        None
    }
    fn show_keyboard(&self, show: bool) {
        log::info!("clavier : {show}");
    }
    fn has_music_permission(&self) -> bool {
        true
    }
    fn request_music_permission(&self) {}
    fn finish(&self) {}
    fn music_dirs(&self) -> Vec<PathBuf> {
        std::env::var_os("ALPAGAMP_SIM_MUSIC").map(PathBuf::from).into_iter().collect()
    }
    fn data_dir(&self) -> PathBuf {
        std::env::temp_dir().join("alpagamp-phone-sim")
    }
    fn url_opener(&self) -> Box<dyn Fn(&str) + Send + Sync> {
        Box::new(|u: &str| {
            if std::process::Command::new("xdg-open").arg(u).spawn().is_err() {
                log::info!("ouvrir : {u}");
            }
        })
    }
}
