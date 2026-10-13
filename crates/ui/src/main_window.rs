//! La fenêtre principale : rendu (275x116, ou 275x14 en bandeau) et interactions.

use alpagamp_core::settings::VisSettings;
use alpagamp_core::time::display_minutes_seconds;
use alpagamp_skin::sprites;
use alpagamp_skin::{Image, Skin};

use crate::marquee::Marquee;
use crate::paint::Painter;
use crate::vis::ClassicVis;
use crate::{Action, Mods, Transport};

pub const WIDTH: u32 = 275;
pub const HEIGHT: u32 = 116;
pub const SHADE_HEIGHT: u32 = 14;

/// Ce que la fenêtre principale affiche (fourni par l'application à chaque image).
#[derive(Clone, Debug, Default)]
pub struct MainView {
    pub transport: Transport,
    pub buffering: bool,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub seekable: bool,
    /// Texte du titre défilant (« 3. Artiste - Titre (4:12) »).
    pub title: String,
    pub kbps: Option<u32>,
    pub khz: Option<u32>,
    pub stereo: Option<bool>,
    pub volume: u8,
    pub balance: i8,
    pub shuffle: bool,
    pub repeat: bool,
    pub eq_open: bool,
    pub pl_open: bool,
    pub time_remaining: bool,
    pub always_on_top: bool,
    pub double_size: bool,
    pub active: bool,
    pub vis: VisSettings,
    /// Horloge en millisecondes (clignotement de la pause).
    pub clock_ms: u64,
}

/// Zones cliquables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Zone {
    Options,
    Minimize,
    Shade,
    Close,
    Clutter(u8),
    Time,
    Vis,
    Marquee,
    Volume,
    Balance,
    EqToggle,
    PlToggle,
    Position,
    Previous,
    Play,
    Pause,
    Stop,
    Next,
    Eject,
    Shuffle,
    Repeat,
    About,
    TitleBar,
    /// Fond : glisser déplace la fenêtre.
    Background,
}

fn inside(x: i32, y: i32, rx: i32, ry: i32, rw: i32, rh: i32) -> bool {
    x >= rx && y >= ry && x < rx + rw && y < ry + rh
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    Volume,
    Balance,
    /// Fraction 0..1 en cours de choix.
    Position(f64),
    Marquee { start_x: i32, start_offset: i32 },
}

pub struct MainWindow {
    pub shade: bool,
    pressed: Option<Zone>,
    pressed_inside: bool,
    drag: Option<Drag>,
    pub marquee: Marquee,
    pub vis: ClassicVis,
}

impl Default for MainWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl MainWindow {
    pub fn new() -> Self {
        Self { shade: false, pressed: None, pressed_inside: false, drag: None, marquee: Marquee::new(), vis: ClassicVis::new() }
    }

    pub fn size(&self) -> (u32, u32) {
        (WIDTH, if self.shade { SHADE_HEIGHT } else { HEIGHT })
    }

    /// Un glissement (volume, position…) est-il en cours ?
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some() || self.pressed.is_some()
    }

    /// Curseur du skin pour ce point (nom du fichier `.cur`), comme Winamp 2.
    pub fn cursor_at(&self, x: i32, y: i32) -> &'static str {
        let z = self.zone_at(x, y);
        if self.shade {
            return match z {
                Zone::Close => "wsclose.cur",
                Zone::Minimize => "wsmin.cur",
                Zone::Shade => "wswinbut.cur",
                Zone::Position => "wsposbar.cur",
                Zone::Options => "mainmenu.cur",
                _ => "wsnormal.cur",
            };
        }
        match z {
            Zone::Close => "close.cur",
            Zone::Minimize => "min.cur",
            Zone::Shade => "winbut.cur",
            Zone::Options => "mainmenu.cur",
            Zone::TitleBar => "titlebar.cur",
            Zone::Position => "posbar.cur",
            Zone::Volume | Zone::Balance => "volbal.cur",
            Zone::Marquee => "songname.cur",
            _ => "normal.cur",
        }
    }

    fn zone_at(&self, x: i32, y: i32) -> Zone {
        if self.shade {
            return if inside(x, y, 6, 3, 9, 9) {
                Zone::Options
            } else if inside(x, y, 244, 3, 9, 9) {
                Zone::Minimize
            } else if inside(x, y, 254, 3, 9, 9) {
                Zone::Shade
            } else if inside(x, y, 264, 3, 9, 9) {
                Zone::Close
            } else if inside(x, y, 78, 4, 39, 8) {
                Zone::Vis
            } else if inside(x, y, 127, 3, 31, 7) {
                Zone::Time
            } else if y >= 2 && y < 12 && (169..176).contains(&x) {
                Zone::Previous
            } else if y >= 2 && y < 12 && (176..186).contains(&x) {
                Zone::Play
            } else if y >= 2 && y < 12 && (186..195).contains(&x) {
                Zone::Pause
            } else if y >= 2 && y < 12 && (195..204).contains(&x) {
                Zone::Stop
            } else if y >= 2 && y < 12 && (204..215).contains(&x) {
                Zone::Next
            } else if y >= 2 && y < 12 && (215..225).contains(&x) {
                Zone::Eject
            } else if inside(x, y, 226, 4, 17, 7) {
                Zone::Position
            } else {
                Zone::TitleBar
            };
        }
        if y < 14 {
            return if inside(x, y, 6, 3, 9, 9) {
                Zone::Options
            } else if inside(x, y, 244, 3, 9, 9) {
                Zone::Minimize
            } else if inside(x, y, 254, 3, 9, 9) {
                Zone::Shade
            } else if inside(x, y, 264, 3, 9, 9) {
                Zone::Close
            } else {
                Zone::TitleBar
            };
        }
        if (10..19).contains(&x) && (22..65).contains(&y) {
            let i = match y {
                22..=31 => 0,
                32..=39 => 1,
                40..=47 => 2,
                48..=55 => 3,
                _ => 4,
            };
            return Zone::Clutter(i);
        }
        let zones: [(Zone, i32, i32, i32, i32); 19] = [
            (Zone::Time, 36, 26, 63, 13),
            (Zone::Vis, 24, 43, 76, 16),
            (Zone::Marquee, 111, 24, 155, 12),
            (Zone::Volume, 107, 57, 68, 13),
            (Zone::Balance, 177, 57, 38, 13),
            (Zone::EqToggle, 219, 58, 23, 12),
            (Zone::PlToggle, 242, 58, 23, 12),
            (Zone::Position, 16, 72, 248, 10),
            (Zone::Previous, 16, 88, 23, 18),
            (Zone::Play, 39, 88, 23, 18),
            (Zone::Pause, 62, 88, 23, 18),
            (Zone::Stop, 85, 88, 23, 18),
            (Zone::Next, 108, 88, 22, 18),
            (Zone::Eject, 136, 89, 22, 16),
            (Zone::Shuffle, 164, 89, 46, 15),
            (Zone::Repeat, 210, 89, 28, 15),
            (Zone::About, 253, 91, 13, 15),
            (Zone::Background, 0, 0, 0, 0),
            (Zone::Background, 0, 0, 0, 0),
        ];
        for (z, zx, zy, zw, zh) in zones {
            if inside(x, y, zx, zy, zw, zh) {
                return z;
            }
        }
        Zone::Background
    }

    fn is_button(z: Zone) -> bool {
        matches!(
            z,
            Zone::Options
                | Zone::Minimize
                | Zone::Shade
                | Zone::Close
                | Zone::Clutter(_)
                | Zone::EqToggle
                | Zone::PlToggle
                | Zone::Previous
                | Zone::Play
                | Zone::Pause
                | Zone::Stop
                | Zone::Next
                | Zone::Eject
                | Zone::Shuffle
                | Zone::Repeat
                | Zone::About
        )
    }

    // --- Souris ---------------------------------------------------------------------------------

    pub fn mouse_down(&mut self, x: i32, y: i32, mods: Mods, view: &MainView) -> Vec<Action> {
        let z = self.zone_at(x, y);
        if Self::is_button(z) {
            self.pressed = Some(z);
            self.pressed_inside = true;
            return Vec::new();
        }
        match z {
            Zone::TitleBar | Zone::Background => vec![Action::StartMove],
            Zone::Volume => {
                self.drag = Some(Drag::Volume);
                self.drag_to(x, y, mods, view)
            }
            Zone::Balance => {
                self.drag = Some(Drag::Balance);
                self.drag_to(x, y, mods, view)
            }
            Zone::Position => {
                if view.seekable && view.duration_ms.is_some() && view.transport != Transport::Stopped {
                    self.drag = Some(Drag::Position(0.0));
                    self.drag_to(x, y, mods, view)
                } else if self.shade {
                    vec![Action::StartMove]
                } else {
                    Vec::new()
                }
            }
            Zone::Marquee => {
                self.drag = Some(Drag::Marquee { start_x: x, start_offset: self.marquee.offset() });
                Vec::new()
            }
            Zone::Time => vec![Action::ToggleTimeMode],
            Zone::Vis => vec![Action::CycleVisMode],
            _ => Vec::new(),
        }
    }

    pub fn mouse_move(&mut self, x: i32, y: i32, mods: Mods, view: &MainView) -> Vec<Action> {
        if let Some(p) = self.pressed {
            self.pressed_inside = self.zone_at(x, y) == p;
            return Vec::new();
        }
        if self.drag.is_some() {
            return self.drag_to(x, y, mods, view);
        }
        Vec::new()
    }

    pub fn mouse_up(&mut self, x: i32, y: i32, mods: Mods, view: &MainView) -> Vec<Action> {
        let mut out = Vec::new();
        if let Some(p) = self.pressed.take() {
            if self.zone_at(x, y) == p {
                out.extend(self.click(p, mods, view));
            }
        }
        if let Some(d) = self.drag.take() {
            match d {
                Drag::Position(f) => {
                    if let Some(dur) = view.duration_ms {
                        out.push(Action::Seek((f * dur as f64) as u64));
                    }
                }
                Drag::Marquee { .. } => self.marquee.release(),
                _ => {}
            }
            self.marquee.clear_status();
        }
        out
    }

    pub fn double_click(&mut self, x: i32, y: i32, _mods: Mods) -> Vec<Action> {
        match self.zone_at(x, y) {
            Zone::TitleBar => vec![Action::ToggleShade],
            Zone::Marquee => vec![Action::FileInfo],
            Zone::Vis => vec![Action::CycleVisMode, Action::ToggleVisPlugin],
            Zone::Options => vec![Action::Close],
            _ => Vec::new(),
        }
    }

    pub fn wheel(&mut self, lines: f32, view: &MainView) -> Vec<Action> {
        let v = (view.volume as i32 + (lines * 4.0).round() as i32).clamp(0, 255) as u8;
        self.marquee.set_status(format!("VOLUME: {}%", (v as u32 * 100 + 127) / 255));
        vec![Action::SetVolume(v)]
    }

    fn click(&mut self, z: Zone, mods: Mods, view: &MainView) -> Vec<Action> {
        match z {
            Zone::Options => vec![Action::OptionsMenu],
            Zone::Minimize => vec![Action::Minimize],
            Zone::Shade => vec![Action::ToggleShade],
            Zone::Close => vec![Action::Close],
            Zone::Clutter(0) => vec![Action::OptionsMenu],
            Zone::Clutter(1) => vec![Action::ToggleAlwaysOnTop],
            Zone::Clutter(2) => vec![Action::FileInfo],
            Zone::Clutter(3) => vec![Action::ToggleDoubleSize],
            Zone::Clutter(_) => vec![Action::VisMenu],
            Zone::EqToggle => vec![Action::ToggleEqWindow],
            Zone::PlToggle => vec![Action::TogglePlaylistWindow],
            Zone::Previous => vec![Action::Previous(mods)],
            Zone::Play => vec![Action::Play(mods)],
            Zone::Pause => vec![Action::Pause],
            Zone::Stop => vec![Action::Stop(mods)],
            Zone::Next => vec![Action::Next(mods)],
            Zone::Eject => vec![Action::Eject(mods)],
            Zone::Shuffle => {
                self.marquee.flash(if view.shuffle { "SHUFFLE: OFF" } else { "SHUFFLE: ON" });
                vec![Action::ToggleShuffle]
            }
            Zone::Repeat => {
                self.marquee.flash(if view.repeat { "REPEAT: OFF" } else { "REPEAT: ON" });
                vec![Action::ToggleRepeat]
            }
            Zone::About => vec![Action::About],
            _ => Vec::new(),
        }
    }

    fn drag_to(&mut self, x: i32, _y: i32, mods: Mods, view: &MainView) -> Vec<Action> {
        match self.drag {
            Some(Drag::Volume) => {
                let v = ((x - 107 - 7) as f32 * 255.0 / 51.0).round().clamp(0.0, 255.0) as u8;
                self.marquee.set_status(format!("VOLUME: {}%", (v as u32 * 100 + 127) / 255));
                vec![Action::SetVolume(v)]
            }
            Some(Drag::Balance) => {
                let rel = (x - 177 - 7 - 12) as f32;
                let mut b = (rel * 127.0 / 12.0).round().clamp(-127.0, 127.0) as i32;
                // Aimantation au centre (sauf avec Maj).
                if !mods.shift && b.abs() <= 9 * 127 / 12 / 4 {
                    b = 0;
                }
                let pct = (b.abs() * 100 + 63) / 127;
                let text = match b {
                    0 => "BALANCE: CENTER".to_string(),
                    b if b < 0 => format!("BALANCE: {pct}% LEFT"),
                    _ => format!("BALANCE: {pct}% RIGHT"),
                };
                self.marquee.set_status(text);
                vec![Action::SetBalance(b as i8)]
            }
            Some(Drag::Position(_)) => {
                let f = if self.shade { ((x - 226 - 1) as f64 / 14.0).clamp(0.0, 1.0) } else { ((x - 16 - 14) as f64 / 219.0).clamp(0.0, 1.0) };
                self.drag = Some(Drag::Position(f));
                if let Some(dur) = view.duration_ms {
                    let target = (f * dur as f64) as u64;
                    let fmt = |ms: u64| {
                        let (m, s) = display_minutes_seconds(ms);
                        format!("{m:02}:{s:02}")
                    };
                    self.marquee.set_status(format!("SEEK TO: {}/{} ({}%)", fmt(target), fmt(dur), (f * 100.0).round() as u32));
                    if mods.shift {
                        return vec![Action::Seek(target)];
                    }
                }
                Vec::new()
            }
            Some(Drag::Marquee { start_x, start_offset }) => {
                self.marquee.drag_to(start_offset - (x - start_x));
                Vec::new()
            }
            None => Vec::new(),
        }
    }

    fn down(&self, z: Zone) -> bool {
        self.pressed == Some(z) && self.pressed_inside
    }

    // --- Rendu ----------------------------------------------------------------------------------

    pub fn render(&mut self, fb: &mut Image, skin: &Skin, view: &MainView) {
        let (w, h) = self.size();
        if fb.width != w || fb.height != h {
            *fb = Image::new(w, h, 0xFF00_0000);
        }
        let mut p = Painter::new(fb, skin);
        if self.shade {
            self.render_shade(&mut p, view);
        } else {
            self.render_normal(&mut p, view);
        }
    }

    fn render_normal(&mut self, p: &mut Painter, v: &MainView) {
        use sprites::*;
        p.sprite(MAIN_BACKGROUND, 0, 0);
        p.sprite(if v.active { TITLE_BAR_ACTIVE } else { TITLE_BAR_INACTIVE }, 0, 0);
        p.sprite(if self.down(Zone::Options) { OPTIONS_BUTTON_DOWN } else { OPTIONS_BUTTON }, 6, 3);
        p.sprite(if self.down(Zone::Minimize) { MINIMIZE_BUTTON_DOWN } else { MINIMIZE_BUTTON }, 244, 3);
        p.sprite(if self.down(Zone::Shade) { SHADE_BUTTON_DOWN } else { SHADE_BUTTON }, 254, 3);
        p.sprite(if self.down(Zone::Close) { CLOSE_BUTTON_DOWN } else { CLOSE_BUTTON }, 264, 3);

        // Clutterbar.
        p.sprite(CLUTTER_BAR, 10, 22);
        for (i, (s, oy)) in CLUTTER_DOWN.iter().enumerate() {
            let toggled = (i == 1 && v.always_on_top) || (i == 3 && v.double_size);
            if self.down(Zone::Clutter(i as u8)) || toggled {
                p.sprite(*s, 10, 22 + oy);
            }
        }

        // État de lecture.
        let playing = v.transport != Transport::Stopped;
        match v.transport {
            Transport::Playing => p.sprite(STATUS_PLAYING, 26, 28),
            Transport::Paused => p.sprite(STATUS_PAUSED, 26, 28),
            Transport::Stopped => p.sprite(STATUS_STOPPED, 26, 28),
        }
        if v.transport == Transport::Playing {
            p.sprite(if v.buffering { STATUS_NOT_WORKING } else { STATUS_WORKING }, 24, 28);
        } else {
            p.sprite(STATUS_NONE, 24, 28);
        }

        // Temps.
        let blink_hidden = v.transport == Transport::Paused && (v.clock_ms / 1000) % 2 == 1;
        if playing && !blink_hidden {
            self.draw_time(p, v);
        }

        // Visualiseur.
        if v.transport == Transport::Playing && v.vis.mode != alpagamp_core::settings::VisMode::Off {
            self.vis.draw(p, 24, 43, &v.vis);
        }

        // Titre défilant.
        let text = self.marquee.text(&v.title);
        let offset = self.marquee.offset_for(&text);
        p.text(111, 27, &text, 154, offset);

        // Débit, fréquence.
        if playing {
            let kbps = match v.kbps {
                Some(k) if k > 9999 => format!("{:2}C", (k / 10000).min(99)),
                Some(k) if k > 999 => format!("{:2}H", (k / 100).min(99)),
                Some(k) => format!("{k:>3}"),
                None => "   ".into(),
            };
            p.text(111, 43, &kbps, 15, 0);
            let khz = v.khz.map(|k| format!("{:>2}", k.min(99))).unwrap_or_else(|| "  ".into());
            p.text(156, 43, &khz, 10, 0);
        }

        // Mono / stéréo.
        p.sprite(if playing && v.stereo == Some(false) { MONO_ON } else { MONO_OFF }, 212, 41);
        p.sprite(if playing && v.stereo == Some(true) { STEREO_ON } else { STEREO_OFF }, 239, 41);

        // Volume et balance.
        let vf = v.volume as i32 * 27 / 255;
        p.sprite(volume_background(vf), 107, 57);
        let vx = v.volume as i32 * 51 / 255;
        p.sprite(if self.drag == Some(Drag::Volume) { VOLUME_THUMB_DOWN } else { VOLUME_THUMB }, 107 + vx, 58);
        let bf = (v.balance as i32).abs() * 27 / 127;
        p.sprite(balance_background(bf), 177, 57);
        let bx = v.balance as i32 * 12 / 127 + 12;
        p.sprite(if self.drag == Some(Drag::Balance) { BALANCE_THUMB_DOWN } else { BALANCE_THUMB }, 177 + bx, 58);

        // EQ / PL.
        let idx = |on: bool, down: bool| on as usize + 2 * down as usize;
        p.sprite(EQ_TOGGLE[idx(v.eq_open, self.down(Zone::EqToggle))], 219, 58);
        p.sprite(PL_TOGGLE[idx(v.pl_open, self.down(Zone::PlToggle))], 242, 58);

        // Barre de position.
        if playing && v.seekable && v.duration_ms.is_some() {
            p.sprite(POSBAR_BACKGROUND, 16, 72);
            let f = match self.drag {
                Some(Drag::Position(f)) => f,
                _ => position_fraction(v),
            };
            let tx = (f * 219.0).round() as i32;
            p.sprite(if matches!(self.drag, Some(Drag::Position(_))) { POSBAR_THUMB_DOWN } else { POSBAR_THUMB }, 16 + tx, 72);
        }

        // Transport.
        let btn = |p: &mut Painter, up: Sprite, down: Sprite, d: bool, x: i32, y: i32| p.sprite(if d { down } else { up }, x, y);
        btn(p, PREVIOUS, PREVIOUS_DOWN, self.down(Zone::Previous), 16, 88);
        btn(p, PLAY, PLAY_DOWN, self.down(Zone::Play), 39, 88);
        btn(p, PAUSE, PAUSE_DOWN, self.down(Zone::Pause), 62, 88);
        btn(p, STOP, STOP_DOWN, self.down(Zone::Stop), 85, 88);
        btn(p, NEXT, NEXT_DOWN, self.down(Zone::Next), 108, 88);
        btn(p, EJECT, EJECT_DOWN, self.down(Zone::Eject), 136, 89);
        p.sprite(SHUFFLE[2 * v.shuffle as usize + self.down(Zone::Shuffle) as usize], 164, 89);
        p.sprite(REPEAT[2 * v.repeat as usize + self.down(Zone::Repeat) as usize], 210, 89);
    }

    fn draw_time(&self, p: &mut Painter, v: &MainView) {
        let remaining = v.time_remaining && v.duration_ms.is_some();
        let ms = if remaining { v.duration_ms.unwrap().saturating_sub(v.position_ms) } else { v.position_ms };
        let (m, s) = display_minutes_seconds(ms);
        let nums_ex = p.skin.uses_nums_ex;
        if remaining {
            if nums_ex {
                p.sprite(sprites::NUMS_EX_MINUS, 36, 26);
            } else {
                p.sprite(sprites::NUMBERS_MINUS, 38, 32);
            }
        } else if m < 100 && nums_ex {
            p.sprite(sprites::NUMS_EX_BLANK, 36, 26);
        }
        if m >= 100 {
            p.digit((m / 100 % 10) as u8, 36, 26);
        }
        p.digit((m / 10 % 10) as u8, 48, 26);
        p.digit((m % 10) as u8, 60, 26);
        p.digit((s / 10) as u8, 78, 26);
        p.digit((s % 10) as u8, 90, 26);
    }

    fn render_shade(&mut self, p: &mut Painter, v: &MainView) {
        use sprites::*;
        p.sprite(if v.active { SHADE_BACKGROUND_ACTIVE } else { SHADE_BACKGROUND_INACTIVE }, 0, 0);
        p.sprite(if self.down(Zone::Options) { OPTIONS_BUTTON_DOWN } else { OPTIONS_BUTTON }, 6, 3);
        p.sprite(if self.down(Zone::Minimize) { MINIMIZE_BUTTON_DOWN } else { MINIMIZE_BUTTON }, 244, 3);
        p.sprite(if self.down(Zone::Shade) { UNSHADE_BUTTON_DOWN } else { UNSHADE_BUTTON }, 254, 3);
        p.sprite(if self.down(Zone::Close) { CLOSE_BUTTON_DOWN } else { CLOSE_BUTTON }, 264, 3);
        let playing = v.transport != Transport::Stopped;
        if v.transport == Transport::Playing {
            self.vis.draw_small(p, 79, 5, &v.vis);
        }
        let blink_hidden = v.transport == Transport::Paused && (v.clock_ms / 1000) % 2 == 1;
        if playing && !blink_hidden {
            let remaining = v.time_remaining && v.duration_ms.is_some();
            let ms = if remaining { v.duration_ms.unwrap().saturating_sub(v.position_ms) } else { v.position_ms };
            let (m, s) = display_minutes_seconds(ms);
            let t = format!("{}{:>2}:{:02}", if remaining { "-" } else { " " }, m.min(99), s);
            p.text(127, 4, &t, 30, 0);
        }
        if playing && v.seekable && v.duration_ms.is_some() {
            p.sprite(SHADE_POSITION_BACKGROUND, 226, 4);
            let f = match self.drag {
                Some(Drag::Position(f)) => f,
                _ => position_fraction(v),
            };
            let tx = (f * 14.0).round() as i32;
            let thumb = if f < 1.0 / 3.0 {
                SHADE_POSITION_THUMB_LEFT
            } else if f < 2.0 / 3.0 {
                SHADE_POSITION_THUMB
            } else {
                SHADE_POSITION_THUMB_RIGHT
            };
            p.sprite(thumb, 226 + tx, 4);
        }

    }
}

fn position_fraction(v: &MainView) -> f64 {
    match v.duration_ms {
        Some(d) if d > 0 => (v.position_ms as f64 / d as f64).clamp(0.0, 1.0),
        _ => 0.0,
    }
}
