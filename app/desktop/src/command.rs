//! Commandes de l'application (venant des fenêtres, du clavier ou des menus).

use std::path::PathBuf;

use alpagamp_core::i18n::Lang;
use alpagamp_core::settings::VisMode;

#[derive(Debug, Clone, PartialEq)]
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
    AddFiles,
    LoadSkin,
    DefaultSkin,
    SkinFile(PathBuf),
    ToggleShuffle,
    ToggleRepeat,
    ToggleManualAdvance,
    ToggleTimeMode,
    ToggleDoubleSize,
    ToggleAlwaysOnTop,
    ToggleShade,
    Vis(VisMode),
    FileInfo,
    About,
    Exit,
    Minimize,
    Language(Option<Lang>),
}
