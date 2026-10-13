//! Raccourcis clavier classiques (Z X C V B, flèches…). Sur macOS, Ctrl devient Cmd et Alt devient Option
//! (ADR 0014).

use winit::event::KeyEvent;
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

use crate::command::Command;

/// La touche « Ctrl » de Winamp : Cmd sur macOS.
fn primary(m: ModifiersState) -> bool {
    if cfg!(target_os = "macos") { m.super_key() } else { m.control_key() }
}

pub fn map(ev: &KeyEvent, m: ModifiersState) -> Option<Command> {
    use Command::*;
    let shift = m.shift_key();
    let ctrl = primary(m);
    let alt = m.alt_key();
    // Pavé numérique (codes physiques).
    if let PhysicalKey::Code(code) = ev.physical_key {
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
            _ => None,
        };
        if np.is_some() {
            return np;
        }
    }
    match &ev.logical_key {
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
                ("r", false, false, false) => Some(ToggleRepeat),
                ("r", false, true, false) => Some(ToggleManualAdvance),
                ("s", false, false, false) => Some(ToggleShuffle),
                ("s", false, false, true) => Some(LoadSkin),
                ("d", true, false, false) => Some(ToggleDoubleSize),
                ("t", true, false, false) => Some(ToggleTimeMode),
                ("a", true, false, false) => Some(ToggleAlwaysOnTop),
                // Bandeau : Ctrl+W (Cmd+Maj+W sur macOS, Cmd+W y fermant la fenêtre).
                ("w", true, false, false) if !cfg!(target_os = "macos") => Some(ToggleShade),
                ("w", true, true, false) if cfg!(target_os = "macos") => Some(ToggleShade),
                ("3", false, false, true) => Some(FileInfo),
                _ => None,
            }
        }
        _ => None,
    }
}
