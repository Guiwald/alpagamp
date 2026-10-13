//! Commandes de l'application (venant des fenêtres, du clavier ou des menus).

use std::path::PathBuf;

use alpagamp_core::i18n::Lang;
use alpagamp_core::playlist::SortKey;
use alpagamp_core::settings::{AnalyzerStyle, ReplayGainMode, ScopeStyle, VisMode};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(not(any(target_os = "macos", target_os = "windows")), allow(dead_code))]
pub enum Command {
    Previous,
    Play,
    Pause,
    Stop,
    StopFade,
    StopAfterCurrent,
    Next,
    First,
    Last,
    Back5,
    Fwd5,
    Back10Tracks,
    Fwd10Tracks,
    VolumeUp,
    VolumeDown,
    OpenFiles,
    OpenFolder,
    LoadSkin,
    DefaultSkin,
    SkinFile(PathBuf),
    ToggleShuffle,
    ToggleRepeat,
    ToggleManualAdvance,
    ToggleTimeMode,
    ToggleDoubleSize,
    ToggleAlwaysOnTop,
    /// Bandeau de la fenêtre qui a le focus (ou de la principale).
    ToggleShade,
    Vis(VisMode),
    FileInfo,
    About,
    Exit,
    Minimize,
    Language(Option<Lang>),
    // --- Fenêtres ---
    ToggleEqWindow,
    TogglePlaylistWindow,
    ToggleLibrary,
    SpotifyLogin,
    SpotifyLogout,
    JumpToFile,
    // --- Égaliseur ---
    ToggleEq,
    ToggleEqAuto,
    EqPreset(usize),
    EqLoadFile,
    EqSaveFile,
    EqSaveAuto,
    EqDeleteAuto,
    EqReset,
    // --- Playlist ---
    PlAddFiles,
    PlAddFolder,
    PlAddUrl,
    PlRemoveSelected,
    PlCrop,
    PlRemoveAll,
    PlRemoveMissing,
    PlRemoveDuplicates,
    PlSelectAll,
    PlSelectNone,
    PlInvertSelection,
    PlSort(SortKey),
    PlReverse,
    PlRandomize,
    PlNew,
    PlSave,
    PlLoad,
    PlFileInfo,
    /// Jouer la ligne qui a le focus (ou la première sélectionnée).
    PlPlaySelected,
    /// Focus clavier : déplacer de n lignes (avec Maj, étendre la sélection).
    PlCursor(isize, bool),
    /// Déplacer la sélection (Alt+↑/↓).
    PlMoveSelection(isize),
    // --- Lecture ---
    Crossfade(u32),
    ReplayGain(ReplayGainMode),
    RgPreamp(i32),
    ToggleRgClipping,
    // --- Signets ---
    AddBookmark,
    Bookmark(usize),
    RemoveBookmark(usize),
    ClearBookmarks,
    // --- Options du visualiseur ---
    VisAnalyzerStyle(AnalyzerStyle),
    VisThickBands(bool),
    ToggleVisPeaks,
    VisFalloff(u8),
    VisPeakFalloff(u8),
    VisScopeStyle(ScopeStyle),
    VisRefresh(u8),
    // --- Fenêtre de visualisation (presets MilkDrop) ---
    ToggleMilk,
    /// Preset suivant (`true` : sans fondu).
    MilkNext(bool),
    MilkPrevious,
    MilkToggleShuffle,
    MilkToggleLock,
    /// « Choisir un preset ».
    MilkPresets,
    MilkFullscreen,
    MilkLeaveFullscreen,
    /// Changement automatique toutes les n secondes (0 : jamais).
    MilkAuto(u32),
    /// Durée du fondu (dixièmes de seconde, 0 : coupe franche).
    MilkBlend(u32),
    MilkDownloadPack,
    MilkOpenFolder,
    MilkRescan,
    /// Navigateur de skins (Alt+S).
    ToggleSkins,
}

/// Les menus surgissants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Popup {
    /// Le menu principal (clic droit, bouton d'options).
    Main,
    EqPresets,
    PlRemoveMisc,
    PlSort,
    PlMisc,
    /// Clic droit dans la liste.
    PlContext,
    /// Clic droit dans la fenêtre de visualisation.
    Milk,
}
