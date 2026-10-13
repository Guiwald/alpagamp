//! Menus natifs (macOS, Windows) : menu contextuel et barre de menus macOS.

use std::path::{Path, PathBuf};

use alpagamp_core::Settings;
use alpagamp_core::i18n::{Lang, t};
use alpagamp_core::settings::VisMode;
use muda::{CheckMenuItem, ContextMenu, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::command::Command;

pub struct Menus {
    context: Menu,
    #[cfg(target_os = "macos")]
    _bar: Menu,
    skins: Vec<PathBuf>,
}

fn item(id: &str, key: &str) -> MenuItem {
    MenuItem::with_id(id, t(key), true, None)
}

fn check(id: &str, key: &str, on: bool) -> CheckMenuItem {
    CheckMenuItem::with_id(id, t(key), true, on, None)
}

fn sep() -> PredefinedMenuItem {
    PredefinedMenuItem::separator()
}

impl Menus {
    pub fn new(settings: &Settings, skins_dir: &Path) -> Menus {
        let skins = list_skins(skins_dir);
        let context = build_context(settings, &skins);
        #[cfg(target_os = "macos")]
        let bar = {
            let bar = Menu::new();
            let app = Submenu::with_items(
                "Alpagamp",
                true,
                &[&item("about", "menu.about"), &sep(), &PredefinedMenuItem::hide(None), &PredefinedMenuItem::hide_others(None), &sep(), &PredefinedMenuItem::quit(None)],
            )
            .expect("menu");
            let _ = bar.append(&app);
            let file = Submenu::with_items("File", true, &[&item("play_file", "menu.play_file"), &item("play_folder", "menu.play_folder")]).expect("menu");
            let _ = bar.append(&file);
            let _ = bar.append(&playback_submenu());
            let _ = bar.append(&options_submenu(settings));
            let _ = bar.append(&skins_submenu(&skins));
            let _ = bar.append(&vis_submenu(settings));
            bar.init_for_nsapp();
            bar
        };
        Menus {
            context,
            #[cfg(target_os = "macos")]
            _bar: bar,
            skins,
        }
    }

    /// Reconstruit après un changement de réglage (coches) ou de langue.
    pub fn refresh(&mut self, settings: &Settings, skins_dir: &Path) {
        *self = Menus::new(settings, skins_dir);
    }

    /// Affiche le menu contextuel sous la souris.
    pub fn popup(&self, window: &Window) {
        let Ok(handle) = window.window_handle() else { return };
        match handle.as_raw() {
            #[cfg(target_os = "macos")]
            RawWindowHandle::AppKit(h) => unsafe {
                self.context.show_context_menu_for_nsview(h.ns_view.as_ptr() as _, None);
            },
            #[cfg(target_os = "windows")]
            RawWindowHandle::Win32(h) => unsafe {
                self.context.show_context_menu_for_hwnd(h.hwnd.get() as _, None);
            },
            _ => {}
        }
    }

    /// Commandes choisies dans les menus depuis le dernier appel.
    pub fn poll(&self) -> Vec<Command> {
        let mut out = Vec::new();
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            if let Some(c) = self.command_for(ev.id.as_ref()) {
                out.push(c);
            }
        }
        out
    }

    fn command_for(&self, id: &str) -> Option<Command> {
        use Command::*;
        Some(match id {
            "play_file" => OpenFiles,
            "play_folder" => OpenFolder,
            "file_info" => FileInfo,
            "previous" => Previous,
            "play" => Play,
            "pause" => Pause,
            "stop" => Stop,
            "next" => Next,
            "stop_fade" => StopFade,
            "stop_after" => StopAfterCurrent,
            "back5" => Back5,
            "fwd5" => Fwd5,
            "on_top" => ToggleAlwaysOnTop,
            "double" => ToggleDoubleSize,
            "remaining" => ToggleTimeMode,
            "repeat" => ToggleRepeat,
            "shuffle" => ToggleShuffle,
            "manual" => ToggleManualAdvance,
            "load_skin" => LoadSkin,
            "default_skin" => DefaultSkin,
            "vis_analyzer" => Vis(VisMode::Analyzer),
            "vis_scope" => Vis(VisMode::Oscilloscope),
            "vis_off" => Vis(VisMode::Off),
            "about" => About,
            "exit" => Exit,
            "lang_system" => Language(None),
            other => {
                if let Some(code) = other.strip_prefix("lang_") {
                    return Lang::ALL.into_iter().find(|l| l.code() == code).map(|l| Language(Some(l)));
                }
                if let Some(i) = other.strip_prefix("skin_").and_then(|n| n.parse::<usize>().ok()) {
                    return self.skins.get(i).cloned().map(SkinFile);
                }
                return None;
            }
        })
    }
}

fn playback_submenu() -> Submenu {
    Submenu::with_items(
        t("menu.playback"),
        true,
        &[
            &item("previous", "menu.previous"),
            &item("play", "menu.play"),
            &item("pause", "menu.pause"),
            &item("stop", "menu.stop"),
            &item("next", "menu.next"),
            &sep(),
            &item("stop_fade", "menu.stop_fadeout"),
            &item("stop_after", "menu.stop_after_current"),
            &sep(),
            &item("back5", "menu.back5"),
            &item("fwd5", "menu.fwd5"),
        ],
    )
    .expect("menu")
}

fn options_submenu(s: &Settings) -> Submenu {
    let lang = Submenu::new(t("menu.language"), true);
    let _ = lang.append(&check("lang_system", "menu.system_language", s.language.is_none()));
    let _ = lang.append(&sep());
    for l in Lang::ALL {
        let on = s.language.as_deref() == Some(l.code());
        let _ = lang.append(&CheckMenuItem::with_id(format!("lang_{}", l.code()), l.native_name(), true, on, None));
    }
    Submenu::with_items(
        t("menu.options"),
        true,
        &[
            &check("on_top", "menu.always_on_top", s.always_on_top),
            &check("double", "menu.double_size", s.double_size),
            &check("remaining", "menu.time_remaining", s.time_remaining),
            &sep(),
            &check("repeat", "menu.repeat", s.repeat),
            &check("shuffle", "menu.shuffle", s.shuffle),
            &check("manual", "menu.manual_advance", s.manual_advance),
            &sep(),
            &lang,
        ],
    )
    .expect("menu")
}

fn skins_submenu(skins: &[PathBuf]) -> Submenu {
    let m = Submenu::new(t("menu.skins"), true);
    let _ = m.append(&item("load_skin", "menu.load_skin"));
    let _ = m.append(&item("default_skin", "menu.default_skin"));
    if !skins.is_empty() {
        let _ = m.append(&sep());
        for (i, p) in skins.iter().enumerate() {
            let name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let _ = m.append(&MenuItem::with_id(format!("skin_{i}"), name, true, None));
        }
    }
    m
}

fn vis_submenu(s: &Settings) -> Submenu {
    Submenu::with_items(
        t("menu.visualization"),
        true,
        &[
            &check("vis_analyzer", "menu.analyzer", s.vis.mode == VisMode::Analyzer),
            &check("vis_scope", "menu.oscilloscope", s.vis.mode == VisMode::Oscilloscope),
            &check("vis_off", "menu.vis_off", s.vis.mode == VisMode::Off),
        ],
    )
    .expect("menu")
}

fn build_context(s: &Settings, skins: &[PathBuf]) -> Menu {
    let m = Menu::new();
    let _ = m.append(&item("about", "menu.about"));
    let _ = m.append(&sep());
    let _ = m.append(&item("play_file", "menu.play_file"));
    let _ = m.append(&item("play_folder", "menu.play_folder"));
    let _ = m.append(&item("file_info", "menu.file_info"));
    let _ = m.append(&sep());
    let _ = m.append(&playback_submenu());
    let _ = m.append(&options_submenu(s));
    let _ = m.append(&skins_submenu(skins));
    let _ = m.append(&vis_submenu(s));
    let _ = m.append(&sep());
    let _ = m.append(&item("exit", "menu.exit"));
    m
}

/// Skins `.wsz` / `.zip` rangés dans le dossier des skins.
pub fn list_skins(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| matches!(p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("wsz" | "zip")))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}
