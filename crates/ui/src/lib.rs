//! Fenêtres skinnées d'Alpagamp, indépendantes de la plateforme : chaque fenêtre se dessine dans une
//! image RGBA à l'échelle 1 et traduit les clics (en coordonnées skin) en [`Action`].

pub mod main_window;
pub mod marquee;
pub mod paint;
pub mod vis;

pub use main_window::{MainView, MainWindow};

/// État du transport tel qu'affiché.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Transport {
    #[default]
    Stopped,
    Playing,
    Paused,
}

/// Touches de modification au moment d'un clic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// Ce que l'utilisateur demande en cliquant dans une fenêtre.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Previous(Mods),
    Play(Mods),
    Pause,
    Stop(Mods),
    Next(Mods),
    Eject(Mods),
    SetVolume(u8),
    SetBalance(i8),
    /// Aller à cette position (ms).
    Seek(u64),
    ToggleShuffle,
    ToggleRepeat,
    ToggleEqWindow,
    TogglePlaylistWindow,
    Close,
    Minimize,
    ToggleShade,
    OptionsMenu,
    ToggleAlwaysOnTop,
    FileInfo,
    ToggleDoubleSize,
    VisMenu,
    ToggleTimeMode,
    CycleVisMode,
    ToggleVisPlugin,
    /// Commencer à déplacer la fenêtre (l'application suit la souris et gère l'aimantation).
    StartMove,
    About,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alpagamp_skin::{Image, Skin};

    fn view() -> MainView {
        MainView {
            transport: Transport::Playing,
            position_ms: 83_000,
            duration_ms: Some(296_000),
            seekable: true,
            title: "1. Alpagamp - Chanson de test tres longue pour faire defiler le titre (4:56)".into(),
            kbps: Some(320),
            khz: Some(44),
            stereo: Some(true),
            volume: 200,
            balance: 0,
            eq_open: true,
            pl_open: true,
            active: true,
            ..Default::default()
        }
    }

    #[test]
    fn renders_both_modes_without_panicking() {
        let skin = Skin::default_skin();
        let mut w = MainWindow::new();
        let mut fb = Image::new(1, 1, 0);
        let v = view();
        w.render(&mut fb, &skin, &v);
        assert_eq!((fb.width, fb.height), (275, 116));
        w.shade = true;
        w.render(&mut fb, &skin, &v);
        assert_eq!((fb.width, fb.height), (275, 14));
    }

    #[test]
    fn click_play_and_volume_drag() {
        let mut w = MainWindow::new();
        let v = view();
        assert!(w.mouse_down(45, 95, Mods::default(), &v).is_empty());
        assert_eq!(w.mouse_up(45, 95, Mods::default(), &v), vec![Action::Play(Mods::default())]);
        // Relâcher hors du bouton n'agit pas.
        w.mouse_down(45, 95, Mods::default(), &v);
        assert!(w.mouse_up(150, 20, Mods::default(), &v).is_empty());
        // Volume : glisser tout à gauche puis tout à droite.
        let a = w.mouse_down(108, 62, Mods::default(), &v);
        assert_eq!(a, vec![Action::SetVolume(0)]);
        let a = w.mouse_move(200, 62, Mods::default(), &v);
        assert_eq!(a, vec![Action::SetVolume(255)]);
        w.mouse_up(200, 62, Mods::default(), &v);
        // Barre de position : relâcher au milieu → déplacement.
        w.mouse_down(140, 76, Mods::default(), &v);
        let a = w.mouse_up(140, 76, Mods::default(), &v);
        assert!(matches!(a.as_slice(), [Action::Seek(ms)] if (140_000..170_000).contains(ms)), "{a:?}");
    }

    #[test]
    fn title_double_click_toggles_shade() {
        let mut w = MainWindow::new();
        assert_eq!(w.double_click(100, 5, Mods::default()), vec![Action::ToggleShade]);
        assert_eq!(w.mouse_down(100, 5, Mods::default(), &view()), vec![Action::StartMove]);
    }
}
