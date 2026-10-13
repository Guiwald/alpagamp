//! Raccourcis clavier classiques (Z X C V B, flèches…). Sur macOS, Ctrl devient Cmd et Alt devient Option
//! (ADR 0014). Les lettres sont lues sans les modificateurs (Option+G donne « g », pas « © »).

use alpagamp_ui::GenKey;
use winit::event::KeyEvent;
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

use crate::command::Command;

/// Fenêtre qui reçoit le clavier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Main,
    Eq,
    Playlist,
}

/// La touche « Ctrl » de Winamp : Cmd sur macOS.
fn primary(m: ModifiersState) -> bool {
    if cfg!(target_os = "macos") { m.super_key() } else { m.control_key() }
}

fn bare_key(ev: &KeyEvent) -> Key {
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        ev.key_without_modifiers()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        ev.logical_key.clone()
    }
}

pub fn map(ev: &KeyEvent, m: ModifiersState, focus: Focus) -> Option<Command> {
    use Command::*;
    let shift = m.shift_key();
    let ctrl = primary(m);
    let alt = m.alt_key();
    let code = match ev.physical_key {
        PhysicalKey::Code(c) => Some(c),
        _ => None,
    };
    // Pavé numérique et chiffres (codes physiques, indépendants de la disposition du clavier).
    if let Some(code) = code {
        let np = match code {
            KeyCode::Numpad4 => Some(Previous),
            KeyCode::Numpad5 => Some(Play),
            KeyCode::Numpad6 => Some(Next),
            KeyCode::Numpad7 => Some(Back5),
            KeyCode::Numpad9 => Some(Fwd5),
            KeyCode::Numpad8 => Some(VolumeUp),
            KeyCode::Numpad2 => Some(VolumeDown),
            KeyCode::Numpad1 => Some(Back10Tracks),
            KeyCode::Numpad3 => Some(Fwd10Tracks),
            KeyCode::Numpad0 if !ctrl => Some(OpenFiles),
            KeyCode::Digit3 if alt && !ctrl && focus == Focus::Playlist => Some(PlFileInfo),
            KeyCode::Digit3 if alt && !ctrl => Some(FileInfo),
            _ => None,
        };
        if np.is_some() {
            return np;
        }
    }
    let key = bare_key(ev);
    // Touches propres à la playlist.
    if focus == Focus::Playlist {
        let pl = match &key {
            Key::Named(NamedKey::ArrowUp) if alt => Some(PlMoveSelection(-1)),
            Key::Named(NamedKey::ArrowDown) if alt => Some(PlMoveSelection(1)),
            Key::Named(NamedKey::ArrowUp) => Some(PlCursor(-1, shift)),
            Key::Named(NamedKey::ArrowDown) => Some(PlCursor(1, shift)),
            Key::Named(NamedKey::PageUp) => Some(PlCursor(-10, shift)),
            Key::Named(NamedKey::PageDown) => Some(PlCursor(10, shift)),
            Key::Named(NamedKey::Home) => Some(PlCursor(-1_000_000, shift)),
            Key::Named(NamedKey::End) => Some(PlCursor(1_000_000, shift)),
            Key::Named(NamedKey::Enter) => Some(PlPlaySelected),
            Key::Named(NamedKey::Delete) => Some(PlRemoveSelected),
            Key::Named(NamedKey::Backspace) if cfg!(target_os = "macos") => Some(PlRemoveSelected),
            Key::Character(c) if ctrl => match c.to_lowercase().as_str() {
                "a" if !shift => Some(PlSelectAll),
                "a" => Some(PlSelectNone),
                "i" => Some(PlInvertSelection),
                "n" => Some(PlNew),
                "o" => Some(PlLoad),
                "s" => Some(PlSave),
                _ => None,
            },
            _ => None,
        };
        if pl.is_some() {
            return pl;
        }
    }
    match &key {
        Key::Named(NamedKey::ArrowLeft) => Some(Back5),
        Key::Named(NamedKey::ArrowRight) => Some(Fwd5),
        Key::Named(NamedKey::ArrowUp) => Some(VolumeUp),
        Key::Named(NamedKey::ArrowDown) => Some(VolumeDown),
        Key::Named(NamedKey::Insert) => Some(OpenFolder),
        Key::Character(c) => {
            let c = c.to_lowercase();
            match (c.as_str(), ctrl, shift, alt) {
                ("z", false, false, false) => Some(Previous),
                ("z", true, false, false) => Some(First),
                ("z", false, true, false) => Some(Back5),
                ("x", false, false, false) => Some(Play),
                ("x", false, true, false) => Some(OpenFiles),
                ("c", false, false, false) => Some(Pause),
                ("v", false, false, false) => Some(Stop),
                ("v", false, true, false) => Some(StopFade),
                ("v", true, false, false) => Some(StopAfterCurrent),
                ("b", false, false, false) => Some(Next),
                ("b", true, false, false) => Some(Last),
                ("b", false, true, false) => Some(Fwd5),
                ("l", false, false, false) => Some(OpenFiles),
                ("l", false, true, false) => Some(OpenFolder),
                ("l", true, false, false) => Some(PlAddUrl),
                ("j", false, false, false) => Some(JumpToFile),
                ("r", false, false, false) => Some(ToggleRepeat),
                ("r", false, true, false) => Some(ToggleManualAdvance),
                ("s", false, false, false) => Some(ToggleShuffle),
                ("s", false, false, true) => Some(LoadSkin),
                ("g", false, false, true) => Some(ToggleEqWindow),
                ("e", false, false, true) => Some(TogglePlaylistWindow),
                ("b", true, false, true) => Some(AddBookmark),
                ("d", true, false, false) => Some(ToggleDoubleSize),
                ("t", true, false, false) => Some(ToggleTimeMode),
                ("a", true, false, false) => Some(ToggleAlwaysOnTop),
                // Bandeau : Ctrl+W (Cmd+Maj+W sur macOS, Cmd+W y fermant la fenêtre).
                ("w", true, false, false) if !cfg!(target_os = "macos") => Some(ToggleShade),
                ("w", true, true, false) if cfg!(target_os = "macos") => Some(ToggleShade),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Clavier d'une fenêtre générique (champ de saisie, liste).
pub fn gen_key(ev: &KeyEvent, m: ModifiersState) -> Option<GenKey> {
    let k = match &ev.logical_key {
        Key::Named(NamedKey::Backspace) => GenKey::Backspace,
        Key::Named(NamedKey::Delete) => GenKey::Delete,
        Key::Named(NamedKey::ArrowLeft) => GenKey::Left,
        Key::Named(NamedKey::ArrowRight) => GenKey::Right,
        Key::Named(NamedKey::ArrowUp) => GenKey::Up,
        Key::Named(NamedKey::ArrowDown) => GenKey::Down,
        Key::Named(NamedKey::PageUp) => GenKey::PageUp,
        Key::Named(NamedKey::PageDown) => GenKey::PageDown,
        Key::Named(NamedKey::Home) => GenKey::Home,
        Key::Named(NamedKey::End) => GenKey::End,
        Key::Named(NamedKey::Enter) => GenKey::Enter,
        Key::Named(NamedKey::Escape) => GenKey::Escape,
        _ => {
            if primary(m) || m.control_key() {
                return None;
            }
            let t = ev.text.as_ref()?.to_string();
            if t.chars().all(|c| c.is_control()) {
                return None;
            }
            GenKey::Text(t)
        }
    };
    Some(k)
}
