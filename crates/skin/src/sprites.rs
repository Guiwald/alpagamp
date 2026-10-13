//! Coordonnées des sprites dans les planches d'un skin classique.
//!
//! Ces coordonnées sont des faits du format (documentés par le Skinner's Atlas et par Webamp,
//! https://github.com/captbaritone/webamp, licence MIT — voir THIRD_PARTY.md). Les noms sont les nôtres.

/// Une planche d'un skin (un fichier BMP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sheet {
    Main,
    Titlebar,
    Cbuttons,
    Shufrep,
    Posbar,
    Volume,
    Balance,
    Monoster,
    Playpaus,
    Numbers,
    NumsEx,
    Text,
    EqMain,
    EqEx,
    PlEdit,
    Gen,
    GenEx,
}

impl Sheet {
    pub const ALL: [Sheet; 17] = [
        Sheet::Main,
        Sheet::Titlebar,
        Sheet::Cbuttons,
        Sheet::Shufrep,
        Sheet::Posbar,
        Sheet::Volume,
        Sheet::Balance,
        Sheet::Monoster,
        Sheet::Playpaus,
        Sheet::Numbers,
        Sheet::NumsEx,
        Sheet::Text,
        Sheet::EqMain,
        Sheet::EqEx,
        Sheet::PlEdit,
        Sheet::Gen,
        Sheet::GenEx,
    ];

    /// Nom du fichier dans l'archive (comparaison insensible à la casse).
    pub fn file_name(self) -> &'static str {
        match self {
            Sheet::Main => "main.bmp",
            Sheet::Titlebar => "titlebar.bmp",
            Sheet::Cbuttons => "cbuttons.bmp",
            Sheet::Shufrep => "shufrep.bmp",
            Sheet::Posbar => "posbar.bmp",
            Sheet::Volume => "volume.bmp",
            Sheet::Balance => "balance.bmp",
            Sheet::Monoster => "monoster.bmp",
            Sheet::Playpaus => "playpaus.bmp",
            Sheet::Numbers => "numbers.bmp",
            Sheet::NumsEx => "nums_ex.bmp",
            Sheet::Text => "text.bmp",
            Sheet::EqMain => "eqmain.bmp",
            Sheet::EqEx => "eq_ex.bmp",
            Sheet::PlEdit => "pledit.bmp",
            Sheet::Gen => "gen.bmp",
            Sheet::GenEx => "genex.bmp",
        }
    }

    /// Taille de référence de la planche (celle du skin de base) : le skin par défaut la respecte.
    pub fn reference_size(self) -> (u32, u32) {
        match self {
            Sheet::Main => (275, 116),
            Sheet::Titlebar => (344, 87),
            Sheet::Cbuttons => (136, 36),
            Sheet::Shufrep => (92, 85),
            Sheet::Posbar => (307, 10),
            Sheet::Volume => (68, 433),
            Sheet::Balance => (68, 433),
            Sheet::Monoster => (58, 24),
            Sheet::Playpaus => (42, 9),
            Sheet::Numbers => (99, 13),
            Sheet::NumsEx => (108, 13),
            Sheet::Text => (155, 18),
            Sheet::EqMain => (275, 315),
            Sheet::EqEx => (275, 56),
            Sheet::PlEdit => (280, 186),
            Sheet::Gen => (194, 109),
            Sheet::GenEx => (112, 59),
        }
    }
}

/// Un rectangle dans une planche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sprite {
    pub sheet: Sheet,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

const fn s(sheet: Sheet, x: i32, y: i32, w: i32, h: i32) -> Sprite {
    Sprite { sheet, x, y, w, h }
}

use Sheet::*;

// --- Fenêtre principale -------------------------------------------------------------------------

pub const MAIN_BACKGROUND: Sprite = s(Main, 0, 0, 275, 116);

pub const TITLE_BAR_ACTIVE: Sprite = s(Titlebar, 27, 0, 275, 14);
pub const TITLE_BAR_INACTIVE: Sprite = s(Titlebar, 27, 15, 275, 14);
pub const SHADE_BACKGROUND_ACTIVE: Sprite = s(Titlebar, 27, 29, 275, 14);
pub const SHADE_BACKGROUND_INACTIVE: Sprite = s(Titlebar, 27, 42, 275, 14);
pub const TITLE_BAR_EASTER_ACTIVE: Sprite = s(Titlebar, 27, 57, 275, 14);
pub const TITLE_BAR_EASTER_INACTIVE: Sprite = s(Titlebar, 27, 72, 275, 14);
pub const OPTIONS_BUTTON: Sprite = s(Titlebar, 0, 0, 9, 9);
pub const OPTIONS_BUTTON_DOWN: Sprite = s(Titlebar, 0, 9, 9, 9);
pub const MINIMIZE_BUTTON: Sprite = s(Titlebar, 9, 0, 9, 9);
pub const MINIMIZE_BUTTON_DOWN: Sprite = s(Titlebar, 9, 9, 9, 9);
pub const SHADE_BUTTON: Sprite = s(Titlebar, 0, 18, 9, 9);
pub const SHADE_BUTTON_DOWN: Sprite = s(Titlebar, 9, 18, 9, 9);
pub const UNSHADE_BUTTON: Sprite = s(Titlebar, 0, 27, 9, 9);
pub const UNSHADE_BUTTON_DOWN: Sprite = s(Titlebar, 9, 27, 9, 9);
pub const CLOSE_BUTTON: Sprite = s(Titlebar, 18, 0, 9, 9);
pub const CLOSE_BUTTON_DOWN: Sprite = s(Titlebar, 18, 9, 9, 9);
pub const CLUTTER_BAR: Sprite = s(Titlebar, 304, 0, 8, 43);
pub const CLUTTER_BAR_DISABLED: Sprite = s(Titlebar, 312, 0, 8, 43);
/// Lettres O, A, I, D, V enfoncées : (sprite, décalage vertical dans la clutterbar).
pub const CLUTTER_DOWN: [(Sprite, i32); 5] = [
    (s(Titlebar, 304, 47, 8, 8), 3),
    (s(Titlebar, 312, 55, 8, 7), 11),
    (s(Titlebar, 320, 62, 8, 7), 18),
    (s(Titlebar, 328, 69, 8, 8), 25),
    (s(Titlebar, 336, 77, 8, 7), 33),
];
pub const SHADE_POSITION_BACKGROUND: Sprite = s(Titlebar, 0, 36, 17, 7);
pub const SHADE_POSITION_THUMB_LEFT: Sprite = s(Titlebar, 17, 36, 3, 7);
pub const SHADE_POSITION_THUMB: Sprite = s(Titlebar, 20, 36, 3, 7);
pub const SHADE_POSITION_THUMB_RIGHT: Sprite = s(Titlebar, 23, 36, 3, 7);

pub const PREVIOUS: Sprite = s(Cbuttons, 0, 0, 23, 18);
pub const PREVIOUS_DOWN: Sprite = s(Cbuttons, 0, 18, 23, 18);
pub const PLAY: Sprite = s(Cbuttons, 23, 0, 23, 18);
pub const PLAY_DOWN: Sprite = s(Cbuttons, 23, 18, 23, 18);
pub const PAUSE: Sprite = s(Cbuttons, 46, 0, 23, 18);
pub const PAUSE_DOWN: Sprite = s(Cbuttons, 46, 18, 23, 18);
pub const STOP: Sprite = s(Cbuttons, 69, 0, 23, 18);
pub const STOP_DOWN: Sprite = s(Cbuttons, 69, 18, 23, 18);
pub const NEXT: Sprite = s(Cbuttons, 92, 0, 22, 18);
pub const NEXT_DOWN: Sprite = s(Cbuttons, 92, 18, 22, 18);
pub const EJECT: Sprite = s(Cbuttons, 114, 0, 22, 16);
pub const EJECT_DOWN: Sprite = s(Cbuttons, 114, 16, 22, 16);

pub const REPEAT: [Sprite; 4] = [
    s(Shufrep, 0, 0, 28, 15),
    s(Shufrep, 0, 15, 28, 15),
    s(Shufrep, 0, 30, 28, 15),
    s(Shufrep, 0, 45, 28, 15),
];
pub const SHUFFLE: [Sprite; 4] = [
    s(Shufrep, 28, 0, 47, 15),
    s(Shufrep, 28, 15, 47, 15),
    s(Shufrep, 28, 30, 47, 15),
    s(Shufrep, 28, 45, 47, 15),
];
/// Bouton EQ : [éteint, allumé, éteint enfoncé, allumé enfoncé].
pub const EQ_TOGGLE: [Sprite; 4] = [
    s(Shufrep, 0, 61, 23, 12),
    s(Shufrep, 0, 73, 23, 12),
    s(Shufrep, 46, 61, 23, 12),
    s(Shufrep, 46, 73, 23, 12),
];
/// Bouton PL : [éteint, allumé, éteint enfoncé, allumé enfoncé].
pub const PL_TOGGLE: [Sprite; 4] = [
    s(Shufrep, 23, 61, 23, 12),
    s(Shufrep, 23, 73, 23, 12),
    s(Shufrep, 69, 61, 23, 12),
    s(Shufrep, 69, 73, 23, 12),
];

pub const POSBAR_BACKGROUND: Sprite = s(Posbar, 0, 0, 248, 10);
pub const POSBAR_THUMB: Sprite = s(Posbar, 248, 0, 29, 10);
pub const POSBAR_THUMB_DOWN: Sprite = s(Posbar, 278, 0, 29, 10);

/// Fond n (0..27) du volume : 68x13 tous les 15 px.
pub const fn volume_background(frame: i32) -> Sprite {
    s(Volume, 0, frame * 15, 68, 13)
}
pub const VOLUME_THUMB: Sprite = s(Volume, 15, 422, 14, 11);
pub const VOLUME_THUMB_DOWN: Sprite = s(Volume, 0, 422, 14, 11);
/// Fond n (0..27) de la balance : 38x13 à partir de x = 9.
pub const fn balance_background(frame: i32) -> Sprite {
    s(Balance, 9, frame * 15, 38, 13)
}
pub const BALANCE_THUMB: Sprite = s(Balance, 15, 422, 14, 11);
pub const BALANCE_THUMB_DOWN: Sprite = s(Balance, 0, 422, 14, 11);

pub const STEREO_ON: Sprite = s(Monoster, 0, 0, 29, 12);
pub const STEREO_OFF: Sprite = s(Monoster, 0, 12, 29, 12);
pub const MONO_ON: Sprite = s(Monoster, 29, 0, 27, 12);
pub const MONO_OFF: Sprite = s(Monoster, 29, 12, 27, 12);

pub const STATUS_PLAYING: Sprite = s(Playpaus, 0, 0, 9, 9);
pub const STATUS_PAUSED: Sprite = s(Playpaus, 9, 0, 9, 9);
pub const STATUS_STOPPED: Sprite = s(Playpaus, 18, 0, 9, 9);
pub const STATUS_NONE: Sprite = s(Playpaus, 27, 0, 2, 9);
pub const STATUS_WORKING: Sprite = s(Playpaus, 36, 0, 3, 9);
pub const STATUS_NOT_WORKING: Sprite = s(Playpaus, 39, 0, 3, 9);

/// Chiffre d (0..9) dans `numbers.bmp` ou `nums_ex.bmp`.
pub const fn digit(sheet: Sheet, d: i32) -> Sprite {
    s(sheet, d * 9, 0, 9, 13)
}
pub const NUMBERS_MINUS: Sprite = s(Numbers, 20, 6, 5, 1);
pub const NUMBERS_NO_MINUS: Sprite = s(Numbers, 9, 6, 5, 1);
pub const NUMS_EX_BLANK: Sprite = s(NumsEx, 90, 0, 9, 13);
pub const NUMS_EX_MINUS: Sprite = s(NumsEx, 99, 0, 9, 13);

// --- Égaliseur ------------------------------------------------------------------------------------

pub const EQ_BACKGROUND: Sprite = s(EqMain, 0, 0, 275, 116);
pub const EQ_TITLE_ACTIVE: Sprite = s(EqMain, 0, 134, 275, 14);
pub const EQ_TITLE_INACTIVE: Sprite = s(EqMain, 0, 149, 275, 14);
pub const EQ_CLOSE: Sprite = s(EqMain, 0, 116, 9, 9);
pub const EQ_CLOSE_DOWN: Sprite = s(EqMain, 0, 125, 9, 9);
/// ON : [éteint, allumé, éteint enfoncé, allumé enfoncé].
pub const EQ_ON: [Sprite; 4] = [
    s(EqMain, 10, 119, 26, 12),
    s(EqMain, 69, 119, 26, 12),
    s(EqMain, 128, 119, 26, 12),
    s(EqMain, 187, 119, 26, 12),
];
/// AUTO : [éteint, allumé, éteint enfoncé, allumé enfoncé].
pub const EQ_AUTO: [Sprite; 4] = [
    s(EqMain, 36, 119, 32, 12),
    s(EqMain, 95, 119, 32, 12),
    s(EqMain, 154, 119, 32, 12),
    s(EqMain, 213, 119, 32, 12),
];
pub const EQ_PRESETS: Sprite = s(EqMain, 224, 164, 44, 12);
pub const EQ_PRESETS_DOWN: Sprite = s(EqMain, 224, 176, 44, 12);
pub const EQ_SLIDER_THUMB: Sprite = s(EqMain, 0, 164, 11, 11);
pub const EQ_SLIDER_THUMB_DOWN: Sprite = s(EqMain, 0, 176, 11, 11);
/// Fond n (0..27) d'un curseur d'égaliseur (14x63) : n = 27 − pos·28/64, où pos 0 = +12 dB et 63 = −12 dB.
pub const fn eq_slider_background(n: i32) -> Sprite {
    if n < 14 { s(EqMain, 13 + 15 * n, 164, 14, 63) } else { s(EqMain, 13 + 15 * (n - 14), 229, 14, 63) }
}
pub const EQ_GRAPH_BACKGROUND: Sprite = s(EqMain, 0, 294, 113, 19);
pub const EQ_GRAPH_LINE_COLORS: Sprite = s(EqMain, 115, 294, 1, 19);
pub const EQ_PREAMP_LINE: Sprite = s(EqMain, 0, 314, 113, 1);
/// Partie du titre utilisée quand `eq_ex.bmp` manque pour le bouton « déplier ».
pub const EQ_MAXIMIZE_FALLBACK: Sprite = s(EqMain, 254, 152, 9, 9);

pub const EQ_SHADE_ACTIVE: Sprite = s(EqEx, 0, 0, 275, 14);
pub const EQ_SHADE_INACTIVE: Sprite = s(EqEx, 0, 15, 275, 14);
pub const EQ_SHADE_VOLUME_THUMB: [Sprite; 3] = [s(EqEx, 1, 30, 3, 7), s(EqEx, 4, 30, 3, 7), s(EqEx, 7, 30, 3, 7)];
pub const EQ_SHADE_BALANCE_THUMB: [Sprite; 3] = [s(EqEx, 11, 30, 3, 7), s(EqEx, 14, 30, 3, 7), s(EqEx, 17, 30, 3, 7)];
pub const EQ_MAXIMIZE_DOWN: Sprite = s(EqEx, 1, 38, 9, 9);
pub const EQ_MINIMIZE_DOWN: Sprite = s(EqEx, 1, 47, 9, 9);
pub const EQ_SHADE_CLOSE: Sprite = s(EqEx, 11, 38, 9, 9);
pub const EQ_SHADE_CLOSE_DOWN: Sprite = s(EqEx, 11, 47, 9, 9);

// --- Playlist -------------------------------------------------------------------------------------

pub const PL_TOP_LEFT_ACTIVE: Sprite = s(PlEdit, 0, 0, 25, 20);
pub const PL_TITLE_ACTIVE: Sprite = s(PlEdit, 26, 0, 100, 20);
pub const PL_TOP_TILE_ACTIVE: Sprite = s(PlEdit, 127, 0, 25, 20);
pub const PL_TOP_RIGHT_ACTIVE: Sprite = s(PlEdit, 153, 0, 25, 20);
pub const PL_TOP_LEFT_INACTIVE: Sprite = s(PlEdit, 0, 21, 25, 20);
pub const PL_TITLE_INACTIVE: Sprite = s(PlEdit, 26, 21, 100, 20);
pub const PL_TOP_TILE_INACTIVE: Sprite = s(PlEdit, 127, 21, 25, 20);
pub const PL_TOP_RIGHT_INACTIVE: Sprite = s(PlEdit, 153, 21, 25, 20);
pub const PL_LEFT_TILE: Sprite = s(PlEdit, 0, 42, 12, 29);
pub const PL_RIGHT_TILE: Sprite = s(PlEdit, 31, 42, 20, 29);
pub const PL_BOTTOM_LEFT: Sprite = s(PlEdit, 0, 72, 125, 38);
pub const PL_BOTTOM_RIGHT: Sprite = s(PlEdit, 126, 72, 150, 38);
pub const PL_BOTTOM_TILE: Sprite = s(PlEdit, 179, 0, 25, 38);
pub const PL_VISUALIZER: Sprite = s(PlEdit, 205, 0, 75, 38);
pub const PL_SCROLL_THUMB: Sprite = s(PlEdit, 52, 53, 8, 18);
pub const PL_SCROLL_THUMB_DOWN: Sprite = s(PlEdit, 61, 53, 8, 18);
pub const PL_SHADE_LEFT: Sprite = s(PlEdit, 72, 42, 25, 14);
pub const PL_SHADE_TILE: Sprite = s(PlEdit, 72, 57, 25, 14);
pub const PL_SHADE_RIGHT_ACTIVE: Sprite = s(PlEdit, 99, 42, 50, 14);
pub const PL_SHADE_RIGHT_INACTIVE: Sprite = s(PlEdit, 99, 57, 50, 14);
pub const PL_CLOSE_DOWN: Sprite = s(PlEdit, 52, 42, 9, 9);
pub const PL_SHADE_DOWN: Sprite = s(PlEdit, 62, 42, 9, 9);
pub const PL_UNSHADE_DOWN: Sprite = s(PlEdit, 150, 42, 9, 9);

/// Boutons des menus en bande de la playlist : (normal, enfoncé).
pub mod pl_menu {
    use super::*;
    pub const ADD_URL: (Sprite, Sprite) = (s(PlEdit, 0, 111, 22, 18), s(PlEdit, 23, 111, 22, 18));
    pub const ADD_DIR: (Sprite, Sprite) = (s(PlEdit, 0, 130, 22, 18), s(PlEdit, 23, 130, 22, 18));
    pub const ADD_FILE: (Sprite, Sprite) = (s(PlEdit, 0, 149, 22, 18), s(PlEdit, 23, 149, 22, 18));
    pub const REM_ALL: (Sprite, Sprite) = (s(PlEdit, 54, 111, 22, 18), s(PlEdit, 77, 111, 22, 18));
    pub const REM_CROP: (Sprite, Sprite) = (s(PlEdit, 54, 130, 22, 18), s(PlEdit, 77, 130, 22, 18));
    pub const REM_SEL: (Sprite, Sprite) = (s(PlEdit, 54, 149, 22, 18), s(PlEdit, 77, 149, 22, 18));
    pub const REM_MISC: (Sprite, Sprite) = (s(PlEdit, 54, 168, 22, 18), s(PlEdit, 77, 168, 22, 18));
    pub const SEL_INV: (Sprite, Sprite) = (s(PlEdit, 104, 111, 22, 18), s(PlEdit, 127, 111, 22, 18));
    pub const SEL_ZERO: (Sprite, Sprite) = (s(PlEdit, 104, 130, 22, 18), s(PlEdit, 127, 130, 22, 18));
    pub const SEL_ALL: (Sprite, Sprite) = (s(PlEdit, 104, 149, 22, 18), s(PlEdit, 127, 149, 22, 18));
    pub const MISC_SORT: (Sprite, Sprite) = (s(PlEdit, 154, 111, 22, 18), s(PlEdit, 177, 111, 22, 18));
    pub const MISC_INFO: (Sprite, Sprite) = (s(PlEdit, 154, 130, 22, 18), s(PlEdit, 177, 130, 22, 18));
    pub const MISC_OPTS: (Sprite, Sprite) = (s(PlEdit, 154, 149, 22, 18), s(PlEdit, 177, 149, 22, 18));
    pub const LIST_NEW: (Sprite, Sprite) = (s(PlEdit, 204, 111, 22, 18), s(PlEdit, 227, 111, 22, 18));
    pub const LIST_SAVE: (Sprite, Sprite) = (s(PlEdit, 204, 130, 22, 18), s(PlEdit, 227, 130, 22, 18));
    pub const LIST_LOAD: (Sprite, Sprite) = (s(PlEdit, 204, 149, 22, 18), s(PlEdit, 227, 149, 22, 18));
    pub const ADD_BAR: Sprite = s(PlEdit, 48, 111, 3, 54);
    pub const REM_BAR: Sprite = s(PlEdit, 100, 111, 3, 72);
    pub const SEL_BAR: Sprite = s(PlEdit, 150, 111, 3, 54);
    pub const MISC_BAR: Sprite = s(PlEdit, 200, 111, 3, 54);
    pub const LIST_BAR: Sprite = s(PlEdit, 250, 111, 3, 54);
}

// --- Fenêtres génériques (gen.bmp) ---------------------------------------------------------------

pub mod generic {
    use super::*;
    pub const TOP_LEFT: [Sprite; 2] = [s(Gen, 0, 21, 25, 20), s(Gen, 0, 0, 25, 20)];
    pub const TOP_LEFT_END: [Sprite; 2] = [s(Gen, 26, 21, 25, 20), s(Gen, 26, 0, 25, 20)];
    pub const TOP_CENTER_FILL: [Sprite; 2] = [s(Gen, 52, 21, 25, 20), s(Gen, 52, 0, 25, 20)];
    pub const TOP_RIGHT_END: [Sprite; 2] = [s(Gen, 78, 21, 25, 20), s(Gen, 78, 0, 25, 20)];
    pub const TOP_LEFT_RIGHT_FILL: [Sprite; 2] = [s(Gen, 104, 21, 25, 20), s(Gen, 104, 0, 25, 20)];
    pub const TOP_RIGHT: [Sprite; 2] = [s(Gen, 130, 21, 25, 20), s(Gen, 130, 0, 25, 20)];
    pub const BOTTOM_LEFT: Sprite = s(Gen, 0, 42, 125, 14);
    pub const BOTTOM_RIGHT: Sprite = s(Gen, 0, 57, 125, 14);
    pub const BOTTOM_FILL: Sprite = s(Gen, 127, 72, 25, 14);
    pub const MIDDLE_LEFT: Sprite = s(Gen, 127, 42, 11, 29);
    pub const MIDDLE_LEFT_BOTTOM: Sprite = s(Gen, 158, 42, 11, 24);
    pub const MIDDLE_RIGHT: Sprite = s(Gen, 139, 42, 8, 29);
    pub const MIDDLE_RIGHT_BOTTOM: Sprite = s(Gen, 170, 42, 8, 24);
    pub const CLOSE_DOWN: Sprite = s(Gen, 148, 42, 9, 9);
    /// Bouton des fenêtres génériques (genex.bmp), normal et enfoncé.
    pub const BUTTON: [Sprite; 2] = [s(GenEx, 0, 0, 47, 15), s(GenEx, 0, 15, 47, 15)];
    pub const SCROLL_UP: [Sprite; 2] = [s(GenEx, 0, 31, 14, 14), s(GenEx, 28, 31, 14, 14)];
    pub const SCROLL_DOWN: [Sprite; 2] = [s(GenEx, 14, 31, 14, 14), s(GenEx, 42, 31, 14, 14)];
    pub const SCROLL_THUMB: [Sprite; 2] = [s(GenEx, 56, 31, 14, 28), s(GenEx, 70, 31, 14, 28)];
}
