//! L'appli Android : une seule surface où sont empilées la fenêtre principale, l'égaliseur et la playlist,
//! agrandies à la largeur de l'écran ; menus et boîtes en surimpression (fenêtres génériques skinnées) ;
//! visualisation MilkDrop en plein écran (module `milk`).

mod milk;
mod plugins;

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use alpagamp_audio::{Media, PlayState};
use alpagamp_core::library::{LibraryItem, LibraryKind};
use alpagamp_spotify::{ConnectCommand, Event as SpotifyEvent, LibraryRequest, Spotify, Status as SpotifyStatus};
use alpagamp_core::eq::{CENTER, factory_presets};
use alpagamp_core::i18n::{self, Lang, t};
use alpagamp_core::playlist::SortKey;
use alpagamp_core::settings::{AnalyzerStyle, ReplayGainMode, ScopeStyle, VisMode};
use alpagamp_core::time::format_duration;
use alpagamp_core::{Entry, Settings, Source};
use alpagamp_player::Player;
use alpagamp_skin::{Image, Skin};
use alpagamp_ui::text::{TextRun, draw_runs_at};
use alpagamp_museum::{Apply as SkinApply, Browser as SkinBrowser};
use alpagamp_ui::{SkinsView, SkinsWindow};
use alpagamp_ui::{
    Action, EqView, EqWindow, GenKey, GenKind, GenList, GenView, GenWindow, LibTab, LibraryAction, LibraryView, LibraryWindow, MainWindow, MenuItem,
    MenuResult, MenuWindow, Mods, PlMenuItem, PlaylistView, PlaylistWindow,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::host::Host;

/// Quelques radios libres d'accès, pour écouter sans rien taper (l'annuaire complet arrive avec la Bibliothèque).
const RADIOS: &[(&str, &str)] = &[
    ("SomaFM – Groove Salad", "https://ice1.somafm.com/groovesalad-128-mp3"),
    ("SomaFM – Drone Zone", "https://ice1.somafm.com/dronezone-128-mp3"),
    ("SomaFM – Secret Agent", "https://ice1.somafm.com/secretagent-128-mp3"),
    ("SomaFM – Indie Pop Rocks!", "https://ice1.somafm.com/indiepop-128-mp3"),
    ("FIP", "https://icecast.radiofrance.fr/fip-midfi.mp3"),
    ("France Inter", "https://icecast.radiofrance.fr/franceinter-midfi.mp3"),
    ("Radio Paradise", "https://stream.radioparadise.com/mp3-128"),
    ("KEXP Seattle", "https://kexp-mp3-128.streamguys1.com/kexp128.mp3"),
];

const DOUBLE_TAP: Duration = Duration::from_millis(400);
const LONG_PRESS: Duration = Duration::from_millis(600);
/// Au-delà (pixels skin), un toucher devient un glissement.
const SLOP: f64 = 4.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Region {
    Main,
    Eq,
    Playlist,
    Overlay,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MenuKind {
    Main,
    EqPresets,
    Playlist,
    Info,
    /// Menu de la visualisation (appui long dessus).
    Milk,
}

enum Overlay {
    Gen(GenWindow),
    Menu(MenuWindow, MenuKind),
    Library,
    /// Navigateur de skins.
    Skins,
}

struct Touch {
    id: u64,
    region: Region,
    start: (f64, f64),
    last: (f64, f64),
    at: Instant,
    moved: bool,
    /// Commencé dans la liste de la playlist (glisser = faire défiler).
    list: bool,
    scroll_acc: f64,
    long_done: bool,
}

/// Disposition à l'écran : facteur d'agrandissement et ordonnée (pixels) de chaque région.
#[derive(Clone, Debug, Default)]
struct Layout {
    scale: f32,
    w: u32,
    h: u32,
    regions: Vec<(Region, f32, u32)>,
}

impl Layout {
    fn region_at(&self, y: f64) -> Option<(Region, f32)> {
        self.regions.iter().rev().find(|(_, y0, h)| y >= *y0 as f64 && y < *y0 as f64 + *h as f64 * self.scale as f64).map(|(r, y0, _)| (*r, *y0))
    }
    fn origin(&self, r: Region) -> Option<f32> {
        self.regions.iter().find(|(k, _, _)| *k == r).map(|(_, y, _)| *y)
    }
}

pub struct Mobile {
    host: Box<dyn Host>,
    /// Position de la souris (simulateur sur ordinateur).
    cursor: (f64, f64),
    mouse_down: bool,
    sim_size: Option<(u32, u32)>,
    window: Option<Rc<Window>>,
    screen: Option<milk::Screen>,
    milk: milk::MobileMilk,
    player: Player,
    skin: Skin,
    default_skin: Skin,
    main_ui: MainWindow,
    eq_ui: EqWindow,
    pl_ui: PlaylistWindow,
    overlay: Option<Overlay>,
    lib_ui: LibraryWindow,
    skins: SkinBrowser,
    spotify: Arc<Spotify>,
    lib_pending: Option<u64>,
    enqueue_pending: Option<u64>,
    fbs: [Image; 4],
    eq_open: bool,
    pl_open: bool,
    data: PathBuf,
    layout: Layout,
    touch: Option<Touch>,
    last_tap: Option<(Instant, Region, i32, i32)>,
    start: Instant,
    last_frame: Instant,
    next_frame: Instant,
    vis_l: Vec<f32>,
    vis_r: Vec<f32>,
    /// Une recherche de musique attend la permission.
    scan_pending: bool,
    asked_permission: bool,
    eq_status_shown: bool,
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

impl Mobile {
    pub fn new(host: Box<dyn Host>) -> Mobile {
        let data = host.data_dir();
        let settings = Settings::load(&data.join("settings.toml"));
        apply_language(&settings);
        let mut player = Player::new(settings);
        player.load_playlist_file(&data.join("playlist.m3u8"));
        let spotify = Arc::new(Spotify::new(data.join("spotify"), "Alpagamp (Android)", true, host.url_opener()));
        let sp = spotify.clone();
        player.resolver = Some(Box::new(move |src: &Source| match src {
            Source::Spotify(uri) => Some(Media::Custom(sp.opener(uri))),
            _ => None,
        }));
        let default_skin = Skin::default_skin();
        let skin = player
            .settings
            .skin
            .as_ref()
            .and_then(|p| std::fs::read(p).ok().and_then(|b| Skin::load_wsz("", &b, &default_skin).ok()))
            .unwrap_or_else(|| default_skin.clone());
        player.init_plugins(data.join("plugins"));
        player.set_plugin_colors(&skin.vis.0);
        let skins = SkinBrowser::new(data.join("skins"), SkinsWindow::default());
        let eq_open = player.settings.eq_window.map(|w| w.open).unwrap_or(true);
        let pl_open = player.settings.playlist_window.map(|w| w.open).unwrap_or(true);
        Mobile {
            host,
            cursor: (0.0, 0.0),
            mouse_down: false,
            sim_size: None,
            window: None,
            screen: None,
            milk: milk::MobileMilk::default(),
            player,
            skin,
            default_skin,
            main_ui: MainWindow::new(),
            eq_ui: EqWindow::new(),
            pl_ui: PlaylistWindow::default(),
            overlay: None,
            lib_ui: LibraryWindow::default(),
            skins,
            spotify,
            lib_pending: None,
            enqueue_pending: None,
            fbs: [Image::new(1, 1, 0), Image::new(1, 1, 0), Image::new(1, 1, 0), Image::new(1, 1, 0)],
            eq_open,
            pl_open,
            data,
            layout: Layout::default(),
            touch: None,
            last_tap: None,
            start: Instant::now(),
            last_frame: Instant::now(),
            next_frame: Instant::now(),
            vis_l: vec![0.0; alpagamp_ui::vis::WINDOW],
            vis_r: vec![0.0; alpagamp_ui::vis::WINDOW],
            scan_pending: false,
            asked_permission: false,
            eq_status_shown: false,
        }
    }

    fn save(&mut self) {
        let st = &mut self.player.settings;
        let ws = |open: bool| alpagamp_core::settings::WindowState { x: 0, y: 0, open, shade: false, w: 275, h: 116 };
        st.eq_window = Some(ws(self.eq_open));
        st.playlist_window = Some(ws(self.pl_open));
        st.resume_index = self.player.playlist.current();
        let _ = st.save(&self.data.join("settings.toml"));
        self.player.save_playlist_file(&self.data.join("playlist.m3u8"));
    }

    // --- Disposition et rendu ---------------------------------------------------------------------

    fn relayout(&mut self) {
        let Some(w) = &self.window else { return };
        let size = w.inner_size();
        let (pw, ph) = (size.width.max(1), size.height.max(1));
        // Zone visible (sans la barre d'état, la barre de navigation ni le clavier).
        let (top, bottom) = match self.host.content_rect() {
            Some((t, b)) if b > t && b as u32 <= ph && t >= 0 => (t as f32, b as f32),
            _ => (0.0, ph as f32),
        };
        let scale = pw as f32 / 275.0;
        let mut regions = Vec::new();
        let mut y = top;
        let mh = self.main_ui.size().1;
        regions.push((Region::Main, y, mh));
        y += mh as f32 * scale;
        if self.eq_open {
            let eh = self.eq_ui.size().1;
            regions.push((Region::Eq, y, eh));
            y += eh as f32 * scale;
        }
        if self.pl_open {
            if self.pl_ui.shade {
                regions.push((Region::Playlist, y, 14));
            } else {
                // La playlist remplit le reste, au pas de 29 du format.
                let avail = ((bottom - y) / scale).floor() as i32;
                let k = ((avail - 116) / 29).max(0) as u32;
                self.pl_ui.w = 275;
                self.pl_ui.h = 116 + 29 * k;
                regions.push((Region::Playlist, y, self.pl_ui.h));
            }
        }
        if let Some(o) = &self.overlay {
            let oh = match o {
                Overlay::Gen(g) => g.size().1,
                Overlay::Menu(m, _) => m.size().1,
                Overlay::Library => {
                    // La Bibliothèque prend la hauteur disponible.
                    let avail = ((bottom - top) / scale) as i32;
                    self.lib_ui.set_height((116 + ((avail - 116) / 29).max(0) * 29) as u32);
                    self.lib_ui.size().1
                }
                Overlay::Skins => {
                    let avail = ((bottom - top) / scale) as i32;
                    self.skins.ui.set_height((116 + ((avail - 116) / 29).max(0) * 29) as u32);
                    self.skins.ui.size().1
                }
            };
            regions.push((Region::Overlay, top, oh));
        }
        self.layout = Layout { scale, w: pw, h: ph, regions };
    }

    /// Hauteur skin disponible pour une surimpression.
    fn overlay_max_h(&self) -> u32 {
        if self.layout.scale > 0.0 { (self.layout.h as f32 / self.layout.scale) as u32 } else { 400 }
    }

    fn redraw(&mut self) {
        self.relayout();
        match self.screen {
            None => {}
            Some(milk::Screen::Gpu(_)) => self.redraw_gpu(),
            Some(milk::Screen::Soft(_)) => self.redraw_soft(),
        }
    }

    /// Dessine chaque fenêtre à l'échelle 1 ; renvoie le texte de la surimpression.
    fn render_windows(&mut self) -> Vec<TextRun> {
        let clock = self.start.elapsed().as_millis() as u64;
        let mut mv = self.player.main_view(clock);
        mv.active = self.overlay.is_none();
        mv.eq_open = self.eq_open;
        mv.pl_open = self.pl_open;
        self.main_ui.render(&mut self.fbs[0], &self.skin, &mv);
        let st = &self.player.settings;
        let ev = EqView { eq: st.eq, active: false, volume: st.volume, balance: st.balance };
        self.eq_ui.render(&mut self.fbs[1], &self.skin, &ev);
        let time_text = time_text(&self.player);
        let pl = &self.player.playlist;
        let pv = PlaylistView { entries: pl.entries(), current: pl.current(), active: false, total: pl.total_time(), selected: pl.selected_time(), time_text };
        self.pl_ui.render(&mut self.fbs[2], &self.skin, &pv);
        match &mut self.overlay {
            Some(Overlay::Gen(g)) => {
                let (title, prompt, ok, cancel) = match g.kind {
                    GenKind::Jump => (t("jump.title"), t("jump.prompt"), t("pl.play_item"), t("dialog.close")),
                    GenKind::AddUrl => (t("url.title"), t("url.prompt"), t("dialog.ok"), t("dialog.cancel")),
                    GenKind::Presets => (t("milk.presets"), t("milk.search"), t("milk.show"), t("dialog.close")),
                };
                let list = match g.kind {
                    GenKind::Presets => GenList::Items(&self.milk.items),
                    _ => GenList::Entries(self.player.playlist.entries()),
                };
                let gv = GenView { list, active: true, title, prompt, ok_label: ok, cancel_label: cancel };
                g.render(&mut self.fbs[3], &self.skin, &gv);
                g.text.clone()
            }
            Some(Overlay::Menu(m, _)) => {
                m.render(&mut self.fbs[3], &self.skin);
                m.text.clone()
            }
            Some(Overlay::Library) => {
                let status = library_status(&self.spotify, &self.lib_ui, self.lib_pending.is_some());
                let prompt = if self.lib_ui.tab == LibTab::Radios { t("lib.radio_prompt") } else { t("lib.search_prompt") };
                let view = LibraryView {
                    active: true,
                    title: t("menu.library"),
                    tabs: [t("lib.tab_playlists"), t("lib.tab_liked"), t("lib.tab_albums"), t("lib.tab_search"), t("lib.tab_radios")],
                    prompt,
                    back: t("lib.back"),
                    play: t("pl.play_item"),
                    add: t("lib.add"),
                    status: &status,
                };
                self.lib_ui.render(&mut self.fbs[3], &self.skin, &view);
                self.lib_ui.text.clone()
            }
            Some(Overlay::Skins) => {
                let status = self.skins.status();
                let view = SkinsView {
                    active: true,
                    title: t("skins.title"),
                    tabs: [t("skins.museum"), t("skins.mine")],
                    prompt: t("skins.search"),
                    open: "",
                    close: t("dialog.close"),
                    status: &status,
                };
                self.skins.ui.render(&mut self.fbs[3], &self.skin, &view);
                self.skins.ui.text.clone()
            }
            None => Vec::new(),
        }
    }

    /// Compose les fenêtres agrandies dans `buf` (`w`×`h`). `only_overlay` : seulement la surimpression.
    /// `mask` est appliqué aux pixels (alpha à 0 pour softbuffer, à 255 pour un calque mélangé).
    fn compose(&self, buf: &mut [u32], w: u32, h: u32, overlay_text: &[TextRun], only_overlay: bool, mask: u32) {
        let scale = self.layout.scale;
        let xmap: Vec<usize> = (0..w).map(|x| ((x as f32 / scale) as usize).min(274)).collect();
        let set_alpha = mask >> 24 == 0xFF;
        for &(region, y0, rh) in &self.layout.regions {
            if only_overlay && region != Region::Overlay {
                continue;
            }
            let img = match region {
                Region::Main => &self.fbs[0],
                Region::Eq => &self.fbs[1],
                Region::Playlist => &self.fbs[2],
                Region::Overlay => &self.fbs[3],
            };
            let ystart = y0.round() as u32;
            let yend = ((y0 + rh as f32 * scale).round() as u32).min(h);
            for y in ystart..yend {
                let sy = (((y as f32 - y0) / scale) as u32).min(img.height.saturating_sub(1));
                let src = &img.pixels[(sy * img.width) as usize..((sy + 1) * img.width) as usize];
                let dst = &mut buf[(y * w) as usize..((y + 1) * w) as usize];
                for (x, d) in dst.iter_mut().enumerate() {
                    let p = src[xmap[x].min(src.len() - 1)];
                    *d = if set_alpha { p | 0xFF00_0000 } else { p & mask };
                }
            }
            let runs = match region {
                Region::Playlist => &self.pl_ui.text,
                Region::Overlay => overlay_text,
                _ => continue,
            };
            draw_runs_at(buf, w, h, scale, (0.0, y0), runs);
        }
    }

    fn redraw_soft(&mut self) {
        let (w, h) = (self.layout.w, self.layout.h);
        let (Some(nw), Some(nh)) = (NonZeroU32::new(w), NonZeroU32::new(h)) else { return };
        let overlay_text = self.render_windows();
        // Visualisation de plugin en plein écran (sans GPU : agrandie par le processeur).
        let plugin_vis = self.milk.on && self.player.plugins.vis_running() && self.plugin_vis_frame(w, h);
        let mut screen = self.screen.take();
        if let Some(milk::Screen::Soft(surface)) = screen.as_mut() {
            if surface.resize(nw, nh).is_ok() {
                if let Ok(mut buf) = surface.buffer_mut() {
                    if plugin_vis {
                        self.milk.blit_plugin(&mut buf, w, h);
                    } else {
                        buf.fill(0);
                    }
                    self.compose(&mut buf, w, h, &overlay_text, plugin_vis, 0x00FF_FFFF);
                    let _ = buf.present();
                }
            }
        }
        self.screen = screen;
    }

    // --- Toucher ----------------------------------------------------------------------------------

    fn to_skin(&self, x: f64, y: f64, y0: f32) -> (i32, i32) {
        let s = self.layout.scale.max(0.01) as f64;
        ((x / s).floor() as i32, ((y - y0 as f64) / s).floor() as i32)
    }

    fn touch_start(&mut self, id: u64, x: f64, y: f64, el: &ActiveEventLoop) {
        if self.touch.is_some() {
            return; // un seul doigt
        }
        if self.milk.on && self.overlay.is_none() {
            self.milk_touch_start(id, x, y);
            return;
        }
        let Some((region, y0)) = self.layout.region_at(y) else {
            // Hors de toute fenêtre : ferme une surimpression.
            self.overlay = None;
            return;
        };
        if self.overlay.is_some() && region != Region::Overlay {
            self.close_overlay();
            return;
        }
        let (sx, sy) = self.to_skin(x, y, y0);
        let mut list = false;
        let actions = match region {
            Region::Main => {
                let view = self.player.main_view(0);
                self.main_ui.mouse_down(sx, sy, Mods::default(), &view)
            }
            Region::Eq => {
                let view = self.eq_view();
                self.eq_ui.mouse_down(sx, sy, Mods::default(), &view)
            }
            Region::Playlist => {
                if self.pl_ui.in_list(sx, sy) && !self.pl_ui.shade {
                    list = true;
                    Vec::new()
                } else {
                    self.pl_ui.mouse_down(sx, sy, Mods::default(), self.player.playlist.entries())
                }
            }
            Region::Overlay => match self.overlay.as_mut() {
                Some(Overlay::Menu(m, _)) => {
                    m.press(sx, sy);
                    Vec::new()
                }
                Some(Overlay::Gen(g)) => g.mouse_down(sx, sy),
                Some(Overlay::Library) => {
                    if self.lib_ui.in_list(sx, sy) {
                        list = true;
                        Vec::new()
                    } else {
                        let a = self.lib_ui.mouse_down(sx, sy);
                        if a.is_empty() && self.lib_ui.tab.has_query() && sy >= 41 && sy < 55 {
                            self.host.show_keyboard(true);
                        }
                        a
                    }
                }
                Some(Overlay::Skins) => {
                    if self.skins.ui.in_list(sx, sy) {
                        list = true;
                        Vec::new()
                    } else {
                        let a = self.skins.ui.mouse_down(sx, sy, false);
                        // Champ de recherche du musée : le clavier.
                        if a.is_empty() && self.skins.ui.tab == alpagamp_ui::SkinsTab::Museum && sx >= 113 && (22..37).contains(&sy) {
                            self.host.show_keyboard(true);
                        }
                        a
                    }
                }
                None => Vec::new(),
            },
        };
        self.touch = Some(Touch { id, region, start: (x, y), last: (x, y), at: Instant::now(), moved: false, list, scroll_acc: 0.0, long_done: false });
        for a in actions {
            // Déplacer une fenêtre n'a pas de sens ici.
            if a != Action::StartMove {
                self.action(region, a, el);
            }
        }
    }

    fn touch_move(&mut self, id: u64, x: f64, y: f64, el: &ActiveEventLoop) {
        if self.milk.touch.is_some() {
            return;
        }
        let scale = self.layout.scale.max(0.01) as f64;
        let Some(t) = self.touch.as_mut().filter(|t| t.id == id) else { return };
        let dy = (y - t.last.1) / scale;
        t.last = (x, y);
        if !t.moved && ((x - t.start.0).abs() / scale > SLOP || (y - t.start.1).abs() / scale > SLOP) {
            t.moved = true;
        }
        let (region, list, moved) = (t.region, t.list, t.moved);
        if list {
            if moved {
                let skins = matches!(self.overlay, Some(Overlay::Skins)) && region == Region::Overlay;
                let row_h = if skins { alpagamp_ui::skins_window::ROW_H } else { alpagamp_ui::playlist_window::ROW_H };
                t.scroll_acc -= dy / row_h as f64;
                let rows = t.scroll_acc.trunc();
                if rows != 0.0 {
                    t.scroll_acc -= rows;
                    if skins {
                        self.skins.ui.scroll_by(rows as isize);
                    } else if region == Region::Overlay {
                        self.lib_ui.scroll_by(rows as isize);
                    } else {
                        self.pl_ui.scroll_by(rows as isize);
                    }
                }
            }
            return;
        }
        let y0 = self.layout.origin(region).unwrap_or(0.0);
        let (sx, sy) = self.to_skin(x, y, y0);
        let actions = match region {
            Region::Main => {
                let view = self.player.main_view(0);
                self.main_ui.mouse_move(sx, sy, Mods::default(), &view)
            }
            Region::Eq => {
                let view = self.eq_view();
                self.eq_ui.mouse_move(sx, sy, Mods::default(), &view)
            }
            Region::Playlist => self.pl_ui.mouse_move(sx, sy, Mods::default()),
            Region::Overlay => {
                match self.overlay.as_mut() {
                    Some(Overlay::Menu(m, _)) if moved => {
                        m.cancel_press();
                        let t = self.touch.as_mut().unwrap();
                        t.scroll_acc -= dy / 16.0;
                        let rows = t.scroll_acc.trunc();
                        if rows != 0.0 {
                            t.scroll_acc -= rows;
                            m.scroll_by(rows as isize);
                        }
                    }
                    Some(Overlay::Gen(g)) => g.mouse_move(sx, sy),
                    Some(Overlay::Library) => self.lib_ui.mouse_move(sx, sy),
                    Some(Overlay::Skins) => self.skins.ui.mouse_move(sx, sy, false),
                    _ => {}
                }
                Vec::new()
            }
        };
        for a in actions {
            self.action(region, a, el);
        }
    }

    fn touch_end(&mut self, id: u64, x: f64, y: f64, cancelled: bool, el: &ActiveEventLoop) {
        if self.milk.touch.is_some() {
            self.milk_touch_end(id, x, y, cancelled);
            return;
        }
        let Some(t) = self.touch.take().filter(|t| t.id == id) else { return };
        let y0 = self.layout.origin(t.region).unwrap_or(0.0);
        let (sx, sy) = if t.long_done || cancelled { (-10_000, -10_000) } else { self.to_skin(x, y, y0) };
        let now = Instant::now();
        let double = !t.moved && self.last_tap.is_some_and(|(at, r, lx, ly)| r == t.region && now - at < DOUBLE_TAP && (lx - sx).abs() < 6 && (ly - sy).abs() < 6);
        self.last_tap = if double || t.moved { None } else { Some((now, t.region, sx, sy)) };
        if t.list && t.region == Region::Overlay && matches!(self.overlay, Some(Overlay::Skins)) {
            if !t.moved && !cancelled {
                if let Some(i) = self.skins.ui.row_at(sx, sy) {
                    self.skins.ui.sel = Some(i);
                    self.action(Region::Overlay, Action::Skins(alpagamp_ui::SkinsAction::Apply(i)), el);
                }
            }
            return;
        }
        if t.list && t.region == Region::Overlay {
            if t.moved || cancelled {
                return;
            }
            if let Some(i) = self.lib_ui.row_at(sx, sy) {
                let actions = if double { self.lib_ui.activate_row(i) } else {
                    self.lib_ui.select(i);
                    Vec::new()
                };
                for a in actions {
                    self.action(Region::Overlay, a, el);
                }
            }
            return;
        }
        if t.list {
            if t.moved || t.long_done || cancelled {
                return;
            }
            match self.pl_ui.row_at(sx, sy) {
                Some(i) if double => self.action(Region::Playlist, Action::PlaylistPlay(i), el),
                Some(i) => self.player.playlist.click(i, false, false),
                None => self.player.playlist.select_none(),
            }
            return;
        }
        let actions = match t.region {
            Region::Main => {
                let view = self.player.main_view(0);
                let mut a = self.main_ui.mouse_up(sx, sy, Mods::default(), &view);
                if double {
                    a.extend(self.main_ui.double_click(sx, sy, Mods::default()));
                }
                a
            }
            Region::Eq => {
                let view = self.eq_view();
                let mut a = self.eq_ui.mouse_up(sx, sy, Mods::default(), &view);
                if double {
                    a.extend(self.eq_ui.double_click(sx, sy));
                }
                a
            }
            Region::Playlist => {
                let mut a = self.pl_ui.mouse_up(sx, sy, Mods::default());
                if double {
                    a.extend(self.pl_ui.double_click(sx, sy));
                }
                a
            }
            Region::Overlay => {
                let max_h = self.overlay_max_h();
                match self.overlay.as_mut() {
                    Some(Overlay::Menu(m, kind)) => {
                        let kind = *kind;
                        if t.moved {
                            m.cancel_press();
                            Vec::new()
                        } else {
                            match m.release(sx, sy, max_h) {
                                MenuResult::Chosen(id) => {
                                    self.close_overlay();
                                    self.menu_command(kind, &id, el);
                                }
                                MenuResult::Closed => self.close_overlay(),
                                MenuResult::None => {}
                            }
                            Vec::new()
                        }
                    }
                    Some(Overlay::Gen(g)) => {
                        let mut a = g.mouse_up(sx, sy);
                        if double {
                            a.extend(g.double_click(sx, sy));
                        }
                        a
                    }
                    Some(Overlay::Library) => self.lib_ui.mouse_up(sx, sy),
                    Some(Overlay::Skins) => self.skins.ui.mouse_up(sx, sy, false),
                    None => Vec::new(),
                }
            }
        };
        for a in actions {
            if a != Action::StartMove {
                self.action(t.region, a, el);
            }
        }
    }

    /// Appui long : menu de la liste ou menu principal.
    fn check_long_press(&mut self) {
        if self.milk.touch.is_some() {
            self.milk_long_press();
            return;
        }
        let Some(t) = self.touch.as_mut() else { return };
        if t.moved || t.long_done || t.at.elapsed() < LONG_PRESS || t.region == Region::Overlay {
            return;
        }
        t.long_done = true;
        let list = t.list;
        if list {
            // Sélectionne la ligne touchée si elle ne l'est pas.
            let (x, y) = t.start;
            let y0 = self.layout.origin(Region::Playlist).unwrap_or(0.0);
            let (sx, sy) = self.to_skin(x, y, y0);
            if let Some(i) = self.pl_ui.row_at(sx, sy) {
                if !self.player.playlist.entries()[i].selected {
                    self.player.playlist.click(i, false, false);
                }
            }
            self.open_menu(MenuKind::Playlist);
        } else {
            self.open_menu(MenuKind::Main);
        }
    }

    fn eq_view(&self) -> EqView {
        let st = &self.player.settings;
        EqView { eq: st.eq, active: true, volume: st.volume, balance: st.balance }
    }

    // --- Surimpressions ---------------------------------------------------------------------------

    fn close_overlay(&mut self) {
        if matches!(self.overlay, Some(Overlay::Gen(_)) | Some(Overlay::Library) | Some(Overlay::Skins)) {
            self.host.show_keyboard(false);
        }
        self.overlay = None;
    }

    fn open_gen(&mut self, kind: GenKind) {
        let mut g = GenWindow::new(kind);
        if kind == GenKind::Jump {
            g.set_results((0..self.player.playlist.len()).collect());
            g.take_query_changed();
            if let Some(c) = self.player.playlist.current() {
                g.select(c);
            }
        }
        self.overlay = Some(Overlay::Gen(g));
        // Demande explicite (une demande « implicite » n'ouvre souvent pas le clavier).
        self.host.show_keyboard(true);
    }

    fn open_menu(&mut self, kind: MenuKind) {
        let items = match kind {
            MenuKind::Main => self.main_menu(),
            MenuKind::EqPresets => self.eq_menu(),
            MenuKind::Playlist => self.playlist_menu(),
            MenuKind::Info => self.info_menu(),
            MenuKind::Milk => self.milk_menu(),
        };
        let title = match kind {
            MenuKind::Main => "Alpagamp",
            MenuKind::EqPresets => t("eq.presets"),
            MenuKind::Playlist => t("menu.playlist_editor"),
            MenuKind::Info => t("info.title"),
            MenuKind::Milk => t("milk.title"),
        };
        let m = MenuWindow::new(title, items, 275, self.overlay_max_h());
        self.overlay = Some(Overlay::Menu(m, kind));
    }

    fn main_menu(&self) -> Vec<MenuItem> {
        let s = &self.player.settings;
        let v = &s.vis;
        let pb = &s.playback;
        let speeds = |prefix: &str, cur: u8| (0..5u8).map(|i| MenuItem::check(&format!("{prefix}{i}"), t(&format!("vis.speed{i}")), cur == i)).collect::<Vec<_>>();
        let mut langs = vec![MenuItem::check("lang_system", t("menu.system_language"), s.language.is_none())];
        langs.extend(Lang::ALL.iter().map(|l| MenuItem::check(&format!("lang_{}", l.code()), l.native_name(), s.language.as_deref() == Some(l.code()))));
        let mut xfade = vec![MenuItem::check("xfade_0", t("opt.off"), pb.crossfade_ms == 0)];
        xfade.extend([1u32, 2, 3, 5, 8, 10].iter().map(|n| MenuItem::check(&format!("xfade_{}", n * 1000), &format!("{n} s"), pb.crossfade_ms == n * 1000)));
        let radios = RADIOS.iter().enumerate().map(|(i, (name, _))| MenuItem::new(&format!("radio_{i}"), name)).collect();
        vec![
            MenuItem::new("library", t("menu.library")),
            MenuItem::new("milk", t("mobile.milk")),
            MenuItem::new("skins", t("menu.skin_browser")),
            MenuItem::new("music_scan", t("mobile.phone_music")),
            MenuItem::sub(t("mobile.radios"), radios),
            MenuItem::new("add_url", t("pl.add_url")),
            MenuItem::new("jump", t("menu.jump_to_file")),
            MenuItem::new("file_info", t("menu.file_info")),
            MenuItem::separator(),
            MenuItem::sub(
                t("menu.playback"),
                vec![
                    MenuItem::new("previous", t("menu.previous")),
                    MenuItem::new("play", t("menu.play")),
                    MenuItem::new("pause", t("menu.pause")),
                    MenuItem::new("stop", t("menu.stop")),
                    MenuItem::new("next", t("menu.next")),
                    MenuItem::separator(),
                    MenuItem::new("stop_fade", t("menu.stop_fadeout")),
                    MenuItem::check("stop_after", t("menu.stop_after_current"), self.player.stop_after_current),
                ],
            ),
            MenuItem::sub(
                t("menu.options"),
                vec![
                    MenuItem::check("repeat", t("menu.repeat"), s.repeat),
                    MenuItem::check("shuffle", t("menu.shuffle"), s.shuffle),
                    MenuItem::check("manual", t("menu.manual_advance"), s.manual_advance),
                    MenuItem::check("remaining", t("menu.time_remaining"), s.time_remaining),
                    MenuItem::sub(t("menu.crossfade"), xfade),
                    MenuItem::sub(
                        t("menu.replaygain"),
                        vec![
                            MenuItem::check("rg_off", t("opt.off"), pb.replay_gain == ReplayGainMode::Off),
                            MenuItem::check("rg_track", t("rg.track"), pb.replay_gain == ReplayGainMode::Track),
                            MenuItem::check("rg_album", t("rg.album"), pb.replay_gain == ReplayGainMode::Album),
                            MenuItem::check("rg_clip", t("rg.prevent_clipping"), pb.rg_prevent_clipping),
                        ],
                    ),
                    MenuItem::sub(t("menu.language"), langs),
                    MenuItem::check("milk_gpu", t("milk.gpu"), s.milkdrop.gpu),
                ],
            ),
            MenuItem::sub(
                t("menu.windows"),
                vec![MenuItem::check("win_eq", t("menu.equalizer"), self.eq_open), MenuItem::check("win_pl", t("menu.playlist_editor"), self.pl_open)],
            ),
            MenuItem::sub(
                t("menu.visualization"),
                vec![
                    MenuItem::check("vis_analyzer", t("menu.analyzer"), v.mode == VisMode::Analyzer),
                    MenuItem::check("vis_scope", t("menu.oscilloscope"), v.mode == VisMode::Oscilloscope),
                    MenuItem::check("vis_off", t("menu.vis_off"), v.mode == VisMode::Off),
                    MenuItem::separator(),
                    MenuItem::sub(
                        t("vis.analyzer_style"),
                        vec![
                            MenuItem::check("vis_normal", t("vis.normal"), v.analyzer_style == AnalyzerStyle::Normal),
                            MenuItem::check("vis_fire", t("vis.fire"), v.analyzer_style == AnalyzerStyle::Fire),
                            MenuItem::check("vis_line", t("vis.line"), v.analyzer_style == AnalyzerStyle::Line),
                            MenuItem::check("vis_thick", t("vis.thick"), v.thick_bands),
                            MenuItem::check("vis_thin", t("vis.thin"), !v.thick_bands),
                            MenuItem::check("vis_peaks", t("vis.peaks"), v.peaks),
                        ],
                    ),
                    MenuItem::sub(t("vis.falloff"), speeds("vis_fall_", v.falloff)),
                    MenuItem::sub(t("vis.peak_falloff"), speeds("vis_pfall_", v.peak_falloff)),
                    MenuItem::sub(
                        t("vis.scope_style"),
                        vec![
                            MenuItem::check("vis_dots", t("vis.dots"), v.scope_style == ScopeStyle::Dots),
                            MenuItem::check("vis_lines", t("vis.lines"), v.scope_style == ScopeStyle::Lines),
                            MenuItem::check("vis_solid", t("vis.solid"), v.scope_style == ScopeStyle::Solid),
                        ],
                    ),
                ],
            ),
            MenuItem::sub(t("menu.plugins"), self.plugins_menu()),
            if self.spotify.is_connected() { MenuItem::new("spotify_logout", t("lib.logout")) } else { MenuItem::new("spotify_login", t("lib.login")) },
            MenuItem::separator(),
            MenuItem::new("about", t("menu.about")),
            MenuItem::new("exit", t("menu.exit")),
        ]
    }

    fn eq_menu(&self) -> Vec<MenuItem> {
        let mut v: Vec<MenuItem> = factory_presets().iter().enumerate().map(|(i, p)| MenuItem::new(&format!("eqp_{i}"), &p.name)).collect();
        v.push(MenuItem::separator());
        v.push(MenuItem::check("eq_on", t("eq.enabled"), self.player.settings.eq.enabled));
        v.push(MenuItem::new("eq_reset", t("eq.reset")));
        v
    }

    fn playlist_menu(&self) -> Vec<MenuItem> {
        vec![
            MenuItem::new("pl_play", t("pl.play_item")),
            MenuItem::new("pl_info", t("menu.file_info")),
            MenuItem::separator(),
            MenuItem::new("pl_remove_sel", t("pl.remove_selected")),
            MenuItem::new("pl_crop", t("pl.crop")),
            MenuItem::new("pl_remove_all", t("pl.remove_all")),
            MenuItem::new("pl_remove_missing", t("pl.remove_missing")),
            MenuItem::new("pl_remove_dups", t("pl.remove_duplicates")),
            MenuItem::separator(),
            MenuItem::new("pl_select_all", t("pl.select_all")),
            MenuItem::new("pl_select_none", t("pl.select_none")),
            MenuItem::new("pl_invert", t("pl.invert")),
            MenuItem::sub(
                t("pl.sort"),
                vec![
                    MenuItem::new("pl_sort_title", t("pl.sort_title")),
                    MenuItem::new("pl_sort_file", t("pl.sort_filename")),
                    MenuItem::new("pl_sort_path", t("pl.sort_path")),
                    MenuItem::new("pl_reverse", t("pl.reverse")),
                    MenuItem::new("pl_randomize", t("pl.randomize")),
                ],
            ),
            MenuItem::separator(),
            MenuItem::new("music_scan_add", t("mobile.phone_music")),
            MenuItem::new("add_url", t("pl.add_url")),
        ]
    }

    /// Infos du morceau sélectionné (ou courant), une ligne par entrée.
    fn info_menu(&self) -> Vec<MenuItem> {
        let pl = &self.player.playlist;
        let i = pl.selected_indices().first().copied().or(pl.current());
        let Some(e) = i.and_then(|i| pl.entries().get(i)) else { return vec![MenuItem::new("", "—").disabled()] };
        let info = if self.player.playing_index() == i { self.player.info.clone() } else { None };
        let mut lines = vec![e.display_title()];
        if let Some(info) = &info {
            if let Some(s) = &info.stream_name {
                lines.push(format!("{} : {s}", t("info.station")));
            }
            for (k, v) in [("info.artist", &info.artist), ("info.album", &info.album), ("info.year", &info.year), ("info.genre", &info.genre)] {
                if let Some(v) = v.as_ref().filter(|v| !v.is_empty()) {
                    lines.push(format!("{} : {v}", t(k)));
                }
            }
            lines.push(format!("{} : {} {} kbit/s, {} Hz", t("info.format"), info.codec, info.bitrate_kbps.map(|b| b.to_string()).unwrap_or("?".into()), info.sample_rate));
        }
        if let Some(d) = e.duration_ms {
            lines.push(format!("{} : {}", t("info.length"), format_duration(d)));
        }
        lines.push(e.source.to_playlist_string());
        lines.into_iter().map(|l| MenuItem::new("", &l)).collect()
    }

    // --- Commandes --------------------------------------------------------------------------------

    fn scan_music(&mut self, replace: bool) {
        if !self.host.has_music_permission() {
            self.scan_pending = true;
            self.asked_permission = true;
            self.host.request_music_permission();
            return;
        }
        let dirs = self.host.music_dirs();
        log::info!("recherche de musique dans {dirs:?}");
        self.player.open(&dirs, replace);
        if replace {
            self.player.stop();
        }
        self.main_ui.marquee.flash(&format!("{} FILES", self.player.playlist.len()));
    }

    fn menu_command(&mut self, kind: MenuKind, id: &str, el: &ActiveEventLoop) {
        let _ = kind;
        if self.milk_menu_command(id) || self.plugin_menu_command(id) {
            return;
        }
        let p = &mut self.player;
        let num = |prefix: &str| id.strip_prefix(prefix).and_then(|n| n.parse::<i64>().ok());
        match id {
            "" => {}
            "library" => self.open_library(),
            "skins" => {
                self.overlay = Some(Overlay::Skins);
                if self.skins.is_empty() {
                    let cur = self.player.settings.skin.clone();
                    self.skins.load_tab(cur.as_deref());
                }
            }
            "spotify_login" => {
                self.spotify.login();
                self.open_library();
            }
            "spotify_logout" => {
                self.spotify.logout();
                self.load_library_tab();
            }
            "music_scan" => self.scan_music(true),
            "music_scan_add" => self.scan_music(false),
            "add_url" => self.open_gen(GenKind::AddUrl),
            "jump" => self.open_gen(GenKind::Jump),
            "file_info" | "pl_info" => self.open_menu(MenuKind::Info),
            "previous" => p.previous(),
            "play" => p.play_pressed(),
            "pause" => p.pause_pressed(),
            "stop" => p.stop(),
            "next" => p.next(),
            "stop_fade" => p.stop_with_fadeout(),
            "stop_after" => {
                p.stop_after_current = !p.stop_after_current;
                p.replan();
            }
            "repeat" => p.toggle_repeat(),
            "shuffle" => p.toggle_shuffle(),
            "manual" => p.toggle_manual_advance(),
            "remaining" => p.settings.time_remaining = !p.settings.time_remaining,
            "rg_off" | "rg_track" | "rg_album" => {
                p.settings.playback.replay_gain = match id {
                    "rg_track" => ReplayGainMode::Track,
                    "rg_album" => ReplayGainMode::Album,
                    _ => ReplayGainMode::Off,
                };
                p.apply_playback();
            }
            "rg_clip" => {
                p.settings.playback.rg_prevent_clipping = !p.settings.playback.rg_prevent_clipping;
                p.apply_playback();
            }
            "win_eq" => self.eq_open = !self.eq_open,
            "win_pl" => self.pl_open = !self.pl_open,
            "vis_analyzer" => p.settings.vis.mode = VisMode::Analyzer,
            "vis_scope" => p.settings.vis.mode = VisMode::Oscilloscope,
            "vis_off" => p.settings.vis.mode = VisMode::Off,
            "vis_normal" => p.settings.vis.analyzer_style = AnalyzerStyle::Normal,
            "vis_fire" => p.settings.vis.analyzer_style = AnalyzerStyle::Fire,
            "vis_line" => p.settings.vis.analyzer_style = AnalyzerStyle::Line,
            "vis_thick" => p.settings.vis.thick_bands = true,
            "vis_thin" => p.settings.vis.thick_bands = false,
            "vis_peaks" => p.settings.vis.peaks = !p.settings.vis.peaks,
            "vis_dots" => p.settings.vis.scope_style = ScopeStyle::Dots,
            "vis_lines" => p.settings.vis.scope_style = ScopeStyle::Lines,
            "vis_solid" => p.settings.vis.scope_style = ScopeStyle::Solid,
            "eq_on" => {
                let mut eq = p.settings.eq;
                eq.enabled = !eq.enabled;
                p.set_eq(eq);
            }
            "eq_reset" => {
                let mut eq = p.settings.eq;
                eq.preamp = CENTER;
                eq.bands = [CENTER; 10];
                p.set_eq(eq);
            }
            "pl_play" => {
                if let Some(i) = p.playlist.selected_indices().first().copied() {
                    p.play_index(i);
                }
            }
            "pl_remove_sel" => self.edit(|pl| pl.remove_selected()),
            "pl_crop" => self.edit(|pl| pl.crop()),
            "pl_remove_all" => self.edit(|pl| pl.clear()),
            "pl_remove_missing" => self.edit(|pl| pl.remove_missing()),
            "pl_remove_dups" => self.edit(|pl| pl.remove_duplicates()),
            "pl_select_all" => p.playlist.select_all(),
            "pl_select_none" => p.playlist.select_none(),
            "pl_invert" => p.playlist.invert_selection(),
            "pl_sort_title" => self.edit(|pl| pl.sort(SortKey::Title)),
            "pl_sort_file" => self.edit(|pl| pl.sort(SortKey::FileName)),
            "pl_sort_path" => self.edit(|pl| pl.sort(SortKey::Path)),
            "pl_reverse" => self.edit(|pl| pl.reverse()),
            "pl_randomize" => self.edit(|pl| pl.randomize()),
            "lang_system" => {
                p.settings.language = None;
                apply_language(&p.settings);
            }
            "about" => {
                let text = t("about.text").replace("{version}", env!("CARGO_PKG_VERSION"));
                let items = text.lines().filter(|l| !l.trim().is_empty()).map(|l| MenuItem::new("", l)).collect();
                self.overlay = Some(Overlay::Menu(MenuWindow::new("Alpagamp", items, 275, self.overlay_max_h()), MenuKind::Info));
            }
            "exit" => {
                self.player.shutdown_plugins();
                self.save();
                self.player.stop();
                self.host.finish();
                el.exit();
            }
            other => {
                if let Some(code) = other.strip_prefix("lang_") {
                    if let Some(l) = Lang::ALL.into_iter().find(|l| l.code() == code) {
                        p.settings.language = Some(l.code().to_string());
                        apply_language(&p.settings);
                    }
                } else if let Some(ms) = num("xfade_") {
                    p.settings.playback.crossfade_ms = ms as u32;
                    p.apply_playback();
                } else if let Some(i) = num("eqp_") {
                    if let Some(preset) = factory_presets().get(i as usize) {
                        let mut eq = p.settings.eq;
                        eq.apply_preset(preset);
                        p.set_eq(eq);
                    }
                } else if let Some(i) = num("radio_") {
                    if let Some((name, url)) = RADIOS.get(i as usize) {
                        let e = Entry::with_meta(Source::parse(url, None), Some(name.to_string()), None);
                        p.playlist.extend([e]);
                        p.playlist_changed();
                        let last = p.playlist.len() - 1;
                        p.play_index(last);
                        self.pl_ui.ensure_visible(last);
                    }
                } else if let Some(v) = num("vis_fall_") {
                    p.settings.vis.falloff = v as u8;
                } else if let Some(v) = num("vis_pfall_") {
                    p.settings.vis.peak_falloff = v as u8;
                }
            }
        }
    }

    fn apply_skin(&mut self, a: SkinApply) {
        match a {
            SkinApply::Default => {
                self.skin = self.default_skin.clone();
                self.player.settings.skin = None;
            }
            SkinApply::File(p) => match std::fs::read(&p).map_err(|e| e.to_string()).and_then(|b| Skin::load_wsz("", &b, &self.default_skin).map_err(|e| e.to_string())) {
                Ok(s) => {
                    self.skin = s;
                    self.player.settings.skin = Some(p);
                }
                Err(e) => self.main_ui.marquee.flash(&format!("{} : {e}", t("error.skin"))),
            },
            SkinApply::OpenDialog => {}
        }
        self.player.set_plugin_colors(&self.skin.vis.0);
    }

    fn edit(&mut self, f: impl FnOnce(&mut alpagamp_core::Playlist)) {
        f(&mut self.player.playlist);
        self.player.playlist_changed();
    }

    fn action(&mut self, from: Region, a: Action, el: &ActiveEventLoop) {
        if self.connect_transport(&a) {
            return;
        }
        let p = &mut self.player;
        match a {
            Action::Previous(_) => p.previous(),
            Action::Play(_) => p.play_pressed(),
            Action::Pause => p.pause_pressed(),
            Action::Stop(_) => p.stop(),
            Action::Next(_) => p.next(),
            Action::Eject(_) => self.open_menu(MenuKind::Main),
            Action::SetVolume(v) => p.set_volume(v),
            Action::SetBalance(b) => p.set_balance(b),
            Action::Seek(ms) => p.seek(ms),
            Action::ToggleShuffle => p.toggle_shuffle(),
            Action::ToggleRepeat => p.toggle_repeat(),
            Action::ToggleEqWindow => self.eq_open = !self.eq_open,
            Action::TogglePlaylistWindow => self.pl_open = !self.pl_open,
            Action::Close => match from {
                Region::Main => self.menu_command(MenuKind::Main, "exit", el),
                Region::Eq => self.eq_open = false,
                Region::Playlist => self.pl_open = false,
                Region::Overlay => self.close_overlay(),
            },
            Action::Minimize => {}
            Action::ToggleShade => match from {
                Region::Main => self.main_ui.shade = !self.main_ui.shade,
                Region::Eq => self.eq_ui.shade = !self.eq_ui.shade,
                Region::Playlist => self.pl_ui.shade = !self.pl_ui.shade,
                Region::Overlay => {}
            },
            Action::OptionsMenu | Action::VisMenu | Action::About => self.open_menu(MenuKind::Main),
            Action::ToggleVisPlugin => self.milk_enter(),
            Action::ToggleAlwaysOnTop | Action::ToggleDoubleSize | Action::StartMove => {}
            Action::FileInfo => self.open_menu(MenuKind::Info),
            Action::ToggleTimeMode => p.settings.time_remaining = !p.settings.time_remaining,
            Action::CycleVisMode => p.settings.vis.mode = p.settings.vis.mode.cycle(),
            Action::SetEqBand(i, v) => {
                let mut eq = p.settings.eq;
                eq.bands[i.min(9)] = v;
                p.set_eq(eq);
            }
            Action::SetPreamp(v) => {
                let mut eq = p.settings.eq;
                eq.preamp = v;
                p.set_eq(eq);
            }
            Action::SetEqAllBands(v) => {
                let mut eq = p.settings.eq;
                eq.bands = [v; 10];
                p.set_eq(eq);
            }
            Action::ToggleEqEnabled => {
                let mut eq = p.settings.eq;
                eq.enabled = !eq.enabled;
                p.set_eq(eq);
            }
            Action::ToggleEqAuto => {
                let mut eq = p.settings.eq;
                eq.auto = !eq.auto;
                p.set_eq(eq);
            }
            Action::EqPresetsMenu => self.open_menu(MenuKind::EqPresets),
            Action::PlaylistClick { index, shift, ctrl } => p.playlist.click(index, shift, ctrl),
            Action::PlaylistPlay(i) => p.play_index(i),
            Action::PlaylistMoveSelection(d) => {
                if p.playlist.move_selection(d) {
                    p.playlist_changed();
                }
            }
            Action::PlaylistResize(..) => {}
            Action::PlaylistMenu(item) => match item {
                PlMenuItem::AddUrl => self.open_gen(GenKind::AddUrl),
                PlMenuItem::AddDir | PlMenuItem::AddFile => self.scan_music(false),
                PlMenuItem::RemoveAll | PlMenuItem::NewList => self.edit(|pl| pl.clear()),
                PlMenuItem::Crop => self.edit(|pl| pl.crop()),
                PlMenuItem::RemoveSelected => self.edit(|pl| pl.remove_selected()),
                PlMenuItem::InvertSelection => p.playlist.invert_selection(),
                PlMenuItem::SelectNone => p.playlist.select_none(),
                PlMenuItem::SelectAll => p.playlist.select_all(),
                PlMenuItem::FileInfo => self.open_menu(MenuKind::Info),
                PlMenuItem::RemoveMisc | PlMenuItem::Sort | PlMenuItem::MiscOptions | PlMenuItem::SaveList | PlMenuItem::LoadList => {
                    self.open_menu(MenuKind::Playlist)
                }
            },
            Action::JumpTo(i) => {
                self.close_overlay();
                self.player.play_index(i);
                self.pl_ui.ensure_visible(i);
            }
            Action::SubmitUrl(url) => {
                self.close_overlay();
                match Source::parse(&url, None) {
                    Source::Spotify(uri) if !uri.starts_with("spotify:track:") && !uri.starts_with("spotify:episode:") => {
                        self.enqueue_pending = Some(self.spotify.request(LibraryRequest::Tracks(uri)));
                    }
                    src => {
                        self.player.playlist.extend([Entry::new(src)]);
                        self.player.playlist_changed();
                        let last = self.player.playlist.len() - 1;
                        self.player.play_index(last);
                    }
                }
            }
            Action::Library(la) => self.library_action(la),
            Action::SelectPreset(i) => {
                self.close_overlay();
                self.milk_select(i);
            }
            Action::MilkResize(..) | Action::MilkFullscreen => {}
            Action::Skins(sa) => {
                let cur = self.player.settings.skin.clone();
                if let Some(apply) = self.skins.action(sa, cur.as_deref()) {
                    self.apply_skin(apply);
                }
            }
        }
    }

    fn gen_key(&mut self, key: GenKey, el: &ActiveEventLoop) {
        let Some(Overlay::Gen(g)) = self.overlay.as_mut() else { return };
        let actions = g.key(key);
        if g.take_query_changed() {
            match g.kind {
                GenKind::Jump => g.set_results(self.player.playlist.search(&g.query)),
                GenKind::Presets => g.set_results(self.milk.engine.as_ref().map(|m| m.presets.search(&g.query)).unwrap_or_default()),
                GenKind::AddUrl => {}
            }
        }
        for a in actions {
            self.action(Region::Overlay, a, el);
        }
    }

    // --- Bibliothèque et Spotify -----------------------------------------------------------------

    fn open_library(&mut self) {
        self.overlay = Some(Overlay::Library);
        if self.lib_ui.items().is_empty() {
            self.load_library_tab();
        }
    }

    fn load_library_tab(&mut self) {
        let tab = self.lib_ui.tab;
        self.lib_pending = None;
        let connected = self.spotify.is_connected();
        let req = match tab {
            LibTab::Radios => Some(LibraryRequest::RadioTop),
            LibTab::Search => None,
            _ if !connected => None,
            LibTab::Playlists => Some(LibraryRequest::Playlists { liked_label: t("lib.liked_title").to_string() }),
            LibTab::Liked => Some(LibraryRequest::Tracks("spotify:collection".into())),
            LibTab::Albums => Some(LibraryRequest::Albums),
        };
        let mut items = Vec::new();
        if tab != LibTab::Radios && !connected && !matches!(self.spotify.status(), SpotifyStatus::Authorizing | SpotifyStatus::Connecting) {
            items.push(LibraryItem::new(LibraryKind::Action, t("lib.login"), "", "action:login"));
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
                self.host.show_keyboard(false);
                self.lib_ui.set_items(Vec::new());
                let r = if tab == LibTab::Radios { LibraryRequest::RadioSearch(query) } else { LibraryRequest::Search(query) };
                if tab == LibTab::Radios || self.spotify.is_connected() {
                    self.lib_pending = Some(self.spotify.request(r));
                } else {
                    self.load_library_tab();
                }
            }
            LibraryAction::Activate(item) => {
                if item.uri == "action:login" {
                    self.spotify.login();
                    self.load_library_tab();
                }
            }
            LibraryAction::Play { items, start } => {
                let entries: Vec<Entry> = items.iter().map(library_entry).collect();
                if entries.is_empty() {
                    return;
                }
                self.close_overlay();
                self.player.stop();
                self.player.playlist.clear();
                self.player.playlist.extend(entries);
                self.player.playlist_changed();
                self.player.play_index(start.min(self.player.playlist.len() - 1));
            }
            LibraryAction::Enqueue(items) => {
                self.player.playlist.extend(items.iter().map(library_entry));
                self.player.playlist_changed();
                self.main_ui.marquee.flash(&format!("+{}", items.len()));
            }
        }
    }

    fn spotify_events(&mut self) {
        for ev in self.spotify.poll_events() {
            match ev {
                SpotifyEvent::Status(_) => {
                    if self.lib_ui.tab != LibTab::Radios && matches!(self.overlay, Some(Overlay::Library)) {
                        self.load_library_tab();
                    }
                }
                SpotifyEvent::ConnectStart => self.player.play_external(Media::Custom(self.spotify.connect_opener()), "Spotify Connect"),
                SpotifyEvent::ConnectPause => {
                    if self.player.is_external() {
                        self.player.engine.pause();
                    }
                }
                SpotifyEvent::ConnectResume => {
                    if self.player.is_external() {
                        self.player.engine.resume();
                    }
                }
                SpotifyEvent::ConnectStop => {
                    if self.player.is_external() {
                        self.player.stop();
                    }
                }
                SpotifyEvent::Library { request, result } if self.enqueue_pending == Some(request) => {
                    self.enqueue_pending = None;
                    if let Ok(items) = result {
                        self.player.playlist.extend(items.iter().filter(|i| i.is_playable()).map(library_entry));
                        self.player.playlist_changed();
                    }
                }
                SpotifyEvent::Library { request, result } => {
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
    fn connect_transport(&mut self, a: &Action) -> bool {
        if !self.player.is_external() {
            return false;
        }
        match a {
            Action::Previous(_) => self.spotify.connect_command(ConnectCommand::Previous),
            Action::Next(_) => self.spotify.connect_command(ConnectCommand::Next),
            Action::Pause => {
                if self.player.state() == PlayState::Paused {
                    self.spotify.connect_command(ConnectCommand::Play);
                    self.player.engine.resume();
                } else {
                    self.spotify.connect_command(ConnectCommand::Pause);
                    self.player.engine.pause();
                }
            }
            Action::Play(_) => {
                self.spotify.connect_command(ConnectCommand::Play);
                self.player.engine.resume();
            }
            _ => return false,
        }
        true
    }

    /// Taille de fenêtre voulue (simulateur ; ignorée sur téléphone).
    pub fn set_window_size(&mut self, w: u32, h: u32) {
        self.sim_size = Some((w, h));
    }

    fn create_surface(&mut self, el: &ActiveEventLoop) {
        if self.window.is_none() {
            let mut attrs = WindowAttributes::default().with_title("Alpagamp");
            if let Some((w, h)) = self.sim_size {
                attrs = attrs.with_inner_size(winit::dpi::PhysicalSize::new(w, h)).with_resizable(false);
            }
            match el.create_window(attrs) {
                Ok(w) => self.window = Some(Rc::new(w)),
                Err(e) => {
                    log::error!("fenêtre : {e}");
                    return;
                }
            }
        }
        let Some(w) = self.window.clone() else { return };
        self.screen = self.make_screen(w.clone());
        w.request_redraw();
    }
}

fn library_status(sp: &Spotify, lib: &LibraryWindow, loading: bool) -> String {
    if loading {
        return t("lib.loading").to_string();
    }
    if lib.tab == LibTab::Radios {
        return String::new();
    }
    match sp.status() {
        SpotifyStatus::LoggedOut => String::new(),
        SpotifyStatus::Authorizing => t("lib.authorizing").to_string(),
        SpotifyStatus::Connecting => t("lib.connecting").to_string(),
        SpotifyStatus::Connected { user } => format!("Spotify : {user}"),
        SpotifyStatus::Failed(e) => format!("Spotify : {e}"),
    }
}

/// Entrée de playlist pour un élément de la Bibliothèque.
fn library_entry(it: &LibraryItem) -> Entry {
    let title = if it.kind == LibraryKind::Radio { it.title.clone() } else { it.playlist_title() };
    let mut e = Entry::with_meta(Source::parse(&it.uri, None), Some(title), it.duration_ms);
    e.meta_loaded = true;
    e
}

fn time_text(player: &Player) -> String {
    let st = player.engine.status();
    if st.state == PlayState::Stopped {
        return String::new();
    }
    let dur = st.info.as_ref().and_then(|i| i.duration_ms);
    match (player.settings.time_remaining, dur) {
        (true, Some(d)) => format!("-{}", format_duration(d.saturating_sub(st.position_ms))),
        _ => format_duration(st.position_ms),
    }
}

impl ApplicationHandler for Mobile {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        self.create_surface(el);
        // Retour de la boîte de permission : on lance la recherche demandée.
        if self.scan_pending && self.host.has_music_permission() {
            self.scan_pending = false;
            self.scan_music(true);
        } else if self.player.playlist.is_empty() && !self.asked_permission {
            self.scan_music(true);
        }
    }

    fn suspended(&mut self, _el: &ActiveEventLoop) {
        // La surface disparaît ; la musique continue (jusqu'à ce que le système arrête l'appli).
        self.screen = None;
        self.milk.ui_layer = None;
        self.save();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            // Simulateur sur ordinateur : la souris fait office de doigt.
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                if self.mouse_down {
                    self.touch_move(0, position.x, position.y, el);
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput { state, button: winit::event::MouseButton::Left, .. } => {
                let (x, y) = self.cursor;
                match state {
                    ElementState::Pressed => {
                        self.mouse_down = true;
                        self.touch_start(0, x, y, el);
                    }
                    ElementState::Released => {
                        self.mouse_down = false;
                        self.touch_end(0, x, y, false, el);
                    }
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::Touch(t) => {
                match t.phase {
                    TouchPhase::Started => self.touch_start(t.id, t.location.x, t.location.y, el),
                    TouchPhase::Moved => self.touch_move(t.id, t.location.x, t.location.y, el),
                    TouchPhase::Ended => self.touch_end(t.id, t.location.x, t.location.y, false, el),
                    TouchPhase::Cancelled => self.touch_end(t.id, t.location.x, t.location.y, true, el),
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let gen_open = matches!(self.overlay, Some(Overlay::Gen(_)));
                let key = match &event.logical_key {
                    Key::Named(NamedKey::GoBack) | Key::Named(NamedKey::BrowserBack) | Key::Named(NamedKey::Escape) => {
                        if self.overlay.is_some() {
                            self.close_overlay();
                        } else if self.milk.on {
                            self.milk_leave();
                        }
                        None
                    }
                    Key::Named(NamedKey::Backspace) => Some(GenKey::Backspace),
                    Key::Named(NamedKey::Delete) => Some(GenKey::Delete),
                    Key::Named(NamedKey::Enter) => Some(GenKey::Enter),
                    Key::Named(NamedKey::ArrowLeft) => Some(GenKey::Left),
                    Key::Named(NamedKey::ArrowRight) => Some(GenKey::Right),
                    Key::Named(NamedKey::ArrowUp) => Some(GenKey::Up),
                    Key::Named(NamedKey::ArrowDown) => Some(GenKey::Down),
                    Key::Named(NamedKey::Space) => Some(GenKey::Text(" ".into())),
                    Key::Character(c) => Some(GenKey::Text(event.text.as_ref().map(|t| t.to_string()).unwrap_or_else(|| c.to_string()))),
                    _ => event.text.as_ref().map(|t| GenKey::Text(t.to_string())),
                };
                if matches!(self.overlay, Some(Overlay::Skins)) {
                    if let Some(k) = key {
                        for a in self.skins.ui.key(k) {
                            self.action(Region::Overlay, a, el);
                        }
                    }
                } else if matches!(self.overlay, Some(Overlay::Library)) {
                    if let Some(k) = key {
                        for a in self.lib_ui.key(k) {
                            self.action(Region::Overlay, a, el);
                        }
                    }
                } else if let (true, Some(k)) = (gen_open, key) {
                    self.gen_key(k, el);
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::CloseRequested => {
                self.save();
                el.exit();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        self.spotify_events();
        self.milk_tick();
        let skins_visible = matches!(self.overlay, Some(Overlay::Skins));
        if let Some(a) = self.skins.tick(skins_visible, &self.default_skin) {
            self.apply_skin(a);
        }
        self.player.tick();
        // Visualisation de plugin arrêtée (erreur) sans MilkDrop pour prendre le relais : on sort.
        if self.milk.on && !self.player.plugins.vis_running() && self.milk.engine.is_none() {
            self.milk_leave();
        }
        if self.player.take_track_changed().is_some() {
            if let Some(i) = self.player.playlist.current() {
                self.pl_ui.ensure_visible(i);
            }
        }
        self.check_long_press();
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
        let now = Instant::now();
        let playing = self.player.state() == PlayState::Playing;
        let interval = Duration::from_millis(if playing || self.milk.on || self.touch.is_some() { 16 } else { 100 });
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
            if let Some(w) = &self.window {
                w.request_redraw();
            }
            self.next_frame = now + interval;
        } else if self.next_frame > now + interval {
            self.next_frame = now + interval;
        }
        el.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }

    fn exiting(&mut self, _el: &ActiveEventLoop) {
        self.save();
    }
}
