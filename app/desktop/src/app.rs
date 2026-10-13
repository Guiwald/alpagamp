//! L'application de bureau : fenêtres, événements, commandes.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use alpagamp_audio::PlayState;
use alpagamp_core::i18n::{self, Lang, t};
use alpagamp_core::settings::{VisMode, WindowState};
use alpagamp_core::time::format_duration;
use alpagamp_core::Settings;
use alpagamp_skin::{Image, Skin};
use alpagamp_ui::{Action, MainWindow, Mods};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::ModifiersState;
use winit::window::{WindowId, WindowLevel};

use crate::command::Command;
use crate::dialogs;
use crate::keys;
use crate::paths::Paths;
use crate::player::Player;
use crate::window::SkinWindow;

const SNAP: i32 = 10;
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

pub struct App {
    paths: Paths,
    player: Player,
    default_skin: Skin,
    skin: Skin,
    main: Option<SkinWindow>,
    main_ui: MainWindow,
    fb: Image,
    mods: ModifiersState,
    /// Position de la souris dans la fenêtre principale (pixels physiques).
    cursor: (f64, f64),
    /// Déplacement de fenêtre en cours : point saisi (pixels physiques, relatif à la fenêtre).
    moving: Option<(f64, f64)>,
    last_click: Option<(Instant, i32, i32)>,
    start: Instant,
    last_frame: Instant,
    vis_l: Vec<f32>,
    vis_r: Vec<f32>,
    dropped: Vec<PathBuf>,
    pending_args: Vec<PathBuf>,
    focused: bool,
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
        let mut player = Player::new(settings);
        if args.is_empty() {
            player.load_playlist_file(&paths.playlist);
        }
        let mut main_ui = MainWindow::new();
        main_ui.shade = player.settings.main_window.map(|w| w.shade).unwrap_or(false);
        App {
            paths,
            player,
            default_skin,
            skin,
            main: None,
            main_ui,
            fb: Image::new(275, 116, 0),
            mods: ModifiersState::empty(),
            cursor: (0.0, 0.0),
            moving: None,
            last_click: None,
            start: Instant::now(),
            last_frame: Instant::now(),
            vis_l: vec![0.0; alpagamp_ui::vis::WINDOW],
            vis_r: vec![0.0; alpagamp_ui::vis::WINDOW],
            dropped: Vec::new(),
            pending_args: args,
            focused: true,
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            menus: None,
        }
    }

    fn mods(&self) -> Mods {
        let ctrl = if cfg!(target_os = "macos") { self.mods.super_key() } else { self.mods.control_key() };
        Mods { shift: self.mods.shift_key(), ctrl, alt: self.mods.alt_key() }
    }

    fn refresh_menus(&mut self) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let Some(m) = self.menus.as_mut() {
            m.refresh(&self.player.settings, &self.paths.skins_dir);
        }
    }

    fn popup_menu(&self) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let (Some(m), Some(w)) = (self.menus.as_ref(), self.main.as_ref()) {
            m.popup(&w.window);
        }
    }

    fn save(&mut self) {
        if let Some(w) = &self.main {
            if let Ok(p) = w.window.outer_position() {
                self.player.settings.main_window = Some(WindowState { x: p.x, y: p.y, open: true, shade: self.main_ui.shade, w: 275, h: 116 });
            }
        }
        self.player.settings.resume_index = self.player.playlist.current();
        if let Err(e) = self.player.settings.save(&self.paths.settings) {
            log::warn!("réglages non enregistrés : {e}");
        }
        self.player.save_playlist_file(&self.paths.playlist);
    }

    // --- Skins --------------------------------------------------------------------------------------

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
                self.player.settings.skin = Some(stored);
                self.refresh_menus();
            }
            Err(e) => dialogs::message(t("error.skin"), &format!("{}\n\n{e}", path.display())),
        }
        self.request_redraw();
    }

    fn request_redraw(&self) {
        if let Some(w) = &self.main {
            w.window.request_redraw();
        }
    }

    // --- Commandes ----------------------------------------------------------------------------------

    fn action(&mut self, a: Action, event_loop: &ActiveEventLoop) {
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
            Action::ToggleEqWindow | Action::TogglePlaylistWindow => {
                self.main_ui.marquee.flash("EQ / PLAYLIST : V0.2");
                return;
            }
            Action::Close => C::Exit,
            Action::Minimize => C::Minimize,
            Action::ToggleShade => C::ToggleShade,
            Action::OptionsMenu | Action::VisMenu => {
                self.popup_menu();
                return;
            }
            Action::ToggleAlwaysOnTop => C::ToggleAlwaysOnTop,
            Action::FileInfo => C::FileInfo,
            Action::ToggleDoubleSize => C::ToggleDoubleSize,
            Action::ToggleTimeMode => C::ToggleTimeMode,
            Action::CycleVisMode => C::Vis(self.player.settings.vis.mode.cycle()),
            Action::ToggleVisPlugin => return,
            Action::StartMove => {
                self.moving = Some(self.cursor);
                return;
            }
            Action::About => C::About,
        };
        self.command(cmd, event_loop);
    }

    fn command(&mut self, cmd: Command, event_loop: &ActiveEventLoop) {
        use Command::*;
        match cmd {
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
            OpenFiles | AddFiles => {
                let files = dialogs::pick_files(self.player.settings.last_dir.as_deref());
                if let Some(d) = files.first().and_then(|f| f.parent()) {
                    self.player.settings.last_dir = Some(d.to_path_buf());
                }
                self.player.open(&files, cmd == OpenFiles);
            }
            OpenFolder => {
                if let Some(dir) = dialogs::pick_folder(self.player.settings.last_dir.as_deref()) {
                    self.player.settings.last_dir = Some(dir.clone());
                    self.player.open(&[dir], true);
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
                self.player.settings.double_size = !self.player.settings.double_size;
                let d = self.player.settings.double_size;
                if let Some(w) = self.main.as_mut() {
                    w.update_zoom(d);
                }
            }
            ToggleAlwaysOnTop => {
                self.player.settings.always_on_top = !self.player.settings.always_on_top;
                if let Some(w) = &self.main {
                    w.window.set_window_level(if self.player.settings.always_on_top { WindowLevel::AlwaysOnTop } else { WindowLevel::Normal });
                }
            }
            ToggleShade => {
                self.main_ui.shade = !self.main_ui.shade;
                let size = self.main_ui.size();
                if let Some(w) = self.main.as_mut() {
                    w.set_skin_size(size);
                }
            }
            Vis(mode) => {
                self.player.settings.vis.mode = mode;
                if mode == VisMode::Off {
                    self.main_ui.vis.reset();
                }
            }
            FileInfo => self.show_file_info(),
            About => {
                let text = t("about.text").replace("{version}", env!("CARGO_PKG_VERSION"));
                dialogs::message("Alpagamp", &text);
            }
            Exit => {
                self.save();
                event_loop.exit();
            }
            Minimize => {
                if let Some(w) = &self.main {
                    w.window.set_minimized(true);
                }
            }
            Language(l) => {
                self.player.settings.language = l.map(|l| l.code().to_string());
                apply_language(&self.player.settings);
            }
        }
        self.refresh_menus();
        self.request_redraw();
    }

    fn show_file_info(&self) {
        let Some(e) = self.player.playlist.current_entry() else { return };
        let info = self.player.info.as_ref();
        let mut lines = Vec::new();
        let mut add = |k: &str, v: Option<String>| {
            if let Some(v) = v.filter(|v| !v.is_empty()) {
                lines.push(format!("{} : {v}", t(k)));
            }
        };
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
        add("info.location", Some(e.source.to_playlist_string()));
        dialogs::message(t("info.title"), &lines.join("\n"));
    }

    // --- Rendu --------------------------------------------------------------------------------------

    fn redraw(&mut self) {
        let clock = self.start.elapsed().as_millis() as u64;
        let mut view = self.player.main_view(clock);
        view.active = self.focused;
        self.main_ui.render(&mut self.fb, &self.skin, &view);
        if let Some(w) = self.main.as_mut() {
            w.present(&self.fb);
        }
    }

    /// Déplace la fenêtre principale en suivant la souris, aimantée aux bords de l'écran.
    fn move_window(&mut self, x: f64, y: f64) {
        let (Some((gx, gy)), Some(w)) = (self.moving, self.main.as_ref()) else { return };
        let Ok(pos) = w.window.outer_position() else { return };
        let mut nx = pos.x + (x - gx).round() as i32;
        let mut ny = pos.y + (y - gy).round() as i32;
        if !self.mods.shift_key() {
            if let Some(m) = w.window.current_monitor() {
                let (mx, my) = (m.position().x, m.position().y);
                let (mw, mh) = (m.size().width as i32, m.size().height as i32);
                let size = w.window.outer_size();
                let (ww, wh) = (size.width as i32, size.height as i32);
                let snap = SNAP * w.zoom as i32;
                if (nx - mx).abs() < snap {
                    nx = mx;
                }
                if (ny - my).abs() < snap {
                    ny = my;
                }
                if (nx + ww - (mx + mw)).abs() < snap {
                    nx = mx + mw - ww;
                }
                if (ny + wh - (my + mh)).abs() < snap {
                    ny = my + mh - wh;
                }
            }
        }
        if nx != pos.x || ny != pos.y {
            w.window.set_outer_position(PhysicalPosition::new(nx, ny));
        }
    }

    fn handle_dropped(&mut self) {
        if self.dropped.is_empty() {
            return;
        }
        let files = std::mem::take(&mut self.dropped);
        let (skins, media): (Vec<PathBuf>, Vec<PathBuf>) = files.into_iter().partition(|p| {
            matches!(p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("wsz" | "wal" | "zip"))
        });
        if let Some(s) = skins.last() {
            self.load_skin(s);
        }
        if !media.is_empty() {
            self.player.open(&media, true);
        }
    }
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
        if self.main.is_some() {
            return;
        }
        let s = &self.player.settings;
        let pos = s.main_window.map(|w| (w.x, w.y));
        match SkinWindow::new(event_loop, "Alpagamp", self.main_ui.size(), pos, s.always_on_top, s.double_size) {
            Ok(w) => self.main = Some(w),
            Err(e) => {
                log::error!("fenêtre impossible : {e}");
                event_loop.exit();
                return;
            }
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            self.menus = Some(crate::menu::Menus::new(&self.player.settings, &self.paths.skins_dir));
        }
        let args = std::mem::take(&mut self.pending_args);
        if !args.is_empty() {
            self.dropped = args;
            self.handle_dropped();
        }
        self.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.command(Command::Exit, event_loop),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::Focused(f) => {
                self.focused = f;
                self.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let d = self.player.settings.double_size;
                if let Some(w) = self.main.as_mut() {
                    w.update_zoom(d);
                }
            }
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                if self.moving.is_some() {
                    self.move_window(position.x, position.y);
                    return;
                }
                if let Some(w) = &self.main {
                    let (x, y) = w.to_skin(position.x, position.y);
                    let view = self.player.main_view(0);
                    let actions = self.main_ui.mouse_move(x, y, self.mods(), &view);
                    for a in actions {
                        self.action(a, event_loop);
                    }
                    self.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                let Some(w) = &self.main else { return };
                let (x, y) = w.to_skin(self.cursor.0, self.cursor.1);
                let view = self.player.main_view(0);
                let mods = self.mods();
                let actions = match state {
                    ElementState::Pressed => {
                        let now = Instant::now();
                        let double = self.last_click.is_some_and(|(t, lx, ly)| now - t < DOUBLE_CLICK && (lx - x).abs() < 3 && (ly - y).abs() < 3);
                        self.last_click = Some((now, x, y));
                        if double {
                            self.last_click = None;
                            let mut a = self.main_ui.double_click(x, y, mods);
                            if a.is_empty() {
                                a = self.main_ui.mouse_down(x, y, mods, &view);
                            }
                            a
                        } else {
                            self.main_ui.mouse_down(x, y, mods, &view)
                        }
                    }
                    ElementState::Released => {
                        if self.moving.take().is_some() {
                            self.save();
                        }
                        self.main_ui.mouse_up(x, y, mods, &view)
                    }
                };
                for a in actions {
                    self.action(a, event_loop);
                }
                self.request_redraw();
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } => self.popup_menu(),
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => (p.y / 20.0) as f32,
                };
                if lines.abs() > 0.0 {
                    let view = self.player.main_view(0);
                    for a in self.main_ui.wheel(lines, &view) {
                        self.action(a, event_loop);
                    }
                    self.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if let Some(c) = keys::map(&event, self.mods) {
                        self.command(c, event_loop);
                    }
                }
            }
            WindowEvent::DroppedFile(p) => self.dropped.push(p),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_dropped();
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let cmds = self.menus.as_ref().map(|m| m.poll()).unwrap_or_default();
            for c in cmds {
                self.command(c, event_loop);
            }
        }
        self.player.tick();
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;
        let playing = self.player.state() == PlayState::Playing;
        if playing {
            self.player.engine.vis_latest(&mut self.vis_l, &mut self.vis_r);
            self.main_ui.vis.analyze(&self.vis_l, &self.vis_r);
        } else {
            self.main_ui.vis.analyze(&[], &[]);
        }
        self.main_ui.vis.advance(dt, &self.player.settings.vis);
        self.main_ui.marquee.tick(now);
        self.request_redraw();
        let next = if playing || self.main_ui.is_dragging() { now + Duration::from_millis(16) } else { now + Duration::from_millis(100) };
        event_loop.set_control_flow(ControlFlow::WaitUntil(next));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.save();
    }
}
