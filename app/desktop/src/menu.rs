//! Menus natifs (macOS, Windows) : menu principal (clic droit), barre de menus macOS et menus surgissants
//! (préréglages de l'égaliseur, menus de la playlist).

use std::path::{Path, PathBuf};

use alpagamp_core::Settings;
use alpagamp_core::eq::factory_presets;
use alpagamp_core::i18n::{Lang, t};
use alpagamp_core::playlist::SortKey;
use alpagamp_core::settings::{AnalyzerStyle, ReplayGainMode, ScopeStyle, VisMode};
use muda::{CheckMenuItem, ContextMenu, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::command::{Command, Popup};

/// État utile pour construire les menus.
pub struct MenuState<'a> {
    pub settings: &'a Settings,
    pub eq_open: bool,
    pub pl_open: bool,
    /// Un préréglage automatique existe pour le morceau courant.
    pub has_auto_preset: bool,
    /// Titres des signets.
    pub bookmarks: Vec<String>,
    pub library_open: bool,
    pub spotify_connected: bool,
}

pub struct Menus {
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
    pub fn new(st: &MenuState, skins_dir: &Path) -> Menus {
        let skins = list_skins(skins_dir);
        #[cfg(not(target_os = "macos"))]
        let _ = st;
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
            let _ = bar.append(&file_submenu());
            let _ = bar.append(&playback_submenu());
            let _ = bar.append(&bookmarks_submenu(st));
            let _ = bar.append(&windows_submenu(st));
            let _ = bar.append(&options_submenu(st.settings));
            let _ = bar.append(&skins_submenu(&skins));
            let _ = bar.append(&vis_submenu(st.settings));
            bar.init_for_nsapp();
            bar
        };
        Menus {
            #[cfg(target_os = "macos")]
            _bar: bar,
            skins,
        }
    }

    /// Affiche un menu sous la souris, au-dessus de `window`.
    pub fn popup(&self, which: Popup, st: &MenuState, window: &Window) {
        let menu = match which {
            Popup::Main => build_main(st, &self.skins),
            Popup::EqPresets => build_eq_presets(st),
            Popup::PlRemoveMisc => build_list(&[("pl_remove_missing", "pl.remove_missing"), ("pl_remove_dups", "pl.remove_duplicates")]),
            Popup::PlSort => build_sort(),
            Popup::PlMisc => build_list(&[("jump", "menu.jump_to_file"), ("pl_info", "menu.file_info")]),
            Popup::PlContext => build_pl_context(),
        };
        show(&menu, window);
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
            "win_eq" => ToggleEqWindow,
            "win_pl" => TogglePlaylistWindow,
            "win_lib" => ToggleLibrary,
            "spotify_login" => SpotifyLogin,
            "spotify_logout" => SpotifyLogout,
            "jump" => JumpToFile,
            "eq_on" => ToggleEq,
            "eq_auto" => ToggleEqAuto,
            "eq_load_file" => EqLoadFile,
            "eq_save_file" => EqSaveFile,
            "eq_save_auto" => EqSaveAuto,
            "eq_delete_auto" => EqDeleteAuto,
            "eq_reset" => EqReset,
            "pl_add_files" => PlAddFiles,
            "pl_add_folder" => PlAddFolder,
            "pl_add_url" => PlAddUrl,
            "pl_remove_sel" => PlRemoveSelected,
            "pl_crop" => PlCrop,
            "pl_remove_all" => PlRemoveAll,
            "pl_remove_missing" => PlRemoveMissing,
            "pl_remove_dups" => PlRemoveDuplicates,
            "pl_select_all" => PlSelectAll,
            "pl_select_none" => PlSelectNone,
            "pl_invert" => PlInvertSelection,
            "pl_sort_title" => PlSort(SortKey::Title),
            "pl_sort_file" => PlSort(SortKey::FileName),
            "pl_sort_path" => PlSort(SortKey::Path),
            "pl_reverse" => PlReverse,
            "pl_randomize" => PlRandomize,
            "pl_new" => PlNew,
            "pl_save" => PlSave,
            "pl_load" => PlLoad,
            "pl_info" => PlFileInfo,
            "pl_play" => PlPlaySelected,
            "rg_off" => ReplayGain(ReplayGainMode::Off),
            "rg_track" => ReplayGain(ReplayGainMode::Track),
            "rg_album" => ReplayGain(ReplayGainMode::Album),
            "rg_clip" => ToggleRgClipping,
            "bm_add" => AddBookmark,
            "bm_clear" => ClearBookmarks,
            "vis_normal" => VisAnalyzerStyle(AnalyzerStyle::Normal),
            "vis_fire" => VisAnalyzerStyle(AnalyzerStyle::Fire),
            "vis_line" => VisAnalyzerStyle(AnalyzerStyle::Line),
            "vis_thick" => VisThickBands(true),
            "vis_thin" => VisThickBands(false),
            "vis_peaks" => ToggleVisPeaks,
            "vis_dots" => VisScopeStyle(ScopeStyle::Dots),
            "vis_lines" => VisScopeStyle(ScopeStyle::Lines),
            "vis_solid" => VisScopeStyle(ScopeStyle::Solid),
            other => {
                if let Some(code) = other.strip_prefix("lang_") {
                    return Lang::ALL.into_iter().find(|l| l.code() == code).map(|l| Language(Some(l)));
                }
                if let Some(i) = other.strip_prefix("skin_").and_then(|n| n.parse::<usize>().ok()) {
                    return self.skins.get(i).cloned().map(SkinFile);
                }
                if let Some(i) = other.strip_prefix("eqp_").and_then(|n| n.parse::<usize>().ok()) {
                    return Some(EqPreset(i));
                }
                let num = |p: &str| other.strip_prefix(p).and_then(|n| n.parse::<i64>().ok());
                if let Some(ms) = num("xfade_") {
                    return Some(Crossfade(ms as u32));
                }
                if let Some(db) = num("rgpre_") {
                    return Some(RgPreamp(db as i32));
                }
                if let Some(i) = num("bm_play_") {
                    return Some(Bookmark(i as usize));
                }
                if let Some(i) = num("bm_del_") {
                    return Some(RemoveBookmark(i as usize));
                }
                if let Some(v) = num("vis_fall_") {
                    return Some(VisFalloff(v as u8));
                }
                if let Some(v) = num("vis_pfall_") {
                    return Some(VisPeakFalloff(v as u8));
                }
                if let Some(v) = num("vis_refresh_") {
                    return Some(VisRefresh(v as u8));
                }
                return None;
            }
        })
    }
}

fn show(menu: &Menu, window: &Window) {
    let Ok(handle) = window.window_handle() else { return };
    match handle.as_raw() {
        #[cfg(target_os = "macos")]
        RawWindowHandle::AppKit(h) => unsafe {
            menu.show_context_menu_for_nsview(h.ns_view.as_ptr() as _, None);
        },
        #[cfg(target_os = "windows")]
        RawWindowHandle::Win32(h) => unsafe {
            menu.show_context_menu_for_hwnd(h.hwnd.get() as _, None);
        },
        _ => {}
    }
}

#[cfg(target_os = "macos")]
fn file_submenu() -> Submenu {
    Submenu::with_items(
        t("menu.file"),
        true,
        &[&item("play_file", "menu.play_file"), &item("play_folder", "menu.play_folder"), &sep(), &item("pl_load", "pl.load"), &item("pl_save", "pl.save"), &sep(), &item("file_info", "menu.file_info")],
    )
    .expect("menu")
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
            &sep(),
            &item("jump", "menu.jump_to_file"),
        ],
    )
    .expect("menu")
}

#[cfg(target_os = "macos")]
fn windows_submenu(st: &MenuState) -> Submenu {
    Submenu::with_items(
        t("menu.windows"),
        true,
        &[&check("win_eq", "menu.equalizer", st.eq_open), &check("win_pl", "menu.playlist_editor", st.pl_open), &check("win_lib", "menu.library", st.library_open)],
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
            &crossfade_submenu(s),
            &replaygain_submenu(s),
            &sep(),
            &lang,
        ],
    )
    .expect("menu")
}

fn spotify_submenu(st: &MenuState) -> Submenu {
    let m = Submenu::new("Spotify", true);
    if st.spotify_connected {
        let _ = m.append(&item("spotify_logout", "lib.logout"));
    } else {
        let _ = m.append(&item("spotify_login", "lib.login"));
    }
    m
}

fn crossfade_submenu(s: &Settings) -> Submenu {
    let m = Submenu::new(t("menu.crossfade"), true);
    let cur = s.playback.crossfade_ms;
    let _ = m.append(&CheckMenuItem::with_id("xfade_0", t("opt.off"), true, cur == 0, None));
    for secs in [1u32, 2, 3, 5, 8, 10] {
        let _ = m.append(&CheckMenuItem::with_id(format!("xfade_{}", secs * 1000), format!("{secs} s"), true, cur == secs * 1000, None));
    }
    m
}

fn replaygain_submenu(s: &Settings) -> Submenu {
    let p = &s.playback;
    let m = Submenu::new(t("menu.replaygain"), true);
    let _ = m.append(&CheckMenuItem::with_id("rg_off", t("opt.off"), true, p.replay_gain == ReplayGainMode::Off, None));
    let _ = m.append(&CheckMenuItem::with_id("rg_track", t("rg.track"), true, p.replay_gain == ReplayGainMode::Track, None));
    let _ = m.append(&CheckMenuItem::with_id("rg_album", t("rg.album"), true, p.replay_gain == ReplayGainMode::Album, None));
    let _ = m.append(&sep());
    let _ = m.append(&check("rg_clip", "rg.prevent_clipping", p.rg_prevent_clipping));
    let pre = Submenu::new(t("rg.preamp"), true);
    for db in [-6i32, -3, 0, 3, 6] {
        let label = format!("{db:+} dB");
        let _ = pre.append(&CheckMenuItem::with_id(format!("rgpre_{db}"), label, true, (p.rg_preamp_db - db as f32).abs() < 0.01, None));
    }
    let _ = m.append(&pre);
    m
}

fn bookmarks_submenu(st: &MenuState) -> Submenu {
    let m = Submenu::new(t("menu.bookmarks"), true);
    let _ = m.append(&item("bm_add", "bm.add"));
    let _ = m.append(&sep());
    if st.bookmarks.is_empty() {
        let _ = m.append(&MenuItem::with_id("bm_none", t("bm.empty"), false, None));
    } else {
        for (i, b) in st.bookmarks.iter().enumerate() {
            let _ = m.append(&MenuItem::with_id(format!("bm_play_{i}"), b, true, None));
        }
        let _ = m.append(&sep());
        let del = Submenu::new(t("bm.remove"), true);
        for (i, b) in st.bookmarks.iter().enumerate() {
            let _ = del.append(&MenuItem::with_id(format!("bm_del_{i}"), b, true, None));
        }
        let _ = m.append(&del);
        let _ = m.append(&item("bm_clear", "bm.clear"));
    }
    m
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
    let v = &s.vis;
    let style = Submenu::with_items(
        t("vis.analyzer_style"),
        true,
        &[
            &check("vis_normal", "vis.normal", v.analyzer_style == AnalyzerStyle::Normal),
            &check("vis_fire", "vis.fire", v.analyzer_style == AnalyzerStyle::Fire),
            &check("vis_line", "vis.line", v.analyzer_style == AnalyzerStyle::Line),
            &sep(),
            &check("vis_thick", "vis.thick", v.thick_bands),
            &check("vis_thin", "vis.thin", !v.thick_bands),
            &sep(),
            &check("vis_peaks", "vis.peaks", v.peaks),
        ],
    )
    .expect("menu");
    let speeds = |prefix: &str, key: &str, cur: u8| {
        let m = Submenu::new(t(key), true);
        for i in 0..5u8 {
            let _ = m.append(&CheckMenuItem::with_id(format!("{prefix}{i}"), t(&format!("vis.speed{i}")), true, cur == i, None));
        }
        m
    };
    let fall = speeds("vis_fall_", "vis.falloff", v.falloff);
    let pfall = speeds("vis_pfall_", "vis.peak_falloff", v.peak_falloff);
    let scope = Submenu::with_items(
        t("vis.scope_style"),
        true,
        &[
            &check("vis_dots", "vis.dots", v.scope_style == ScopeStyle::Dots),
            &check("vis_lines", "vis.lines", v.scope_style == ScopeStyle::Lines),
            &check("vis_solid", "vis.solid", v.scope_style == ScopeStyle::Solid),
        ],
    )
    .expect("menu");
    let refresh = Submenu::new(t("vis.refresh"), true);
    for (i, label) in [t("vis.full"), "1/2", "1/4", "1/8"].into_iter().enumerate() {
        let _ = refresh.append(&CheckMenuItem::with_id(format!("vis_refresh_{i}"), label, true, v.refresh as usize == i, None));
    }
    Submenu::with_items(
        t("menu.visualization"),
        true,
        &[
            &check("vis_analyzer", "menu.analyzer", v.mode == VisMode::Analyzer),
            &check("vis_scope", "menu.oscilloscope", v.mode == VisMode::Oscilloscope),
            &check("vis_off", "menu.vis_off", v.mode == VisMode::Off),
            &sep(),
            &style,
            &fall,
            &pfall,
            &scope,
            &refresh,
        ],
    )
    .expect("menu")
}

fn build_main(st: &MenuState, skins: &[PathBuf]) -> Menu {
    let m = Menu::new();
    let _ = m.append(&item("about", "menu.about"));
    let _ = m.append(&sep());
    let _ = m.append(&item("play_file", "menu.play_file"));
    let _ = m.append(&item("play_folder", "menu.play_folder"));
    let _ = m.append(&item("file_info", "menu.file_info"));
    let _ = m.append(&sep());
    let _ = m.append(&check("win_eq", "menu.equalizer", st.eq_open));
    let _ = m.append(&check("win_pl", "menu.playlist_editor", st.pl_open));
    let _ = m.append(&check("win_lib", "menu.library", st.library_open));
    let _ = m.append(&spotify_submenu(st));
    let _ = m.append(&sep());
    let _ = m.append(&playback_submenu());
    let _ = m.append(&bookmarks_submenu(st));
    let _ = m.append(&options_submenu(st.settings));
    let _ = m.append(&skins_submenu(skins));
    let _ = m.append(&vis_submenu(st.settings));
    let _ = m.append(&sep());
    let _ = m.append(&item("exit", "menu.exit"));
    m
}

fn build_eq_presets(st: &MenuState) -> Menu {
    let m = Menu::new();
    let load = Submenu::new(t("eq.load"), true);
    for (i, p) in factory_presets().iter().enumerate() {
        let _ = load.append(&MenuItem::with_id(format!("eqp_{i}"), &p.name, true, None));
    }
    let _ = load.append(&sep());
    let _ = load.append(&item("eq_load_file", "eq.from_file"));
    let _ = m.append(&load);
    let save = Submenu::with_items(t("eq.save"), true, &[&item("eq_save_file", "eq.to_file"), &item("eq_save_auto", "eq.auto_preset")]).expect("menu");
    let _ = m.append(&save);
    let _ = m.append(&MenuItem::with_id("eq_delete_auto", t("eq.delete_auto"), st.has_auto_preset, None));
    let _ = m.append(&sep());
    let _ = m.append(&check("eq_on", "eq.enabled", st.settings.eq.enabled));
    let _ = m.append(&check("eq_auto", "eq.auto", st.settings.eq.auto));
    let _ = m.append(&sep());
    let _ = m.append(&item("eq_reset", "eq.reset"));
    m
}

fn build_list(items: &[(&str, &str)]) -> Menu {
    let m = Menu::new();
    for (id, key) in items {
        let _ = m.append(&item(id, key));
    }
    m
}

fn build_sort() -> Menu {
    build_list(&[
        ("pl_sort_title", "pl.sort_title"),
        ("pl_sort_file", "pl.sort_filename"),
        ("pl_sort_path", "pl.sort_path"),
        ("pl_reverse", "pl.reverse"),
        ("pl_randomize", "pl.randomize"),
    ])
}

fn build_pl_context() -> Menu {
    let m = Menu::new();
    let _ = m.append(&item("pl_play", "pl.play_item"));
    let _ = m.append(&item("pl_info", "menu.file_info"));
    let _ = m.append(&sep());
    let _ = m.append(&item("pl_remove_sel", "pl.remove_selected"));
    let _ = m.append(&item("pl_crop", "pl.crop"));
    let _ = m.append(&sep());
    let _ = m.append(&item("pl_select_all", "pl.select_all"));
    let _ = m.append(&item("pl_select_none", "pl.select_none"));
    let _ = m.append(&item("pl_invert", "pl.invert"));
    let _ = m.append(&sep());
    let sort = Submenu::new(t("pl.sort"), true);
    for (id, key) in [
        ("pl_sort_title", "pl.sort_title"),
        ("pl_sort_file", "pl.sort_filename"),
        ("pl_sort_path", "pl.sort_path"),
        ("pl_reverse", "pl.reverse"),
        ("pl_randomize", "pl.randomize"),
    ] {
        let _ = sort.append(&item(id, key));
    }
    let _ = m.append(&sort);
    let _ = m.append(&sep());
    let _ = m.append(&item("pl_add_files", "pl.add_files"));
    let _ = m.append(&item("pl_add_folder", "pl.add_folder"));
    let _ = m.append(&item("pl_add_url", "pl.add_url"));
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
