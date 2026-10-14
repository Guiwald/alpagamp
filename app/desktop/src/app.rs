//! L'application de bureau : les fenêtres (principale, égaliseur, playlist, fenêtre générique, Bibliothèque,
//! visualisation), leurs événements, l'aimantation et les commandes.

mod cursors;
mod milk;
mod plugins;
mod skins;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use alpagamp_audio::{Decoder, PlayState};
use alpagamp_core::eq::{CENTER, EqPreset as Preset, factory_presets, read_eqf, write_eqf};
use alpagamp_core::formats::{read_playlist_bytes, write_m3u, write_pls};
use alpagamp_core::i18n::{self, Lang, t};
use alpagamp_core::settings::{VisMode, WindowState};
use alpagamp_core::time::format_duration;
use alpagamp_core::{Entry, Settings, Source};
use alpagamp_skin::{Image, Skin};
use alpagamp_audio::Media;
use alpagamp_core::library::{LibraryItem, LibraryKind};
use alpagamp_spotify::{ConnectCommand, LibraryRequest, Spotify, Status as SpotifyStatus};
use alpagamp_ui::{
    Action, EqView, EqWindow, GenKind, GenList, GenView, GenWindow, LibTab, LibraryAction, LibraryView, LibraryWindow, MainWindow, Mods, PlMenuItem,
    PlaylistView, PlaylistWindow,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::ModifiersState;
use winit::window::{WindowId, WindowLevel};

use crate::command::{Command, Popup};
use crate::dialogs;
use crate::keys::{self, Focus};
use crate::layout::{self, R};
use crate::paths::Paths;
use alpagamp_player::Player;
use crate::window::{SkinWindow, WindowOptions};

/// Bouton « Ouvrir… » du navigateur de skins (boîte de dialogue native : Mac et Windows).
const SKINS_OPEN: bool = cfg!(any(target_os = "macos", target_os = "windows"));

/// Distance d'aimantation, en pixels skin.
const SNAP: i32 = 10;
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Main = 0,
    Eq = 1,
    Playlist = 2,
    Gen = 3,
    Library = 4,
    /// Visualisation (presets MilkDrop), présentée par le GPU.
    Vis = 5,
    /// Navigateur de skins.
    Skins = 6,
}

const KINDS: [Kind; 7] = [Kind::Main, Kind::Eq, Kind::Playlist, Kind::Gen, Kind::Library, Kind::Vis, Kind::Skins];

/// Une fenêtre système et son état.
struct Slot {
    win: Option<SkinWindow>,
    fb: Image,
    /// Coin haut-gauche (pixels physiques), tenu à jour par l'application.
    pos: Option<(i32, i32)>,
    open: bool,
    /// Position de la souris dans la fenêtre (pixels physiques).
    cursor: (f64, f64),
}

impl Slot {
    fn new(open: bool, pos: Option<(i32, i32)>) -> Slot {
        Slot { win: None, fb: Image::new(1, 1, 0), pos, open, cursor: (0.0, 0.0) }
    }

    fn visible(&self) -> bool {
        self.open && self.win.is_some()
    }
}

/// Déplacement de fenêtre(s) en cours.
struct Moving {
    kind: Kind,
    /// Point saisi, relatif à la fenêtre (pixels physiques).
    grab: (f64, f64),
    /// Fenêtres déplacées ensemble (la principale entraîne celles qui lui sont collées).
    group: Vec<Kind>,
}

pub struct App {
    paths: Paths,
    player: Player,
    default_skin: Skin,
    skin: Skin,
    slots: [Slot; 7],
    main_ui: MainWindow,
    eq_ui: EqWindow,
    pl_ui: PlaylistWindow,
    gen_ui: GenWindow,
    lib_ui: LibraryWindow,
    milk: milk::MilkState,
    /// Agrandissement suggéré par l'écran (×1 à ×3).
    auto_zoom: u32,
    cursors: cursors::Cursors,
    skins: alpagamp_museum::Browser,
    spotify: Arc<Spotify>,
    /// Requête de bibliothèque attendue pour la vue affichée.
    lib_pending: Option<u64>,
    /// Requête dont les morceaux iront à la fin de la playlist (lien de playlist ou d'album Spotify).
    enqueue_pending: Option<u64>,
    mods: ModifiersState,
    moving: Option<Moving>,
    last_click: Option<(Instant, Kind, i32, i32)>,
    start: Instant,
    last_frame: Instant,
    next_frame: Instant,
    vis_l: Vec<f32>,
    vis_r: Vec<f32>,
    dropped: Vec<(Kind, PathBuf)>,
    pending_args: Vec<PathBuf>,
    focused: Option<Kind>,
    /// Les fenêtres secondaires ont été cachées en même temps que la principale a été réduite.
    hidden_by_minimize: bool,
    eq_status_shown: bool,
    /// Préréglages d'égaliseur automatiques (nom = nom du fichier du morceau).
    auto_presets: Vec<Preset>,
    bookmarks: Vec<Entry>,
    started: bool,
    menus_dirty: bool,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    menus: Option<crate::menu::Menus>,
}

impl App {
    pub fn new(args: Vec<PathBuf>) -> App {
        let paths = Paths::new();
        let settings = Settings::load(&paths.settings);
        apply_language(&settings);
        let default_skin = Skin::default_skin();
        let skin = settings
            .skin
            .as_ref()
            .and_then(|p| std::fs::read(p).ok().and_then(|b| Skin::load_wsz(&p.display().to_string(), &b, &default_skin).ok()))
            .unwrap_or_else(|| default_skin.clone());
        let auto_presets = std::fs::read(&paths.eq_auto).ok().and_then(|b| read_eqf(&b).ok()).unwrap_or_default();
        let bookmarks = std::fs::read(&paths.bookmarks).ok().map(|b| read_playlist_bytes(&b, paths.bookmarks.parent())).unwrap_or_default();
        let mut player = Player::new(settings);
        player.init_plugins(paths.plugins_dir.clone());
        player.set_plugin_colors(&skin.vis.0);
        if args.is_empty() {
            player.load_playlist_file(&paths.playlist);
        }
        let s = &player.settings;
        let mut main_ui = MainWindow::new();
        main_ui.shade = s.main_window.map(|w| w.shade).unwrap_or(false);
        let mut eq_ui = EqWindow::new();
        eq_ui.shade = s.eq_window.map(|w| w.shade).unwrap_or(false);
        let mut pl_ui = match s.playlist_window {
            Some(w) if w.w > 0 && w.h > 0 => PlaylistWindow::new(w.w, w.h),
            _ => PlaylistWindow::default(),
        };
        pl_ui.shade = s.playlist_window.map(|w| w.shade).unwrap_or(false);
        let pos = |w: Option<WindowState>| w.map(|w| (w.x, w.y));
        let slots = [
            Slot::new(true, pos(s.main_window)),
            Slot::new(s.eq_window.map(|w| w.open).unwrap_or(true), pos(s.eq_window)),
            Slot::new(s.playlist_window.map(|w| w.open).unwrap_or(true), pos(s.playlist_window)),
            Slot::new(false, None),
            Slot::new(false, None),
            Slot::new(s.milkdrop.window.map(|w| w.open).unwrap_or(false), s.milkdrop.window.map(|w| (w.x, w.y))),
            Slot::new(false, None),
        ];
        let milk = match s.milkdrop.window {
            Some(w) if w.w > 0 && w.h > 0 => milk::MilkState::new(w.w, w.h),
            _ => milk::MilkState::new(275, 232),
        };
        let spotify = Arc::new(Spotify::new(paths.config_dir.join("spotify"), device_name(), false, Box::new(crate::dialogs::open_url)));
        let sp = spotify.clone();
        player.resolver = Some(Box::new(move |src: &Source| match src {
            Source::Spotify(uri) => Some(Media::Custom(sp.opener(uri))),
            _ => None,
        }));
        let skins = skins::new_browser(paths.skins_dir.clone());
        App {
            paths,
            player,
            default_skin,
            skin,
            slots,
            main_ui,
            eq_ui,
            pl_ui,
            gen_ui: GenWindow::new(GenKind::Jump),
            lib_ui: LibraryWindow::default(),
            milk,
            auto_zoom: 1,
            cursors: cursors::Cursors::default(),
            skins,
            spotify,
            lib_pending: None,
            enqueue_pending: None,
            mods: ModifiersState::empty(),
            moving: None,
            last_click: None,
            start: Instant::now(),
            last_frame: Instant::now(),
            next_frame: Instant::now(),
            vis_l: vec![0.0; alpagamp_ui::vis::WINDOW],
            vis_r: vec![0.0; alpagamp_ui::vis::WINDOW],
            dropped: Vec::new(),
            pending_args: args,
            focused: None,
            hidden_by_minimize: false,
            eq_status_shown: false,
            auto_presets,
            bookmarks,
            started: false,
            menus_dirty: false,
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            menus: None,
        }
    }

    fn slot(&self, k: Kind) -> &Slot {
        &self.slots[k as usize]
    }

    fn slot_mut(&mut self, k: Kind) -> &mut Slot {
        &mut self.slots[k as usize]
    }

    fn kind_of(&self, id: WindowId) -> Option<Kind> {
        KINDS.into_iter().find(|&k| self.slot(k).win.as_ref().is_some_and(|w| w.window.id() == id))
    }

    fn mods(&self) -> Mods {
        let ctrl = if cfg!(target_os = "macos") { self.mods.super_key() } else { self.mods.control_key() };
        Mods { shift: self.mods.shift_key(), ctrl, alt: self.mods.alt_key() }
    }

    fn skin_size(&self, k: Kind) -> (u32, u32) {
        match k {
            Kind::Main => self.main_ui.size(),
            Kind::Eq => self.eq_ui.size(),
            Kind::Playlist => self.pl_ui.size(),
            Kind::Gen => self.gen_ui.size(),
            Kind::Library => self.lib_ui.size(),
            Kind::Vis => self.milk.ui.size(),
            Kind::Skins => self.skins.ui.size(),
        }
    }

    /// Agrandissement de toutes les fenêtres (×1 à ×3 ; selon l'écran par défaut).
    fn zoom(&self) -> u32 {
        self.player.settings.zoom(self.auto_zoom)
    }

    fn title_for(&self, k: Kind) -> String {
        match k {
            Kind::Main => "Alpagamp".into(),
            Kind::Eq => format!("Alpagamp – {}", t("menu.equalizer")),
            Kind::Playlist => format!("Alpagamp – {}", t("menu.playlist_editor")),
            Kind::Library => format!("Alpagamp – {}", t("menu.library")),
            Kind::Vis => format!("Alpagamp – {}", t("milk.title")),
            Kind::Skins => format!("Alpagamp – {}", t("skins.title")),
            Kind::Gen => match self.gen_ui.kind {
                GenKind::Jump => format!("Alpagamp – {}", t("jump.title")),
                GenKind::AddUrl => format!("Alpagamp – {}", t("url.title")),
                GenKind::Presets => format!("Alpagamp – {}", t("milk.presets")),
            },
        }
    }

    /// Rectangles physiques des fenêtres visibles.
    fn rects(&self) -> Vec<Option<R>> {
        KINDS
            .iter()
            .map(|&k| {
                let s = self.slot(k);
                match (s.visible(), s.pos, s.win.as_ref()) {
                    (true, Some((x, y)), Some(w)) => {
                        let (lw, lh) = w.layout_size();
                        Some(R::new(x, y, lw, lh))
                    }
                    _ => None,
                }
            })
            .collect()
    }

    fn set_pos(&mut self, k: Kind, x: i32, y: i32) {
        let s = self.slot_mut(k);
        if s.pos != Some((x, y)) {
            s.pos = Some((x, y));
            if let Some(w) = &s.win {
                w.set_layout_position(x, y);
            }
        }
    }

    fn apply_rects(&mut self, rects: &[Option<R>]) {
        for (i, r) in rects.iter().enumerate() {
            if let Some(r) = r {
                self.set_pos(KINDS[i], r.x, r.y);
            }
        }
    }

    // --- Ouverture et taille des fenêtres -------------------------------------------------------------

    fn create_window(&mut self, k: Kind, event_loop: &ActiveEventLoop) {
        if self.slot(k).win.is_some() {
            return;
        }
        let title = self.title_for(k);
        let owner = if k == Kind::Main { None } else { self.slot(Kind::Main).win.as_ref().map(|w| w.window.clone()) };
        let opts = WindowOptions {
            title: &title,
            skin_size: self.skin_size(k),
            position: self.slot(k).pos,
            on_top: self.player.settings.always_on_top,
            user_zoom: self.zoom(),
            secondary: k != Kind::Main,
            owner: owner.as_deref(),
            gpu: k == Kind::Vis,
        };
        match SkinWindow::new(event_loop, opts) {
            Ok(w) => {
                if self.slot(k).pos.is_none() {
                    self.slot_mut(k).pos = w.layout_position();
                }
                self.slot_mut(k).win = Some(w);
                self.apply_shape(k);
            }
            Err(e) => log::error!("fenêtre impossible : {e}"),
        }
    }

    /// Place une fenêtre secondaire qui n'a pas encore de position : sous la dernière fenêtre empilée sous la
    /// principale.
    fn default_position(&self, k: Kind) -> Option<(i32, i32)> {
        let (mx, my) = self.slot(Kind::Main).pos?;
        let mw = self.slot(Kind::Main).win.as_ref()?;
        let mut y = my + mw.layout_size().1;
        for other in [Kind::Eq, Kind::Playlist] {
            if other == k {
                break;
            }
            if let (true, Some(w)) = (self.slot(other).visible(), self.slot(other).win.as_ref()) {
                y += w.layout_size().1;
            }
        }
        if matches!(k, Kind::Gen | Kind::Library | Kind::Vis | Kind::Skins) {
            // La fenêtre générique se met à droite de la visualisation si elle est ouverte.
            if k == Kind::Gen {
                if let (true, Some((vx, vy)), Some(vw)) = (self.slot(Kind::Vis).visible(), self.slot(Kind::Vis).pos, self.slot(Kind::Vis).win.as_ref()) {
                    return Some((vx + vw.layout_size().0 + 10, vy));
                }
            }
            let w = mw.layout_size().0;
            return Some((mx + w + if matches!(k, Kind::Library | Kind::Skins) { 0 } else { 20 }, my));
        }
        Some((mx, y))
    }

    fn set_open(&mut self, k: Kind, open: bool, event_loop: &ActiveEventLoop) {
        if k == Kind::Main {
            return;
        }
        if open {
            if self.slot(k).pos.is_none() {
                let p = self.default_position(k);
                self.slot_mut(k).pos = p;
            }
            self.slot_mut(k).open = true;
            if self.slot(k).win.is_none() {
                self.create_window(k, event_loop);
            } else if let Some(w) = &self.slot(k).win {
                w.window.set_visible(true);
                w.window.request_redraw();
            }
            if let Some(w) = &self.slot(k).win {
                w.window.focus_window();
            }
        } else {
            self.slot_mut(k).open = false;
            if let Some(w) = &self.slot(k).win {
                w.window.set_visible(false);
            }
            if self.focused == Some(k) {
                self.focused = None;
                if let Some(w) = &self.slot(Kind::Main).win {
                    w.window.focus_window();
                }
            }
        }
        self.menus_dirty = true;
    }

    /// Applique la taille skin courante d'une fenêtre (bandeau, double taille, redimensionnement) en décalant
    /// les fenêtres collées sous elle ou à sa droite.
    fn relayout(&mut self, k: Kind) {
        let size = self.skin_size(k);
        let zoom = self.zoom();
        let Some(w) = self.slot(k).win.as_ref() else { return };
        let (nw, nh) = w.layout_size_for(size, zoom);
        let rects = self.rects();
        if rects[k as usize].is_some() {
            let out = layout::resize(&rects, k as usize, nw, nh);
            self.apply_rects(&out);
        }
        if let Some(w) = self.slot_mut(k).win.as_mut() {
            w.set_geometry(size, zoom);
        }
        self.apply_shape(k);
    }

    // --- Menus ----------------------------------------------------------------------------------------

    fn current_file_name(&self) -> Option<String> {
        match &self.player.playlist.current_entry()?.source {
            Source::File(p) => p.file_name().map(|n| n.to_string_lossy().into_owned()),
            _ => None,
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    fn menu_state(&self) -> crate::menu::MenuState<'_> {
        let name = self.current_file_name();
        crate::menu::MenuState {
            settings: &self.player.settings,
            eq_open: self.slot(Kind::Eq).open,
            pl_open: self.slot(Kind::Playlist).open,
            has_auto_preset: name.is_some_and(|n| self.auto_presets.iter().any(|p| p.name == n)),
            bookmarks: self.bookmarks.iter().map(|e| e.display_title()).collect(),
            library_open: self.slot(Kind::Library).open,
            spotify_connected: self.spotify.is_connected(),
            milk_open: self.slot(Kind::Vis).open,
            zoom: self.zoom(),
            pack_installed: alpagamp_milkdrop::pack::installed(&self.paths.presets_dir) || self.milk.pack.busy(),
            plugins: self.plugin_menu_items(),
            plugins_broken: self.player.plugins.broken.clone(),
        }
    }

    fn refresh_menus(&mut self) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let st = self.menu_state();
            let menus = crate::menu::Menus::new(&st, &self.paths.skins_dir);
            self.menus = Some(menus);
        }
    }

    #[allow(unused_variables)]
    fn popup(&self, which: Popup, over: Kind) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let (Some(m), Some(w)) = (self.menus.as_ref(), self.slot(over).win.as_ref()) {
            m.popup(which, &self.menu_state(), &w.window);
        }
    }

    fn save(&mut self) {
        let state = |s: &Slot, shade: bool, size: (u32, u32)| s.pos.map(|(x, y)| WindowState { x, y, open: s.open, shade, w: size.0, h: size.1 });
        let milk_full = self.milk_fullscreen_now();
        let st = &mut self.player.settings;
        if let Some(w) = state(&self.slots[0], self.main_ui.shade, (275, 116)) {
            st.main_window = Some(w);
        }
        if let Some(w) = state(&self.slots[1], self.eq_ui.shade, (275, 116)) {
            st.eq_window = Some(w);
        } else if let Some(w) = st.eq_window.as_mut() {
            w.open = self.slots[1].open;
        }
        if let Some(w) = state(&self.slots[2], self.pl_ui.shade, (self.pl_ui.w, self.pl_ui.h)) {
            st.playlist_window = Some(w);
        } else if let Some(w) = st.playlist_window.as_mut() {
            w.open = self.slots[2].open;
        }
        if !milk_full {
            if let Some(w) = state(&self.slots[5], false, self.milk.ui.size()) {
                st.milkdrop.window = Some(w);
            } else if let Some(w) = st.milkdrop.window.as_mut() {
                w.open = self.slots[5].open;
            }
        }
        st.resume_index = self.player.playlist.current();
        if let Err(e) = st.save(&self.paths.settings) {
            log::warn!("réglages non enregistrés : {e}");
        }
        self.player.save_playlist_file(&self.paths.playlist);
    }

    // --- Skins ----------------------------------------------------------------------------------------

    fn load_skin(&mut self, path: &Path) {
        let result = std::fs::read(path).map_err(|e| e.to_string()).and_then(|b| {
            let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            Skin::load_wsz(&name, &b, &self.default_skin).map_err(|e| e.to_string())
        });
        match result {
            Ok(skin) => {
                // Range une copie dans le dossier des skins pour qu'elle apparaisse dans le menu.
                let mut stored = path.to_path_buf();
                if !path.starts_with(&self.paths.skins_dir) {
                    if let Some(name) = path.file_name() {
                        let dest = self.paths.skins_dir.join(name);
                        if std::fs::create_dir_all(&self.paths.skins_dir).is_ok() && std::fs::copy(path, &dest).is_ok() {
                            stored = dest;
                        }
                    }
                }
                self.skin = skin;
                self.cursors.reset();
                self.apply_shapes();
                self.player.set_plugin_colors(&self.skin.vis.0);
                self.player.settings.skin = Some(stored);
                self.menus_dirty = true;
            }
            Err(e) => dialogs::message(t("error.skin"), &format!("{}\n\n{e}", path.display())),
        }
    }

    fn request_redraw(&self) {
        for s in &self.slots {
            if let (true, Some(w)) = (s.open, &s.win) {
                w.window.request_redraw();
            }
        }
    }

    // --- Actions des fenêtres -------------------------------------------------------------------------

    fn action(&mut self, from: Kind, a: Action, event_loop: &ActiveEventLoop) {
        use Command as C;
        let cmd = match a {
            Action::Previous(m) if m.ctrl => C::First,
            Action::Previous(m) if m.shift => C::Back5,
            Action::Previous(_) => C::Previous,
            Action::Play(m) if m.shift || m.ctrl => C::OpenFiles,
            Action::Play(_) => C::Play,
            Action::Pause => C::Pause,
            Action::Stop(m) if m.shift => C::StopFade,
            Action::Stop(m) if m.ctrl => C::StopAfterCurrent,
            Action::Stop(_) => C::Stop,
            Action::Next(m) if m.ctrl => C::Last,
            Action::Next(m) if m.shift => C::Fwd5,
            Action::Next(_) => C::Next,
            Action::Eject(m) if m.shift => C::OpenFolder,
            Action::Eject(_) => C::OpenFiles,
            Action::SetVolume(v) => {
                self.player.set_volume(v);
                return;
            }
            Action::SetBalance(b) => {
                self.player.set_balance(b);
                return;
            }
            Action::Seek(ms) => {
                self.player.seek(ms);
                return;
            }
            Action::ToggleShuffle => C::ToggleShuffle,
            Action::ToggleRepeat => C::ToggleRepeat,
            Action::ToggleEqWindow => C::ToggleEqWindow,
            Action::TogglePlaylistWindow => C::TogglePlaylistWindow,
            Action::Close => match from {
                Kind::Main => C::Exit,
                Kind::Eq => C::ToggleEqWindow,
                Kind::Playlist => C::TogglePlaylistWindow,
                Kind::Gen | Kind::Library => {
                    self.set_open(from, false, event_loop);
                    return;
                }
                Kind::Vis => C::ToggleMilk,
                Kind::Skins => C::ToggleSkins,
            },
            Action::Minimize => C::Minimize,
            Action::ToggleShade => {
                self.toggle_shade(from);
                return;
            }
            Action::OptionsMenu | Action::VisMenu => {
                self.popup(Popup::Main, from);
                return;
            }
            Action::ToggleAlwaysOnTop => C::ToggleAlwaysOnTop,
            Action::FileInfo => C::FileInfo,
            Action::ToggleDoubleSize => C::ToggleDoubleSize,
            Action::ToggleTimeMode => C::ToggleTimeMode,
            Action::CycleVisMode => C::Vis(self.player.settings.vis.mode.cycle()),
            Action::ToggleVisPlugin => C::ToggleMilk,
            Action::StartMove => {
                self.start_move(from);
                return;
            }
            Action::About => C::About,
            Action::SetEqBand(i, v) => {
                let mut eq = self.player.settings.eq;
                eq.bands[i.min(9)] = v;
                self.player.set_eq(eq);
                return;
            }
            Action::SetPreamp(v) => {
                let mut eq = self.player.settings.eq;
                eq.preamp = v;
                self.player.set_eq(eq);
                return;
            }
            Action::SetEqAllBands(v) => {
                let mut eq = self.player.settings.eq;
                eq.bands = [v; 10];
                self.player.set_eq(eq);
                return;
            }
            Action::ToggleEqEnabled => C::ToggleEq,
            Action::ToggleEqAuto => C::ToggleEqAuto,
            Action::EqPresetsMenu => {
                self.popup(Popup::EqPresets, Kind::Eq);
                return;
            }
            Action::PlaylistClick { index, shift, ctrl } => {
                self.player.playlist.click(index, shift, ctrl);
                return;
            }
            Action::PlaylistPlay(i) => {
                self.player.play_index(i);
                return;
            }
            Action::PlaylistMoveSelection(d) => C::PlMoveSelection(d),
            Action::PlaylistResize(_, _) => {
                self.relayout(Kind::Playlist);
                return;
            }
            Action::PlaylistMenu(item) => match item {
                PlMenuItem::AddUrl => C::PlAddUrl,
                PlMenuItem::AddDir => C::PlAddFolder,
                PlMenuItem::AddFile => C::PlAddFiles,
                PlMenuItem::RemoveAll => C::PlRemoveAll,
                PlMenuItem::Crop => C::PlCrop,
                PlMenuItem::RemoveSelected => C::PlRemoveSelected,
                PlMenuItem::RemoveMisc => {
                    self.popup(Popup::PlRemoveMisc, Kind::Playlist);
                    return;
                }
                PlMenuItem::InvertSelection => C::PlInvertSelection,
                PlMenuItem::SelectNone => C::PlSelectNone,
                PlMenuItem::SelectAll => C::PlSelectAll,
                PlMenuItem::Sort => {
                    self.popup(Popup::PlSort, Kind::Playlist);
                    return;
                }
                PlMenuItem::FileInfo => C::PlFileInfo,
                PlMenuItem::MiscOptions => {
                    self.popup(Popup::PlMisc, Kind::Playlist);
                    return;
                }
                PlMenuItem::NewList => C::PlNew,
                PlMenuItem::SaveList => C::PlSave,
                PlMenuItem::LoadList => C::PlLoad,
            },
            Action::JumpTo(i) => {
                self.set_open(Kind::Gen, false, event_loop);
                self.player.play_index(i);
                self.pl_ui.ensure_visible(i);
                return;
            }
            Action::Library(la) => {
                self.library_action(la);
                return;
            }
            Action::SelectPreset(i) => {
                if let Some(md) = self.milk.engine.as_mut() {
                    md.select(i, false);
                }
                return;
            }
            Action::MilkResize(_, _) => {
                self.relayout(Kind::Vis);
                return;
            }
            Action::MilkFullscreen => C::MilkFullscreen,
            Action::Skins(sa) => {
                self.skins_action(sa, event_loop);
                return;
            }
            Action::SubmitUrl(url) => {
                self.set_open(Kind::Gen, false, event_loop);
                match Source::parse(&url, None) {
                    // Lien de playlist ou d'album Spotify : on ajoute ses morceaux.
                    Source::Spotify(uri) if !uri.starts_with("spotify:track:") && !uri.starts_with("spotify:episode:") => {
                        self.enqueue_pending = Some(self.spotify.request(LibraryRequest::Tracks(uri)));
                    }
                    src => {
                        self.player.playlist.extend([Entry::new(src)]);
                        self.player.playlist_changed();
                    }
                }
                return;
            }
        };
        self.command(cmd, event_loop);
    }

    /// Forme du skin (`region.txt`) pour la fenêtre principale et l'égaliseur.
    fn apply_shape(&self, k: Kind) {
        use alpagamp_skin::RegionWindow as RW;
        let which = match k {
            Kind::Main if self.main_ui.shade => RW::MainShade,
            Kind::Main => RW::Main,
            Kind::Eq if self.eq_ui.shade => RW::EqualizerShade,
            Kind::Eq => RW::Equalizer,
            _ => return,
        };
        if let Some(w) = &self.slot(k).win {
            crate::shape::set_shape(&w.window, self.skin.regions.get(which), w.zoom, w.user_zoom(), w.skin_size);
        }
    }

    fn apply_shapes(&self) {
        self.apply_shape(Kind::Main);
        self.apply_shape(Kind::Eq);
    }

    /// Nouvel agrandissement pour toutes les fenêtres (en gardant le groupe collé).
    fn set_zoom(&mut self, z: u8) {
        self.player.settings.window_zoom = z.min(3);
        self.player.settings.double_size = false;
        for k in KINDS {
            if k != Kind::Vis || !self.milk_fullscreen_now() {
                self.relayout(k);
            }
        }
        self.milk_reset_chrome();
    }

    fn toggle_shade(&mut self, k: Kind) {
        match k {
            Kind::Main => self.main_ui.shade = !self.main_ui.shade,
            Kind::Eq => self.eq_ui.shade = !self.eq_ui.shade,
            Kind::Playlist => self.pl_ui.shade = !self.pl_ui.shade,
            Kind::Gen | Kind::Library | Kind::Vis | Kind::Skins => return,
        }
        self.relayout(k);
    }

    fn start_move(&mut self, k: Kind) {
        let rects = self.rects();
        let group = if k == Kind::Main {
            layout::docked_group(&rects, Kind::Main as usize).into_iter().map(|i| KINDS[i]).collect()
        } else {
            vec![k]
        };
        self.moving = Some(Moving { kind: k, grab: self.slot(k).cursor, group });
    }

    /// Déplace la fenêtre saisie (et son groupe) en suivant la souris, avec aimantation.
    fn move_windows(&mut self, x: f64, y: f64) {
        let Some(m) = &self.moving else { return };
        let k = m.kind;
        if self.slot(k).pos.is_none() {
            return;
        }
        // Déplacement de la souris dans la fenêtre, converti en unités de disposition.
        let unit = self.slot(k).win.as_ref().map(|w| w.unit()).unwrap_or(1.0);
        let mut dx = ((x - m.grab.0) / unit).round() as i32;
        let mut dy = ((y - m.grab.1) / unit).round() as i32;
        if dx == 0 && dy == 0 {
            return;
        }
        let rects = self.rects();
        if !self.mods.shift_key() {
            let moving: Vec<R> = m.group.iter().filter_map(|&g| rects[g as usize]).map(|r| R::new(r.x + dx, r.y + dy, r.w, r.h)).collect();
            let targets: Vec<R> = KINDS.iter().filter(|g| !m.group.contains(g)).filter_map(|&g| rects[g as usize]).collect();
            let screens: Vec<R> = self
                .slot(k)
                .win
                .as_ref()
                .map(|w| w.window.available_monitors().map(|mo| monitor_rect(&mo)).collect())
                .unwrap_or_default();
            // Aimantation à 10 pixels skin.
            let per_skin_px = self.slot(k).win.as_ref().map(|w| w.layout_size().0 / w.skin_size.0.max(1) as i32).unwrap_or(1);
            let (sx, sy) = layout::snap(&moving, &targets, &screens, SNAP * per_skin_px.max(1));
            dx += sx;
            dy += sy;
        }
        let group = m.group.clone();
        for g in group {
            if let Some((gx, gy)) = self.slot(g).pos {
                self.set_pos(g, gx + dx, gy + dy);
            }
        }
    }

    // --- Commandes ------------------------------------------------------------------------------------

    fn command(&mut self, cmd: Command, event_loop: &ActiveEventLoop) {
        use Command::*;
        if self.connect_transport(&cmd) {
            self.request_redraw();
            return;
        }
        if self.milk_command(&cmd, event_loop) || self.plugin_command(&cmd, event_loop) {
            self.request_redraw();
            return;
        }
        match cmd {
            ToggleSkins => self.skins_toggle(event_loop),
            ToggleLibrary => {
                let open = !self.slot(Kind::Library).open;
                self.set_open(Kind::Library, open, event_loop);
                if open && self.lib_ui.items().is_empty() {
                    self.load_library_tab();
                }
            }
            SpotifyLogin => {
                self.spotify.login();
                if !self.slot(Kind::Library).open {
                    self.set_open(Kind::Library, true, event_loop);
                }
                self.load_library_tab();
            }
            SpotifyLogout => {
                self.spotify.logout();
                self.load_library_tab();
            }
            Previous => self.player.previous(),
            Play => self.player.play_pressed(),
            Pause => self.player.pause_pressed(),
            Stop => self.player.stop(),
            StopFade => self.player.stop_with_fadeout(),
            StopAfterCurrent => {
                self.player.stop_after_current = !self.player.stop_after_current;
                self.player.replan();
                self.main_ui.marquee.flash(if self.player.stop_after_current { "STOP AFTER CURRENT: ON" } else { "STOP AFTER CURRENT: OFF" });
            }
            Next => self.player.next(),
            First => self.player.first(),
            Last => self.player.last(),
            Back5 => self.player.seek_relative(-5000),
            Fwd5 => self.player.seek_relative(5000),
            Back10Tracks | Fwd10Tracks => {
                let n = self.player.playlist.len();
                if n > 0 {
                    let cur = self.player.playlist.current().unwrap_or(0) as i64;
                    let i = if cmd == Back10Tracks { (cur - 10).max(0) } else { (cur + 10).min(n as i64 - 1) } as usize;
                    self.player.play_index(i);
                }
            }
            VolumeUp | VolumeDown => {
                let d = if cmd == VolumeUp { 4 } else { -4 };
                let v = (self.player.settings.volume as i32 + d).clamp(0, 255) as u8;
                self.player.set_volume(v);
                self.main_ui.marquee.flash(&format!("VOLUME: {}%", (v as u32 * 100 + 127) / 255));
            }
            OpenFiles | PlAddFiles => {
                let files = dialogs::pick_files(self.player.settings.last_dir.as_deref());
                if let Some(d) = files.first().and_then(|f| f.parent()) {
                    self.player.settings.last_dir = Some(d.to_path_buf());
                }
                self.player.open(&files, cmd == OpenFiles);
            }
            OpenFolder | PlAddFolder => {
                if let Some(dir) = dialogs::pick_folder(self.player.settings.last_dir.as_deref()) {
                    self.player.settings.last_dir = Some(dir.clone());
                    self.player.open(&[dir], cmd == OpenFolder);
                }
            }
            LoadSkin => {
                if let Some(p) = dialogs::pick_skin() {
                    self.load_skin(&p);
                }
            }
            SkinFile(p) => self.load_skin(&p),
            DefaultSkin => {
                self.skin = self.default_skin.clone();
                self.cursors.reset();
                self.apply_shapes();
                self.player.set_plugin_colors(&self.skin.vis.0);
                self.player.settings.skin = None;
            }
            ToggleShuffle => self.player.toggle_shuffle(),
            ToggleRepeat => self.player.toggle_repeat(),
            ToggleManualAdvance => {
                self.player.toggle_manual_advance();
                self.main_ui.marquee.flash(if self.player.settings.manual_advance { "MANUAL ADVANCE: ON" } else { "MANUAL ADVANCE: OFF" });
            }
            ToggleTimeMode => self.player.settings.time_remaining = !self.player.settings.time_remaining,
            ToggleDoubleSize => {
                let z = if self.zoom() >= 2 { 1 } else { 2 };
                self.set_zoom(z);
            }
            WindowZoom(z) => self.set_zoom(z),
            ToggleAlwaysOnTop => {
                self.player.settings.always_on_top = !self.player.settings.always_on_top;
                let level = if self.player.settings.always_on_top { WindowLevel::AlwaysOnTop } else { WindowLevel::Normal };
                for s in &self.slots {
                    if let Some(w) = &s.win {
                        w.window.set_window_level(level);
                    }
                }
            }
            ToggleShade => {
                let k = self.focused.filter(|&k| !matches!(k, Kind::Gen | Kind::Library | Kind::Vis | Kind::Skins)).unwrap_or(Kind::Main);
                self.toggle_shade(k);
            }
            Vis(mode) => {
                self.player.settings.vis.mode = mode;
                if mode == VisMode::Off {
                    self.main_ui.vis.reset();
                }
            }
            FileInfo => {
                if let Some(i) = self.player.playlist.current() {
                    self.show_file_info(i);
                }
            }
            About => {
                let text = t("about.text").replace("{version}", env!("CARGO_PKG_VERSION"));
                dialogs::message("Alpagamp", &text);
            }
            Exit => {
                self.save();
                event_loop.exit();
            }
            Minimize => {
                self.milk_fullscreen(false);
                for k in [Kind::Eq, Kind::Playlist, Kind::Gen, Kind::Library, Kind::Vis, Kind::Skins] {
                    if let (true, Some(w)) = (self.slot(k).open, &self.slot(k).win) {
                        w.window.set_visible(false);
                    }
                }
                self.hidden_by_minimize = true;
                if let Some(w) = &self.slot(Kind::Main).win {
                    w.window.set_minimized(true);
                }
            }
            Language(l) => {
                self.player.settings.language = l.map(|l| l.code().to_string());
                apply_language(&self.player.settings);
                for k in KINDS {
                    if let Some(w) = &self.slot(k).win {
                        w.window.set_title(&self.title_for(k));
                    }
                }
            }
            ToggleEqWindow => {
                let open = !self.slot(Kind::Eq).open;
                self.set_open(Kind::Eq, open, event_loop);
            }
            TogglePlaylistWindow => {
                let open = !self.slot(Kind::Playlist).open;
                self.set_open(Kind::Playlist, open, event_loop);
            }
            JumpToFile => self.open_gen(GenKind::Jump, event_loop),
            PlAddUrl => self.open_gen(GenKind::AddUrl, event_loop),
            ToggleEq => {
                let mut eq = self.player.settings.eq;
                eq.enabled = !eq.enabled;
                self.player.set_eq(eq);
            }
            ToggleEqAuto => {
                let mut eq = self.player.settings.eq;
                eq.auto = !eq.auto;
                self.player.set_eq(eq);
                if eq.auto {
                    self.apply_auto_preset();
                }
            }
            EqPreset(i) => {
                if let Some(p) = factory_presets().get(i) {
                    let mut eq = self.player.settings.eq;
                    eq.apply_preset(p);
                    self.player.set_eq(eq);
                }
            }
            EqLoadFile => {
                if let Some(path) = dialogs::pick_eqf() {
                    match std::fs::read(&path).map_err(|e| e.to_string()).and_then(|b| read_eqf(&b).map_err(|e| e.to_string())) {
                        Ok(presets) if !presets.is_empty() => {
                            let mut eq = self.player.settings.eq;
                            eq.apply_preset(&presets[0]);
                            self.player.set_eq(eq);
                        }
                        Ok(_) => {}
                        Err(e) => dialogs::message(t("error.eqf"), &format!("{}\n\n{e}", path.display())),
                    }
                }
            }
            EqSaveFile => {
                if let Some(path) = dialogs::save_eqf() {
                    let eq = self.player.settings.eq;
                    let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Alpagamp".into());
                    let _ = std::fs::write(&path, write_eqf(&[Preset { name, preamp: eq.preamp, bands: eq.bands }]));
                }
            }
            EqSaveAuto => {
                if let Some(name) = self.current_file_name() {
                    let eq = self.player.settings.eq;
                    self.auto_presets.retain(|p| p.name != name);
                    self.auto_presets.push(Preset { name, preamp: eq.preamp, bands: eq.bands });
                    self.save_auto_presets();
                }
            }
            EqDeleteAuto => {
                if let Some(name) = self.current_file_name() {
                    self.auto_presets.retain(|p| p.name != name);
                    self.save_auto_presets();
                }
            }
            EqReset => {
                let mut eq = self.player.settings.eq;
                eq.preamp = CENTER;
                eq.bands = [CENTER; 10];
                self.player.set_eq(eq);
            }
            PlRemoveSelected => self.edit_playlist(|p| p.remove_selected()),
            PlCrop => self.edit_playlist(|p| p.crop()),
            PlRemoveAll | PlNew => self.edit_playlist(|p| p.clear()),
            PlRemoveMissing => self.edit_playlist(|p| p.remove_missing()),
            PlRemoveDuplicates => self.edit_playlist(|p| p.remove_duplicates()),
            PlSelectAll => self.player.playlist.select_all(),
            PlSelectNone => self.player.playlist.select_none(),
            PlInvertSelection => self.player.playlist.invert_selection(),
            PlSort(key) => self.edit_playlist(|p| p.sort(key)),
            PlReverse => self.edit_playlist(|p| p.reverse()),
            PlRandomize => self.edit_playlist(|p| p.randomize()),
            PlSave => {
                if let Some(path) = dialogs::save_playlist(self.player.settings.last_dir.as_deref()) {
                    let is_pls = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("pls"));
                    let text = if is_pls { write_pls(self.player.playlist.entries()) } else { write_m3u(self.player.playlist.entries(), false) };
                    let _ = std::fs::write(&path, text);
                }
            }
            PlLoad => {
                if let Some(path) = dialogs::pick_playlist(self.player.settings.last_dir.as_deref()) {
                    if let Ok(bytes) = std::fs::read(&path) {
                        let entries = read_playlist_bytes(&bytes, path.parent());
                        self.edit_playlist(move |p| {
                            p.clear();
                            p.extend(entries);
                        });
                    }
                }
            }
            PlFileInfo => {
                let pl = &self.player.playlist;
                if let Some(i) = pl.cursor().filter(|&c| pl.entries().get(c).is_some_and(|e| e.selected)).or_else(|| pl.selected_indices().first().copied()) {
                    self.show_file_info(i);
                }
            }
            PlPlaySelected => {
                let pl = &self.player.playlist;
                if let Some(i) = pl.cursor().or_else(|| pl.selected_indices().first().copied()) {
                    self.player.play_index(i);
                }
            }
            PlCursor(d, extend) => {
                self.player.playlist.move_cursor(d, extend);
                if let Some(c) = self.player.playlist.cursor() {
                    self.pl_ui.ensure_visible(c);
                }
            }
            PlMoveSelection(d) => {
                if self.player.playlist.move_selection(d) {
                    self.player.playlist_changed();
                    if let Some(c) = self.player.playlist.cursor() {
                        self.pl_ui.ensure_visible(c);
                    }
                }
            }
            Crossfade(ms) => {
                self.player.settings.playback.crossfade_ms = ms;
                self.player.apply_playback();
            }
            ReplayGain(mode) => {
                self.player.settings.playback.replay_gain = mode;
                self.player.apply_playback();
            }
            RgPreamp(db) => {
                self.player.settings.playback.rg_preamp_db = db as f32;
                self.player.apply_playback();
            }
            ToggleRgClipping => {
                let pb = &mut self.player.settings.playback;
                pb.rg_prevent_clipping = !pb.rg_prevent_clipping;
                self.player.apply_playback();
            }
            AddBookmark => {
                if let Some(e) = self.player.playlist.current_entry() {
                    let mut b = Entry::with_meta(e.source.clone(), Some(e.display_title()), e.duration_ms);
                    b.meta_loaded = true;
                    if !self.bookmarks.iter().any(|x| x.source == b.source) {
                        self.bookmarks.push(b);
                        self.save_bookmarks();
                    }
                    self.main_ui.marquee.flash("BOOKMARK ADDED");
                }
            }
            Bookmark(i) => {
                if let Some(b) = self.bookmarks.get(i).cloned() {
                    let pl = &mut self.player.playlist;
                    let idx = match pl.entries().iter().position(|e| e.source == b.source) {
                        Some(i) => i,
                        None => {
                            pl.extend([Entry::with_meta(b.source.clone(), b.title.clone(), b.duration_ms)]);
                            pl.len() - 1
                        }
                    };
                    self.player.playlist_changed();
                    self.player.play_index(idx);
                    self.pl_ui.ensure_visible(idx);
                }
            }
            RemoveBookmark(i) => {
                if i < self.bookmarks.len() {
                    self.bookmarks.remove(i);
                    self.save_bookmarks();
                }
            }
            ClearBookmarks => {
                self.bookmarks.clear();
                self.save_bookmarks();
            }
            VisAnalyzerStyle(st) => self.player.settings.vis.analyzer_style = st,
            VisThickBands(b) => self.player.settings.vis.thick_bands = b,
            ToggleVisPeaks => self.player.settings.vis.peaks = !self.player.settings.vis.peaks,
            VisFalloff(v) => self.player.settings.vis.falloff = v.min(4),
            VisPeakFalloff(v) => self.player.settings.vis.peak_falloff = v.min(4),
            VisScopeStyle(st) => self.player.settings.vis.scope_style = st,
            VisRefresh(v) => self.player.settings.vis.refresh = v.min(3),
            // Traitées par `milk_command`.
            ToggleMilk | MilkNext(_) | MilkPrevious | MilkToggleShuffle | MilkToggleLock | MilkPresets | MilkFullscreen
            | MilkLeaveFullscreen | MilkAuto(_) | MilkBlend(_) | MilkDownloadPack | MilkOpenFolder | MilkRescan => {}
            // Traitées par `plugin_command`.
            PluginToggle(_) | PluginOption(..) | PluginAbout(_) | PluginInstall | PluginsFolder | PluginsRescan => {}
        }
        self.menus_dirty = true;
        self.request_redraw();
    }

    fn edit_playlist(&mut self, f: impl FnOnce(&mut alpagamp_core::Playlist)) {
        f(&mut self.player.playlist);
        self.player.playlist_changed();
    }

    fn save_bookmarks(&self) {
        if let Some(dir) = self.paths.bookmarks.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&self.paths.bookmarks, write_m3u(&self.bookmarks, false));
    }

    fn save_auto_presets(&self) {
        if let Some(dir) = self.paths.eq_auto.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&self.paths.eq_auto, write_eqf(&self.auto_presets));
    }

    /// Charge le préréglage automatique du morceau courant, s'il y en a un.
    fn apply_auto_preset(&mut self) {
        if !self.player.settings.eq.auto {
            return;
        }
        let Some(name) = self.current_file_name() else { return };
        if let Some(p) = self.auto_presets.iter().find(|p| p.name == name).cloned() {
            let mut eq = self.player.settings.eq;
            eq.apply_preset(&p);
            self.player.set_eq(eq);
        }
    }

    fn open_gen(&mut self, kind: GenKind, event_loop: &ActiveEventLoop) {
        let mut g = GenWindow::new(kind);
        if kind == GenKind::Jump {
            let all: Vec<usize> = (0..self.player.playlist.len()).collect();
            g.set_results(all);
            g.take_query_changed();
            if let Some(c) = self.player.playlist.current() {
                g.select(c);
            }
        }
        let size = g.size();
        self.gen_ui = g;
        if kind == GenKind::Presets && !self.slot(Kind::Gen).open {
            let p = self.default_position(Kind::Gen);
            if let Some((x, y)) = p {
                self.set_pos(Kind::Gen, x, y);
            }
        }
        let title = self.title_for(Kind::Gen);
        let zoom = self.zoom();
        if let Some(w) = self.slot_mut(Kind::Gen).win.as_mut() {
            w.set_geometry(size, zoom);
            w.window.set_title(&title);
        }
        self.set_open(Kind::Gen, true, event_loop);
    }

    // --- Bibliothèque et Spotify ----------------------------------------------------------------------

    fn library_status(&self) -> String {
        if self.lib_pending.is_some() {
            return t("lib.loading").to_string();
        }
        if self.lib_ui.tab == LibTab::Radios {
            return String::new();
        }
        match self.spotify.status() {
            SpotifyStatus::LoggedOut => String::new(),
            SpotifyStatus::Authorizing => t("lib.authorizing").to_string(),
            SpotifyStatus::Connecting => t("lib.connecting").to_string(),
            SpotifyStatus::Connected { user } => format!("Spotify : {user}"),
            SpotifyStatus::Failed(e) => format!("Spotify : {e}"),
        }
    }

    /// Charge l'onglet courant de la Bibliothèque.
    fn load_library_tab(&mut self) {
        let tab = self.lib_ui.tab;
        self.lib_pending = None;
        let req = match tab {
            LibTab::Radios => Some(LibraryRequest::RadioTop),
            LibTab::Search => None,
            _ if !self.spotify.is_connected() => None,
            LibTab::Playlists => Some(LibraryRequest::Playlists { liked_label: t("lib.liked_title").to_string() }),
            LibTab::Liked => Some(LibraryRequest::Tracks("spotify:collection".into())),
            LibTab::Albums => Some(LibraryRequest::Albums),
        };
        let mut items = Vec::new();
        if tab != LibTab::Radios && !self.spotify.is_connected() {
            if !matches!(self.spotify.status(), SpotifyStatus::Authorizing | SpotifyStatus::Connecting) {
                items.push(LibraryItem::new(LibraryKind::Action, t("lib.login"), "", "action:login"));
            }
        }
        self.lib_ui.set_items(items);
        if let Some(r) = req {
            self.lib_pending = Some(self.spotify.request(r));
        }
    }

    fn library_action(&mut self, a: LibraryAction) {
        match a {
            LibraryAction::Tab(_) => self.load_library_tab(),
            LibraryAction::Open(item) => {
                self.lib_ui.push(&item.title, Vec::new());
                self.lib_pending = Some(self.spotify.request(LibraryRequest::Tracks(item.uri)));
            }
            LibraryAction::Search { tab, query } => {
                self.lib_ui.set_items(Vec::new());
                let r = if tab == LibTab::Radios { LibraryRequest::RadioSearch(query) } else { LibraryRequest::Search(query) };
                if tab == LibTab::Radios || self.spotify.is_connected() {
                    self.lib_pending = Some(self.spotify.request(r));
                } else {
                    self.load_library_tab();
                }
            }
            LibraryAction::Activate(item) => match item.uri.as_str() {
                "action:login" => self.spotify.login(),
                "action:logout" => self.spotify.logout(),
                _ => {}
            },
            LibraryAction::Play { items, start } => {
                let entries: Vec<Entry> = items.iter().map(library_entry).collect();
                if entries.is_empty() {
                    return;
                }
                self.player.stop();
                self.player.playlist.clear();
                self.player.playlist.extend(entries);
                self.player.playlist_changed();
                self.player.play_index(start.min(self.player.playlist.len() - 1));
            }
            LibraryAction::Enqueue(items) => {
                self.player.playlist.extend(items.iter().map(library_entry));
                self.player.playlist_changed();
            }
        }
    }

    /// Événements de Spotify.
    fn spotify_events(&mut self) {
        for ev in self.spotify.poll_events() {
            match ev {
                alpagamp_spotify::Event::Status(st) => {
                    if matches!(st, SpotifyStatus::Connected { .. }) && self.slot(Kind::Library).open && self.lib_ui.tab != LibTab::Radios {
                        self.load_library_tab();
                    } else if matches!(st, SpotifyStatus::Failed(_) | SpotifyStatus::LoggedOut) && self.lib_ui.tab != LibTab::Radios {
                        self.load_library_tab();
                    }
                    self.menus_dirty = true;
                }
                alpagamp_spotify::Event::ConnectStart => {
                    self.player.play_external(Media::Custom(self.spotify.connect_opener()), "Spotify Connect");
                }
                alpagamp_spotify::Event::ConnectPause => {
                    if self.player.is_external() {
                        self.player.engine.pause();
                    }
                }
                alpagamp_spotify::Event::ConnectResume => {
                    if self.player.is_external() {
                        self.player.engine.resume();
                    }
                }
                alpagamp_spotify::Event::ConnectStop => {
                    if self.player.is_external() {
                        self.player.stop();
                    }
                }
                alpagamp_spotify::Event::Library { request, result } if self.enqueue_pending == Some(request) => {
                    self.enqueue_pending = None;
                    match result {
                        Ok(items) => {
                            self.player.playlist.extend(items.iter().filter(|i| i.is_playable()).map(library_entry));
                            self.player.playlist_changed();
                        }
                        Err(e) => self.player.last_error = Some((e, Instant::now())),
                    }
                }
                alpagamp_spotify::Event::Library { request, result } => {
                    if self.lib_pending == Some(request) {
                        self.lib_pending = None;
                        match result {
                            Ok(items) if items.is_empty() => self.lib_ui.set_items(vec![LibraryItem::info(t("lib.empty"))]),
                            Ok(items) => self.lib_ui.set_items(items),
                            Err(e) => self.lib_ui.set_items(vec![LibraryItem::info(e)]),
                        }
                    }
                }
            }
        }
    }

    /// Spotify Connect pilote la lecture : les boutons de transport s'adressent à lui.
    fn connect_transport(&mut self, cmd: &Command) -> bool {
        if !self.player.is_external() {
            return false;
        }
        use Command::*;
        match cmd {
            Previous => self.spotify.connect_command(ConnectCommand::Previous),
            Next => self.spotify.connect_command(ConnectCommand::Next),
            Pause => {
                if self.player.state() == PlayState::Paused {
                    self.spotify.connect_command(ConnectCommand::Play);
                    self.player.engine.resume();
                } else {
                    self.spotify.connect_command(ConnectCommand::Pause);
                    self.player.engine.pause();
                }
            }
            Play => {
                self.spotify.connect_command(ConnectCommand::Play);
                self.player.engine.resume();
            }
            _ => return false,
        }
        true
    }

    fn show_file_info(&self, i: usize) {
        let Some(e) = self.player.playlist.entries().get(i) else { return };
        let probed;
        let info = if self.player.playing_index() == Some(i) && self.player.info.is_some() {
            self.player.info.as_ref()
        } else {
            probed = match &e.source {
                Source::File(p) => Decoder::open_file(p).ok().map(|d| d.info),
                _ => None,
            };
            probed.as_ref()
        };
        let mut lines = Vec::new();
        let mut add = |k: &str, v: Option<String>| {
            if let Some(v) = v.filter(|v| !v.is_empty()) {
                lines.push(format!("{} : {v}", t(k)));
            }
        };
        add("info.station", info.and_then(|i| i.stream_name.clone()));
        add("info.track_title", info.and_then(|i| i.title.clone()).or_else(|| e.title.clone()));
        add("info.artist", info.and_then(|i| i.artist.clone()));
        add("info.album", info.and_then(|i| i.album.clone()));
        add("info.year", info.and_then(|i| i.year.clone()));
        add("info.genre", info.and_then(|i| i.genre.clone()));
        add("info.length", e.duration_ms.or(info.and_then(|i| i.duration_ms)).map(format_duration));
        add(
            "info.format",
            info.map(|i| {
                format!(
                    "{} {} kbit/s, {} Hz, {}",
                    i.codec,
                    i.bitrate_kbps.map(|b| b.to_string()).unwrap_or("?".into()),
                    i.sample_rate,
                    if i.channels >= 2 { "stereo" } else { "mono" }
                )
            }),
        );
        add(
            "info.replaygain",
            info.and_then(|i| match (i.rg_track_gain, i.rg_album_gain) {
                (None, None) => None,
                (track, album) => Some(format!(
                    "{} {}",
                    track.map(|g| format!("{} {g:+.2} dB", t("rg.track"))).unwrap_or_default(),
                    album.map(|g| format!("{} {g:+.2} dB", t("rg.album"))).unwrap_or_default()
                )),
            }),
        );
        add("info.location", Some(e.source.to_playlist_string()));
        dialogs::message(t("info.title"), &lines.join("\n"));
    }

    // --- Rendu ----------------------------------------------------------------------------------------

    fn time_text(&self) -> String {
        let st = self.player.engine.status();
        if st.state == PlayState::Stopped {
            return String::new();
        }
        let dur = st.info.as_ref().and_then(|i| i.duration_ms);
        match (self.player.settings.time_remaining, dur) {
            (true, Some(d)) => format!("-{}", format_duration(d.saturating_sub(st.position_ms))),
            _ => format_duration(st.position_ms),
        }
    }

    fn redraw(&mut self, k: Kind) {
        let active = self.focused == Some(k);
        match k {
            Kind::Main => {
                let clock = self.start.elapsed().as_millis() as u64;
                let mut view = self.player.main_view(clock);
                view.active = active;
                view.eq_open = self.slot(Kind::Eq).open;
                view.pl_open = self.slot(Kind::Playlist).open;
                let s = &mut self.slots[0];
                self.main_ui.render(&mut s.fb, &self.skin, &view);
                if let Some(w) = s.win.as_mut() {
                    w.present(&s.fb, &[]);
                }
            }
            Kind::Eq => {
                let st = &self.player.settings;
                let view = EqView { eq: st.eq, active, volume: st.volume, balance: st.balance };
                let s = &mut self.slots[1];
                self.eq_ui.render(&mut s.fb, &self.skin, &view);
                if let Some(w) = s.win.as_mut() {
                    w.present(&s.fb, &[]);
                }
            }
            Kind::Playlist => {
                let time_text = self.time_text();
                let pl = &self.player.playlist;
                let view = PlaylistView { entries: pl.entries(), current: pl.current(), active, total: pl.total_time(), selected: pl.selected_time(), time_text };
                let s = &mut self.slots[2];
                self.pl_ui.render(&mut s.fb, &self.skin, &view);
                if let Some(w) = s.win.as_mut() {
                    w.present(&s.fb, &self.pl_ui.text);
                }
            }
            Kind::Gen => {
                let (title, prompt, ok, cancel) = match self.gen_ui.kind {
                    GenKind::Jump => (t("jump.title"), t("jump.prompt"), t("pl.play_item"), t("dialog.close")),
                    GenKind::AddUrl => (t("url.title"), t("url.prompt"), t("dialog.ok"), t("dialog.cancel")),
                    GenKind::Presets => (t("milk.presets"), t("milk.search"), t("milk.show"), t("dialog.close")),
                };
                let list = match self.gen_ui.kind {
                    GenKind::Presets => GenList::Items(&self.milk.items),
                    _ => GenList::Entries(self.player.playlist.entries()),
                };
                let view = GenView { list, active, title, prompt, ok_label: ok, cancel_label: cancel };
                let s = &mut self.slots[3];
                self.gen_ui.render(&mut s.fb, &self.skin, &view);
                if let Some(w) = s.win.as_mut() {
                    w.present(&s.fb, &self.gen_ui.text);
                }
            }
            Kind::Library => {
                let status = self.library_status();
                let prompt = if self.lib_ui.tab == LibTab::Radios { t("lib.radio_prompt") } else { t("lib.search_prompt") };
                let view = LibraryView {
                    active,
                    title: t("menu.library"),
                    tabs: [t("lib.tab_playlists"), t("lib.tab_liked"), t("lib.tab_albums"), t("lib.tab_search"), t("lib.tab_radios")],
                    prompt,
                    back: t("lib.back"),
                    play: t("pl.play_item"),
                    add: t("lib.add"),
                    status: &status,
                };
                let s = &mut self.slots[4];
                self.lib_ui.render(&mut s.fb, &self.skin, &view);
                if let Some(w) = s.win.as_mut() {
                    w.present(&s.fb, &self.lib_ui.text);
                }
            }
            Kind::Vis => self.milk_redraw(),
            Kind::Skins => self.skins_redraw(),
        }
    }

    fn milk_fullscreen_now(&self) -> bool {
        milk::MilkState::fullscreen(self)
    }

    fn handle_dropped(&mut self, event_loop: &ActiveEventLoop) {
        if self.dropped.is_empty() {
            return;
        }
        let files = std::mem::take(&mut self.dropped);
        let onto_playlist = files.iter().all(|(k, _)| *k == Kind::Playlist);
        let files: Vec<PathBuf> = files.into_iter().map(|(_, p)| p).collect();
        let ext = |p: &PathBuf| p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
        let (plugins, files): (Vec<PathBuf>, Vec<PathBuf>) = files.into_iter().partition(|p| ext(p) == "wasm");
        for p in &plugins {
            self.install_plugin(p, event_loop);
        }
        let (skins, media): (Vec<PathBuf>, Vec<PathBuf>) = files.into_iter().partition(|p| matches!(ext(p).as_str(), "wsz" | "wal" | "zip"));
        if let Some(s) = skins.last() {
            self.load_skin(s);
        }
        if !media.is_empty() {
            // Sur la playlist : ajout ; ailleurs : remplacement et lecture (comme Winamp).
            self.player.open(&media, !onto_playlist);
        }
    }

    fn mouse_down(&mut self, k: Kind, event_loop: &ActiveEventLoop) {
        let Some(w) = &self.slot(k).win else { return };
        let (cx, cy) = self.slot(k).cursor;
        let (x, y) = w.to_skin(cx, cy);
        let mods = self.mods();
        let now = Instant::now();
        let double = self.last_click.is_some_and(|(t, lk, lx, ly)| lk == k && now - t < DOUBLE_CLICK && (lx - x).abs() < 3 && (ly - y).abs() < 3);
        self.last_click = if double { None } else { Some((now, k, x, y)) };
        let actions = match k {
            Kind::Main => {
                let view = self.player.main_view(0);
                let mut a = if double { self.main_ui.double_click(x, y, mods) } else { Vec::new() };
                // Double-clic sur le visualiseur : trois changements de mode au total (aucun effet), et la
                // fenêtre de visualisation s'ouvre.
                if a.contains(&Action::ToggleVisPlugin) {
                    a.insert(0, Action::CycleVisMode);
                }
                if a.is_empty() {
                    a = self.main_ui.mouse_down(x, y, mods, &view);
                }
                a
            }
            Kind::Eq => {
                let view = EqView { eq: self.player.settings.eq, active: true, volume: self.player.settings.volume, balance: self.player.settings.balance };
                let mut a = if double { self.eq_ui.double_click(x, y) } else { Vec::new() };
                if a.is_empty() {
                    a = self.eq_ui.mouse_down(x, y, mods, &view);
                }
                a
            }
            Kind::Playlist => {
                if double {
                    let a = self.pl_ui.double_click(x, y);
                    if !a.is_empty() {
                        for a in a {
                            self.action(k, a, event_loop);
                        }
                        return;
                    }
                }
                self.pl_ui.mouse_down(x, y, mods, self.player.playlist.entries())
            }
            Kind::Gen => {
                let mut a = if double { self.gen_ui.double_click(x, y) } else { Vec::new() };
                if a.is_empty() {
                    a = self.gen_ui.mouse_down(x, y);
                }
                a
            }
            Kind::Library => {
                let mut a = if double { self.lib_ui.double_click(x, y) } else { Vec::new() };
                if a.is_empty() {
                    a = self.lib_ui.mouse_down(x, y);
                }
                a
            }
            // Plein écran : seul le double-clic compte (pour en sortir).
            Kind::Vis if self.milk_fullscreen_now() => {
                if double { vec![Action::MilkFullscreen] } else { Vec::new() }
            }
            Kind::Vis => {
                let mut a = if double { self.milk.ui.double_click(x, y) } else { Vec::new() };
                if a.is_empty() {
                    a = self.milk.ui.mouse_down(x, y);
                }
                a
            }
            Kind::Skins => self.skins.ui.mouse_down(x, y, SKINS_OPEN),
        };
        for a in actions {
            self.action(k, a, event_loop);
        }
    }

    fn mouse_up(&mut self, k: Kind, event_loop: &ActiveEventLoop) {
        if self.moving.take().is_some() {
            self.save();
            return;
        }
        let Some(w) = &self.slot(k).win else { return };
        let (cx, cy) = self.slot(k).cursor;
        let (x, y) = w.to_skin(cx, cy);
        let mods = self.mods();
        let actions = match k {
            Kind::Main => {
                let view = self.player.main_view(0);
                self.main_ui.mouse_up(x, y, mods, &view)
            }
            Kind::Eq => {
                let view = EqView { eq: self.player.settings.eq, active: true, volume: self.player.settings.volume, balance: self.player.settings.balance };
                self.eq_ui.mouse_up(x, y, mods, &view)
            }
            Kind::Playlist => self.pl_ui.mouse_up(x, y, mods),
            Kind::Gen => self.gen_ui.mouse_up(x, y),
            Kind::Library => self.lib_ui.mouse_up(x, y),
            Kind::Vis => self.milk.ui.mouse_up(x, y),
            Kind::Skins => self.skins.ui.mouse_up(x, y, SKINS_OPEN),
        };
        for a in actions {
            self.action(k, a, event_loop);
        }
    }

    fn mouse_move(&mut self, k: Kind, event_loop: &ActiveEventLoop) {
        let Some(w) = &self.slot(k).win else { return };
        let (cx, cy) = self.slot(k).cursor;
        let (x, y) = w.to_skin(cx, cy);
        let mods = self.mods();
        let actions = match k {
            Kind::Main => {
                let view = self.player.main_view(0);
                self.main_ui.mouse_move(x, y, mods, &view)
            }
            Kind::Eq => {
                let view = EqView { eq: self.player.settings.eq, active: true, volume: self.player.settings.volume, balance: self.player.settings.balance };
                self.eq_ui.mouse_move(x, y, mods, &view)
            }
            Kind::Playlist => self.pl_ui.mouse_move(x, y, mods),
            Kind::Gen => {
                self.gen_ui.mouse_move(x, y);
                Vec::new()
            }
            Kind::Library => {
                self.lib_ui.mouse_move(x, y);
                Vec::new()
            }
            Kind::Vis => self.milk.ui.mouse_move(x, y),
            Kind::Skins => {
                self.skins.ui.mouse_move(x, y, SKINS_OPEN);
                Vec::new()
            }
        };
        for a in actions {
            self.action(k, a, event_loop);
        }
    }

    fn is_dragging(&self) -> bool {
        self.moving.is_some() || self.main_ui.is_dragging() || self.eq_ui.is_dragging() || self.pl_ui.is_dragging() || self.gen_ui.is_dragging() || self.lib_ui.is_dragging() || self.milk.ui.is_dragging() || self.skins.ui.is_dragging()
    }

    /// Ouvre les fenêtres au démarrage.
    fn startup(&mut self, event_loop: &ActiveEventLoop) {
        // Positions enregistrées hors de tout écran (écran débranché) : on laisse le système placer.
        // Agrandissement suggéré par l'écran principal (en points).
        if let Some(m) = event_loop.primary_monitor().or_else(|| event_loop.available_monitors().next()) {
            self.auto_zoom = Settings::auto_zoom(m.size().width as f64 / m.scale_factor().max(0.5));
        }
        if let Some((x, y)) = self.slot(Kind::Main).pos {
            let on_screen = event_loop.available_monitors().any(|m| {
                let r = monitor_rect(&m);
                x + 50 > r.x && x < r.x + r.w - 20 && y + 10 > r.y && y < r.y + r.h - 20
            });
            if !on_screen {
                for s in &mut self.slots {
                    s.pos = None;
                }
            }
        }
        self.create_window(Kind::Main, event_loop);
        if self.slot(Kind::Main).win.is_none() {
            event_loop.exit();
            return;
        }
        for k in [Kind::Eq, Kind::Playlist] {
            if self.slot(k).open {
                if self.slot(k).pos.is_none() {
                    let p = self.default_position(k);
                    self.slot_mut(k).pos = p;
                }
                self.create_window(k, event_loop);
            }
        }
        if self.slot(Kind::Vis).open {
            self.slot_mut(Kind::Vis).open = false;
            self.milk_set_open(true, event_loop);
        }
        if let Some(w) = &self.slot(Kind::Main).win {
            w.window.focus_window();
        }
        self.refresh_menus();
        let args = std::mem::take(&mut self.pending_args);
        if !args.is_empty() {
            self.dropped = args.into_iter().map(|p| (Kind::Main, p)).collect();
            self.handle_dropped(event_loop);
        }
        self.request_redraw();
    }
}

/// Rectangle d'un écran, en unités de disposition (points sur macOS).
fn monitor_rect(m: &winit::monitor::MonitorHandle) -> R {
    let u = if crate::window::POINTS { m.scale_factor().max(0.5) } else { 1.0 };
    let (p, s) = (m.position(), m.size());
    R::new((p.x as f64 / u).round() as i32, (p.y as f64 / u).round() as i32, (s.width as f64 / u).round() as i32, (s.height as f64 / u).round() as i32)
}

/// Nom de l'appareil dans Spotify Connect.
fn device_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "Alpagamp (Mac)"
    } else if cfg!(target_os = "windows") {
        "Alpagamp (PC)"
    } else {
        "Alpagamp (Linux)"
    }
}

/// Entrée de playlist pour un élément de la Bibliothèque.
fn library_entry(it: &LibraryItem) -> Entry {
    let src = Source::parse(&it.uri, None);
    let title = if it.kind == LibraryKind::Radio { it.title.clone() } else { it.playlist_title() };
    let mut e = Entry::with_meta(src, Some(title), it.duration_ms);
    e.meta_loaded = true;
    e
}

fn apply_language(s: &Settings) {
    let lang = s
        .language
        .as_deref()
        .and_then(Lang::from_tag)
        .or_else(|| sys_locale::get_locale().as_deref().and_then(Lang::from_tag))
        .unwrap_or(Lang::En);
    i18n::set_lang(lang);
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.started {
            self.started = true;
            self.startup(event_loop);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(k) = self.kind_of(id) else { return };
        match event {
            WindowEvent::CloseRequested => self.action(k, Action::Close, event_loop),
            WindowEvent::RedrawRequested => self.redraw(k),
            WindowEvent::Focused(f) => {
                if f {
                    self.focused = Some(k);
                    if k == Kind::Main && self.hidden_by_minimize {
                        self.hidden_by_minimize = false;
                        for o in [Kind::Eq, Kind::Playlist, Kind::Gen, Kind::Library, Kind::Vis, Kind::Skins] {
                            if let (true, Some(w)) = (self.slot(o).open, &self.slot(o).win) {
                                w.window.set_visible(true);
                            }
                        }
                    }
                } else if self.focused == Some(k) {
                    self.focused = None;
                }
                self.request_redraw();
            }
            WindowEvent::Moved(p) => {
                // Position réelle (déplacement par le système, ou confirmation du nôtre) ; pas en plein écran.
                if self.moving.is_none() && !(k == Kind::Vis && self.milk_fullscreen_now()) {
                    let lp = self.slot(k).win.as_ref().map(|w| w.layout_from_physical(p)).unwrap_or((p.x, p.y));
                    self.slot_mut(k).pos = Some(lp);
                }
            }
            WindowEvent::Resized(_) if k == Kind::Vis => {
                if let Some(w) = &self.slot(k).win {
                    w.window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(w) = self.slot_mut(k).win.as_mut() {
                    w.rescale();
                }
                self.apply_shape(k);
            }
            WindowEvent::Resized(_) if matches!(k, Kind::Main | Kind::Eq) => self.apply_shape(k),
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
            WindowEvent::CursorMoved { position, .. } => {
                self.slot_mut(k).cursor = (position.x, position.y);
                if self.moving.as_ref().is_some_and(|m| m.kind == k) {
                    self.move_windows(position.x, position.y);
                    return;
                }
                self.mouse_move(k, event_loop);
                self.update_cursor(k, event_loop);
                if let Some(w) = &self.slot(k).win {
                    w.window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                match state {
                    ElementState::Pressed => self.mouse_down(k, event_loop),
                    ElementState::Released => self.mouse_up(k, event_loop),
                }
                self.request_redraw();
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } => {
                let which = if k == Kind::Vis {
                    Popup::Milk
                } else if k == Kind::Playlist && !self.pl_ui.shade {
                    let (cx, cy) = self.slot(k).cursor;
                    let (x, y) = self.slot(k).win.as_ref().map(|w| w.to_skin(cx, cy)).unwrap_or((0, 0));
                    // Clic droit sur une ligne : la sélectionner si elle ne l'est pas, puis menu de la liste.
                    let row = self.pl_ui.row_at(x, y);
                    if let Some(i) = row {
                        if !self.player.playlist.entries()[i].selected {
                            self.player.playlist.click(i, false, false);
                        }
                    }
                    if row.is_some() || self.pl_ui.in_list(x, y) { Popup::PlContext } else { Popup::Main }
                } else {
                    Popup::Main
                };
                self.request_redraw();
                self.popup(which, k);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => (p.y / 20.0) as f32,
                };
                if lines.abs() > 0.0 {
                    match k {
                        Kind::Main => {
                            let view = self.player.main_view(0);
                            for a in self.main_ui.wheel(lines, &view) {
                                self.action(k, a, event_loop);
                            }
                        }
                        Kind::Playlist => self.pl_ui.wheel(lines),
                        Kind::Gen => self.gen_ui.wheel(lines),
                        Kind::Library => self.lib_ui.wheel(lines),
                        Kind::Skins => self.skins.ui.wheel(lines),
                        Kind::Eq | Kind::Vis => {}
                    }
                    self.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                if k == Kind::Skins {
                    if let Some(gk) = keys::gen_key(&event, self.mods) {
                        for a in self.skins.ui.key(gk) {
                            self.action(Kind::Skins, a, event_loop);
                        }
                        self.request_redraw();
                    }
                    return;
                }
                if k == Kind::Library {
                    if let Some(gk) = keys::gen_key(&event, self.mods) {
                        for a in self.lib_ui.key(gk) {
                            self.action(Kind::Library, a, event_loop);
                        }
                        self.request_redraw();
                    }
                    return;
                }
                if k == Kind::Gen {
                    if let Some(gk) = keys::gen_key(&event, self.mods) {
                        for a in self.gen_ui.key(gk) {
                            self.action(Kind::Gen, a, event_loop);
                        }
                        if self.gen_ui.take_query_changed() {
                            if self.gen_ui.kind == GenKind::Presets {
                                let q = self.gen_ui.query.clone();
                                self.milk_search(&q);
                            } else {
                                let r = self.player.playlist.search(&self.gen_ui.query);
                                self.gen_ui.set_results(r);
                            }
                        }
                        self.request_redraw();
                    }
                    return;
                }
                let focus = match k {
                    Kind::Playlist => Focus::Playlist,
                    Kind::Eq => Focus::Eq,
                    Kind::Vis => Focus::Milk,
                    _ => Focus::Main,
                };
                if let Some(c) = keys::map(&event, self.mods, focus) {
                    self.command(c, event_loop);
                }
            }
            WindowEvent::DroppedFile(p) => self.dropped.push((k, p)),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_dropped(event_loop);
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let cmds = self.menus.as_ref().map(|m| m.poll()).unwrap_or_default();
            for c in cmds {
                self.command(c, event_loop);
            }
        }
        self.spotify_events();
        self.milk_tick();
        self.skins_tick(event_loop);
        self.player.tick();
        if self.player.take_track_changed().is_some() {
            self.apply_auto_preset();
            if let Some(i) = self.player.playlist.current() {
                self.pl_ui.ensure_visible(i);
            }
            self.menus_dirty = true;
        }
        // Message de l'égaliseur dans le titre défilant.
        match &self.eq_ui.status {
            Some(s) => {
                self.main_ui.marquee.set_status(s.clone());
                self.eq_status_shown = true;
            }
            None if self.eq_status_shown => {
                self.main_ui.marquee.clear_status();
                self.eq_status_shown = false;
            }
            None => {}
        }
        if self.menus_dirty && !self.is_dragging() {
            self.menus_dirty = false;
            self.refresh_menus();
        }
        // Une image toutes les 16 ms en lecture (100 ms sinon). `about_to_wait` est aussi appelé après chaque lot
        // d'événements : redemander un dessin à chaque fois ferait tourner la boucle à vide (et, sous Windows, une
        // seule fenêtre recevrait ses WM_PAINT).
        let now = Instant::now();
        let playing = self.player.state() == PlayState::Playing;
        let milk_visible = self.slot(Kind::Vis).visible() && self.milk.engine.is_some();
        let interval = Duration::from_millis(if playing || milk_visible || self.is_dragging() { 16 } else { 100 });
        if now >= self.next_frame {
            let dt = (now - self.last_frame).as_secs_f32().min(0.25);
            self.last_frame = now;
            if playing {
                self.player.engine.vis_latest(&mut self.vis_l, &mut self.vis_r);
                self.main_ui.vis.analyze(&self.vis_l, &self.vis_r);
            } else {
                self.main_ui.vis.analyze(&[], &[]);
            }
            self.main_ui.vis.advance(dt, &self.player.settings.vis);
            self.main_ui.marquee.tick(now);
            self.request_redraw();
            self.next_frame = now + interval;
        } else if self.next_frame > now + interval {
            // Passage de l'attente à la lecture : ne pas attendre la fin d'un long intervalle.
            self.next_frame = now + interval;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Les plugins s'arrêtent d'abord (la minuterie rend le volume avant l'enregistrement).
        self.player.shutdown_plugins();
        self.save();
    }
}
