//! L'éditeur de playlist : fenêtre redimensionnable (pas de 25x29), liste, menus en bande, mini transport.

use alpagamp_core::playlist::TotalTime;
use alpagamp_core::time::format_duration;
use alpagamp_core::Entry;
use alpagamp_skin::sprites::{pl_menu, *};
use alpagamp_skin::{Image, Skin};

use crate::paint::Painter;
use crate::text::{Align, Rect, TextRun};
use crate::{Action, Mods};

pub const MIN_W: u32 = 275;
pub const MIN_H: u32 = 116;
pub const ROW_H: i32 = 13;
const TOP: i32 = 20;
const BOTTOM: i32 = 38;
const LEFT: i32 = 12;
const RIGHT: i32 = 20;
pub const FONT_SIZE: f32 = 9.0;

/// Choix des menus en bande.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlMenuItem {
    AddUrl,
    AddDir,
    AddFile,
    RemoveAll,
    Crop,
    RemoveSelected,
    RemoveMisc,
    InvertSelection,
    SelectNone,
    SelectAll,
    Sort,
    FileInfo,
    MiscOptions,
    NewList,
    SaveList,
    LoadList,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuKind {
    Add,
    Rem,
    Sel,
    Misc,
    List,
}

impl MenuKind {
    /// (élément, sprites normal/enfoncé) du haut vers le bas.
    fn items(self) -> &'static [(PlMenuItem, (Sprite, Sprite))] {
        use PlMenuItem::*;
        match self {
            MenuKind::Add => &[(AddUrl, pl_menu::ADD_URL), (AddDir, pl_menu::ADD_DIR), (AddFile, pl_menu::ADD_FILE)],
            MenuKind::Rem => &[(RemoveAll, pl_menu::REM_ALL), (Crop, pl_menu::REM_CROP), (RemoveSelected, pl_menu::REM_SEL), (RemoveMisc, pl_menu::REM_MISC)],
            MenuKind::Sel => &[(InvertSelection, pl_menu::SEL_INV), (SelectNone, pl_menu::SEL_ZERO), (SelectAll, pl_menu::SEL_ALL)],
            MenuKind::Misc => &[(Sort, pl_menu::MISC_SORT), (FileInfo, pl_menu::MISC_INFO), (MiscOptions, pl_menu::MISC_OPTS)],
            MenuKind::List => &[(NewList, pl_menu::LIST_NEW), (SaveList, pl_menu::LIST_SAVE), (LoadList, pl_menu::LIST_LOAD)],
        }
    }

    fn bar(self) -> Sprite {
        match self {
            MenuKind::Add => pl_menu::ADD_BAR,
            MenuKind::Rem => pl_menu::REM_BAR,
            MenuKind::Sel => pl_menu::SEL_BAR,
            MenuKind::Misc => pl_menu::MISC_BAR,
            MenuKind::List => pl_menu::LIST_BAR,
        }
    }
}

/// Ce que la playlist affiche.
pub struct PlaylistView<'a> {
    pub entries: &'a [Entry],
    pub current: Option<usize>,
    pub active: bool,
    pub total: TotalTime,
    pub selected: TotalTime,
    /// Temps du morceau (« 1:23 », « -2:34 »), vide à l'arrêt.
    pub time_text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Zone {
    Close,
    Shade,
    Row(usize),
    EmptyList,
    Scrollbar,
    ScrollUp,
    ScrollDown,
    Menu(MenuKind),
    Mini(u8),
    Resize,
    TitleBar,
    Background,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    Rows { start_y: i32, moved: isize, clicked: usize, shift: bool, ctrl: bool, dragged: bool },
    Scroll { grab: i32 },
    Resize { start_w: u32, start_h: u32, start_x: i32, start_y: i32 },
}

pub struct PlaylistWindow {
    pub w: u32,
    pub h: u32,
    pub shade: bool,
    pub scroll: usize,
    pressed: Option<Zone>,
    pressed_inside: bool,
    drag: Option<Drag>,
    /// Menu ouvert : (menu, élément survolé, armé). Armé = un relâchement sur un élément le choisit.
    menu: Option<(MenuKind, Option<usize>, bool)>,
    /// Texte à dessiner par-dessus l'image (rempli par `render`).
    pub text: Vec<TextRun>,
    len: usize,
}

fn inside(x: i32, y: i32, rx: i32, ry: i32, rw: i32, rh: i32) -> bool {
    x >= rx && y >= ry && x < rx + rw && y < ry + rh
}

/// Arrondit une taille aux pas du format (25x29), avec les minimums.
pub fn snap_size(w: i32, h: i32) -> (u32, u32) {
    let sw = (((w - MIN_W as i32) as f32 / 25.0).round().max(0.0) as u32) * 25 + MIN_W;
    let sh = (((h - MIN_H as i32) as f32 / 29.0).round().max(0.0) as u32) * 29 + MIN_H;
    (sw, sh)
}

impl Default for PlaylistWindow {
    fn default() -> Self {
        Self::new(MIN_W, MIN_H + 29 * 2)
    }
}

impl PlaylistWindow {
    pub fn new(w: u32, h: u32) -> Self {
        let (w, h) = snap_size(w as i32, h as i32);
        Self { w, h, shade: false, scroll: 0, pressed: None, pressed_inside: false, drag: None, menu: None, text: Vec::new(), len: 0 }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.w, if self.shade { 14 } else { self.h })
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.is_some() || self.pressed.is_some() || self.menu.is_some()
    }

    pub fn visible_rows(&self) -> usize {
        ((self.h as i32 - TOP - BOTTOM) / ROW_H).max(1) as usize
    }

    fn max_scroll(&self) -> usize {
        self.len.saturating_sub(self.visible_rows())
    }

    pub fn scroll_by(&mut self, rows: isize) {
        self.scroll = (self.scroll as isize + rows).clamp(0, self.max_scroll() as isize) as usize;
    }

    /// Fait défiler pour que la ligne `i` soit visible.
    pub fn ensure_visible(&mut self, i: usize) {
        let vis = self.visible_rows();
        if i < self.scroll {
            self.scroll = i;
        } else if i >= self.scroll + vis {
            self.scroll = i + 1 - vis;
        }
    }

    fn menu_button_x(&self, k: MenuKind) -> i32 {
        match k {
            MenuKind::Add => 14,
            MenuKind::Rem => 43,
            MenuKind::Sel => 72,
            MenuKind::Misc => 101,
            MenuKind::List => self.w as i32 - 44,
        }
    }

    /// Curseur du skin pour ce point (nom du fichier `.cur`).
    pub fn cursor_at(&self, x: i32, y: i32) -> &'static str {
        let z = self.zone_at(x, y);
        if self.shade {
            return match z {
                Zone::Resize => "pwssize.cur",
                Zone::Close => "pclose.cur",
                Zone::Shade => "pwinbut.cur",
                _ => "pwsnorm.cur",
            };
        }
        match z {
            Zone::Close => "pclose.cur",
            Zone::Shade => "pwinbut.cur",
            Zone::Resize => "psize.cur",
            Zone::TitleBar => "ptbar.cur",
            Zone::Scrollbar | Zone::ScrollUp | Zone::ScrollDown => "pvscroll.cur",
            _ => "pnormal.cur",
        }
    }

    fn zone_at(&self, x: i32, y: i32) -> Zone {
        let (w, h) = (self.w as i32, self.h as i32);
        if self.shade {
            return if inside(x, y, w - 11, 3, 9, 9) {
                Zone::Close
            } else if inside(x, y, w - 20, 3, 9, 9) {
                Zone::Shade
            } else if inside(x, y, w - 29, 3, 9, 9) {
                Zone::Resize
            } else {
                Zone::TitleBar
            };
        }
        if y < TOP {
            return if inside(x, y, w - 11, 3, 9, 9) {
                Zone::Close
            } else if inside(x, y, w - 20, 3, 9, 9) {
                Zone::Shade
            } else {
                Zone::TitleBar
            };
        }
        if y < h - BOTTOM {
            if x >= LEFT && x < w - RIGHT {
                let row = self.scroll + ((y - TOP) / ROW_H) as usize;
                return if row < self.len { Zone::Row(row) } else { Zone::EmptyList };
            }
            if x >= w - 15 && x < w - 7 {
                return Zone::Scrollbar;
            }
            return Zone::Background;
        }
        let by = h - 30;
        for k in [MenuKind::Add, MenuKind::Rem, MenuKind::Sel, MenuKind::Misc, MenuKind::List] {
            if inside(x, y, self.menu_button_x(k), by, 22, 18) {
                return Zone::Menu(k);
            }
        }
        let bx = w - 150;
        for i in 0..6 {
            if inside(x, y, bx + 3 + 10 * i, h - BOTTOM + 22, 10, 10) {
                return Zone::Mini(i as u8);
            }
        }
        if inside(x, y, w - 15, h - BOTTOM + 2, 8, 5) {
            return Zone::ScrollUp;
        }
        if inside(x, y, w - 15, h - BOTTOM + 8, 8, 5) {
            return Zone::ScrollDown;
        }
        if inside(x, y, w - 20, h - 20, 20, 20) {
            return Zone::Resize;
        }
        Zone::Background
    }

    /// Ligne de la liste sous ce point, s'il y en a une.
    pub fn row_at(&self, x: i32, y: i32) -> Option<usize> {
        match self.zone_at(x, y) {
            Zone::Row(i) => Some(i),
            _ => None,
        }
    }

    /// Le point est-il dans la zone de la liste (lignes ou espace vide) ?
    pub fn in_list(&self, x: i32, y: i32) -> bool {
        matches!(self.zone_at(x, y), Zone::Row(_) | Zone::EmptyList)
    }

    /// Élément de menu sous la souris quand un menu est ouvert.
    fn menu_item_at(&self, x: i32, y: i32) -> Option<usize> {
        let (k, _, _) = self.menu?;
        self.menu_item_at_kind(k, x, y)
    }

    fn menu_item_at_kind(&self, k: MenuKind, x: i32, y: i32) -> Option<usize> {
        let items = k.items();
        let bx = self.menu_button_x(k);
        let bottom = self.h as i32 - 30;
        let top = bottom - 18 * (items.len() as i32 - 1);
        if x >= bx && x < bx + 22 && y >= top && y < bottom + 18 {
            Some(((y - top) / 18) as usize)
        } else {
            None
        }
    }

    fn thumb_y(&self) -> i32 {
        let track = self.h as i32 - TOP - BOTTOM - 18;
        let max = self.max_scroll();
        if max == 0 { TOP } else { TOP + (self.scroll as i32 * track) / max as i32 }
    }

    // --- Souris ---------------------------------------------------------------------------------

    pub fn mouse_down(&mut self, x: i32, y: i32, mods: Mods, entries: &[Entry]) -> Vec<Action> {
        self.len = entries.len();
        if self.menu.is_some() {
            // Un menu ouvert : clic sur un élément (géré au relâchement) ou ailleurs (fermeture).
            match self.menu_item_at(x, y) {
                None => self.menu = None,
                Some(i) => {
                    if let Some(m) = self.menu.as_mut() {
                        m.1 = Some(i);
                        m.2 = true;
                    }
                }
            }
            return Vec::new();
        }
        let z = self.zone_at(x, y);
        match z {
            Zone::Close | Zone::Shade | Zone::Mini(_) | Zone::ScrollUp | Zone::ScrollDown => {
                self.pressed = Some(z);
                self.pressed_inside = true;
                Vec::new()
            }
            Zone::Menu(k) => {
                let start = self.menu_item_at_kind(k, x, y);
                self.menu = Some((k, start, false));
                Vec::new()
            }
            Zone::Row(i) => {
                let already = entries.get(i).is_some_and(|e| e.selected);
                if already && !mods.shift && !mods.ctrl {
                    self.drag = Some(Drag::Rows { start_y: y, moved: 0, clicked: i, shift: false, ctrl: false, dragged: false });
                    Vec::new()
                } else {
                    self.drag = Some(Drag::Rows { start_y: y, moved: 0, clicked: i, shift: mods.shift, ctrl: mods.ctrl, dragged: true });
                    vec![Action::PlaylistClick { index: i, shift: mods.shift, ctrl: mods.ctrl }]
                }
            }
            Zone::EmptyList => vec![Action::PlaylistMenu(PlMenuItem::SelectNone)],
            Zone::Scrollbar => {
                let ty = self.thumb_y();
                let grab = if y >= ty && y < ty + 18 { y - ty } else { 9 };
                self.drag = Some(Drag::Scroll { grab });
                self.scroll_to_thumb(y - grab);
                Vec::new()
            }
            Zone::Resize => {
                self.drag = Some(Drag::Resize { start_w: self.w, start_h: self.h, start_x: x, start_y: y });
                Vec::new()
            }
            Zone::TitleBar | Zone::Background => vec![Action::StartMove],
        }
    }

    fn scroll_to_thumb(&mut self, thumb_top: i32) {
        let track = (self.h as i32 - TOP - BOTTOM - 18).max(1);
        let f = ((thumb_top - TOP) as f32 / track as f32).clamp(0.0, 1.0);
        self.scroll = (f * self.max_scroll() as f32).round() as usize;
    }

    pub fn mouse_move(&mut self, x: i32, y: i32, _mods: Mods) -> Vec<Action> {
        if let Some((k, hover, armed)) = self.menu {
            let now = self.menu_item_at(x, y);
            self.menu = Some((k, now, armed || (now.is_some() && now != hover)));
            return Vec::new();
        }
        if let Some(p) = self.pressed {
            self.pressed_inside = self.zone_at(x, y) == p;
            return Vec::new();
        }
        match self.drag {
            Some(Drag::Rows { start_y, moved, clicked, shift, ctrl, .. }) => {
                let target = ((y - start_y) as f32 / ROW_H as f32).round() as isize;
                if target != moved && !shift && !ctrl {
                    self.drag = Some(Drag::Rows { start_y, moved: target, clicked, shift, ctrl, dragged: true });
                    return vec![Action::PlaylistMoveSelection(target - moved)];
                }
                Vec::new()
            }
            Some(Drag::Scroll { grab }) => {
                self.scroll_to_thumb(y - grab);
                Vec::new()
            }
            Some(Drag::Resize { start_w, start_h, start_x, start_y }) => {
                let (nw, nh) = snap_size(start_w as i32 + x - start_x, start_h as i32 + (if self.shade { 0 } else { y - start_y }));
                if (nw, nh) != (self.w, self.h) {
                    self.w = nw;
                    self.h = nh;
                    self.scroll = self.scroll.min(self.max_scroll());
                    return vec![Action::PlaylistResize(nw, nh)];
                }
                Vec::new()
            }
            None => Vec::new(),
        }
    }

    pub fn mouse_up(&mut self, x: i32, y: i32, mods: Mods) -> Vec<Action> {
        if let Some((k, hover, armed)) = self.menu {
            match self.menu_item_at(x, y) {
                Some(i) if armed => {
                    self.menu = None;
                    return vec![Action::PlaylistMenu(k.items()[i].0)];
                }
                // Simple clic sur le bouton : le menu reste ouvert, le clic suivant choisit.
                Some(_) => self.menu = Some((k, hover, true)),
                None => self.menu = None,
            }
            return Vec::new();
        }
        let drag = self.drag.take();
        if let Some(Drag::Rows { clicked, dragged: false, .. }) = drag {
            return vec![Action::PlaylistClick { index: clicked, shift: mods.shift, ctrl: mods.ctrl }];
        }
        let Some(p) = self.pressed.take() else { return Vec::new() };
        if self.zone_at(x, y) != p {
            return Vec::new();
        }
        let none = Mods::default();
        match p {
            Zone::Close => vec![Action::Close],
            Zone::Shade => vec![Action::ToggleShade],
            Zone::Mini(0) => vec![Action::Previous(none)],
            Zone::Mini(1) => vec![Action::Play(none)],
            Zone::Mini(2) => vec![Action::Pause],
            Zone::Mini(3) => vec![Action::Stop(none)],
            Zone::Mini(4) => vec![Action::Next(none)],
            Zone::Mini(_) => vec![Action::Eject(none)],
            Zone::ScrollUp => {
                self.scroll_by(-1);
                Vec::new()
            }
            Zone::ScrollDown => {
                self.scroll_by(1);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    pub fn double_click(&mut self, x: i32, y: i32) -> Vec<Action> {
        match self.zone_at(x, y) {
            Zone::Row(i) => {
                self.drag = None;
                vec![Action::PlaylistPlay(i)]
            }
            Zone::TitleBar => vec![Action::ToggleShade],
            _ => Vec::new(),
        }
    }

    pub fn wheel(&mut self, lines: f32) {
        self.scroll_by((-lines * 3.0).round() as isize);
    }

    // --- Rendu ----------------------------------------------------------------------------------

    pub fn render(&mut self, fb: &mut Image, skin: &Skin, view: &PlaylistView) {
        self.len = view.entries.len();
        self.scroll = self.scroll.min(self.max_scroll());
        let (w, h) = self.size();
        if fb.width != w || fb.height != h {
            *fb = Image::new(w, h, 0xFF00_0000);
        }
        self.text.clear();
        let mut p = Painter::new(fb, skin);
        if self.shade {
            self.render_shade(&mut p, view);
        } else {
            self.render_normal(&mut p, view);
        }
    }

    fn render_normal(&mut self, p: &mut Painter, v: &PlaylistView) {
        let (w, h) = (self.w as i32, self.h as i32);
        let a = v.active;
        // Barre de titre : coins, titre centré, tuiles.
        let (tl, title, tile, tr) = if a {
            (PL_TOP_LEFT_ACTIVE, PL_TITLE_ACTIVE, PL_TOP_TILE_ACTIVE, PL_TOP_RIGHT_ACTIVE)
        } else {
            (PL_TOP_LEFT_INACTIVE, PL_TITLE_INACTIVE, PL_TOP_TILE_INACTIVE, PL_TOP_RIGHT_INACTIVE)
        };
        let mut x = 25;
        while x < w - 25 {
            p.sprite(tile, x, 0);
            x += 25;
        }
        // Comme Winamp : 25 + 12 + remplissage + titre (100) + 13 + remplissage + 25.
        p.sprite(title, 37 + (w - 175) / 2, 0);
        p.sprite(tl, 0, 0);
        p.sprite(tr, w - 25, 0);
        // Côtés.
        let mut y = TOP;
        while y < h - BOTTOM {
            p.sprite_part(PL_LEFT_TILE, 0, y, 12, (h - BOTTOM - y).min(29));
            p.sprite_part(PL_RIGHT_TILE, w - 20, y, 20, (h - BOTTOM - y).min(29));
            y += 29;
        }
        // Bas.
        let mut x = 125;
        while x < w - 150 {
            p.sprite(PL_BOTTOM_TILE, x, h - BOTTOM);
            x += 25;
        }
        if w >= 350 {
            p.sprite(PL_VISUALIZER, w - 225, h - BOTTOM);
        }
        p.sprite(PL_BOTTOM_LEFT, 0, h - BOTTOM);
        p.sprite(PL_BOTTOM_RIGHT, w - 150, h - BOTTOM);
        // Boutons de fenêtre.
        if self.down(Zone::Close) {
            p.sprite(PL_CLOSE_DOWN, w - 11, 3);
        }
        if self.down(Zone::Shade) {
            p.sprite(PL_SHADE_DOWN, w - 20, 3);
        }
        // Liste.
        let pl = p.skin.pledit.clone();
        let area = Rect::new(LEFT, TOP, w - LEFT - RIGHT, h - TOP - BOTTOM);
        p.fill(area.x, area.y, area.w, area.h, pl.normal_bg);
        let rows = self.visible_rows();
        for r in 0..rows {
            let i = self.scroll + r;
            let Some(e) = v.entries.get(i) else { break };
            let ry = TOP + r as i32 * ROW_H;
            if e.selected {
                p.fill(LEFT, ry, area.w, ROW_H, pl.selected_bg);
            }
            let color = if Some(i) == v.current { pl.current } else { pl.normal };
            let dur = e.duration_ms.map(format_duration).unwrap_or_default();
            let dur_w = crate::text::measure(&dur, FONT_SIZE);
            let right = (w - RIGHT - 3) as f32;
            let title = format!("{}. {}", i + 1, e.display_title());
            let max_title = right - dur_w - 8.0 - (LEFT + 2) as f32;
            self.text.push(TextRun {
                x: (LEFT + 2) as f32,
                y: ry as f32 + 1.0,
                text: crate::text::ellipsize(&title, FONT_SIZE, max_title),
                color,
                size: FONT_SIZE,
                clip: Rect::new(LEFT, ry, area.w, ROW_H),
                align: Align::Left,
            });
            if !dur.is_empty() {
                self.text.push(TextRun { x: right, y: ry as f32 + 1.0, text: dur, color, size: FONT_SIZE, clip: Rect::new(LEFT, ry, area.w, ROW_H), align: Align::Right });
            }
        }
        // Barre de défilement.
        let pressed_scroll = matches!(self.drag, Some(Drag::Scroll { .. }));
        p.sprite(if pressed_scroll { PL_SCROLL_THUMB_DOWN } else { PL_SCROLL_THUMB }, w - 15, self.thumb_y());
        // Temps total « sélection/total ».
        let fmt = |t: TotalTime| -> String {
            if t.unknown {
                "?".into()
            } else {
                format!("{}{}", format_duration(t.ms), if t.partial { "+" } else { "" })
            }
        };
        let running = format!("{}/{}", fmt(v.selected), fmt(v.total));
        p.text(w - 150 + 7, h - BOTTOM + 10, &running, 90, 0);
        // Temps du morceau.
        p.text(w - 150 + 66, h - BOTTOM + 23, &format!("{:>6}", v.time_text), 30, 0);
        // Menu ouvert.
        if let Some((k, hover, _)) = self.menu {
            let items = k.items();
            let bx = self.menu_button_x(k);
            let bottom = h - 30;
            let top = bottom - 18 * (items.len() as i32 - 1);
            p.sprite(k.bar(), bx - 3, top);
            for (j, (_, (normal, sel))) in items.iter().enumerate() {
                p.sprite(if hover == Some(j) { *sel } else { *normal }, bx, top + 18 * j as i32);
            }
        }
    }

    fn render_shade(&mut self, p: &mut Painter, v: &PlaylistView) {
        let w = self.w as i32;
        let mut x = 25;
        while x < w - 50 {
            p.sprite(PL_SHADE_TILE, x, 0);
            x += 25;
        }
        p.sprite(PL_SHADE_LEFT, 0, 0);
        p.sprite(if v.active { PL_SHADE_RIGHT_ACTIVE } else { PL_SHADE_RIGHT_INACTIVE }, w - 50, 0);
        if self.down(Zone::Close) {
            p.sprite(PL_CLOSE_DOWN, w - 11, 3);
        }
        if self.down(Zone::Shade) {
            p.sprite(PL_UNSHADE_DOWN, w - 20, 3);
        }
        let title = v
            .current
            .and_then(|i| v.entries.get(i).map(|e| format!("{}. {}", i + 1, e.display_title())))
            .unwrap_or_default();
        let dur = v.current.and_then(|i| v.entries.get(i)).and_then(|e| e.duration_ms).map(format_duration).unwrap_or_default();
        let dur_w = crate::paint::text_width(&dur);
        p.text(5, 4, &title, (w - 30 - dur_w - 10).max(0), 0);
        p.text(w - 30 - dur_w, 4, &dur, dur_w, 0);
    }

    fn down(&self, z: Zone) -> bool {
        self.pressed == Some(z) && self.pressed_inside
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alpagamp_core::Source;
    use std::path::PathBuf;

    fn entries(n: usize) -> Vec<Entry> {
        (0..n).map(|i| Entry::with_meta(Source::File(PathBuf::from(format!("/m/{i}.mp3"))), Some(format!("Morceau {i}")), Some(60_000))).collect()
    }

    #[test]
    fn snap_sizes() {
        assert_eq!(snap_size(100, 50), (275, 116));
        assert_eq!(snap_size(310, 160), (300, 174));
    }

    #[test]
    fn click_rows_menus_and_render() {
        let e = entries(30);
        let mut w = PlaylistWindow::new(275, 232);
        let a = w.mouse_down(50, TOP + ROW_H * 2 + 3, Mods::default(), &e);
        assert_eq!(a, vec![Action::PlaylistClick { index: 2, shift: false, ctrl: false }]);
        w.mouse_up(50, TOP + ROW_H * 2 + 3, Mods::default());
        // Menu ADD : ouvrir puis choisir FILE (en bas).
        let h = w.h as i32;
        assert!(w.mouse_down(20, h - 25, Mods::default(), &e).is_empty());
        assert!(w.mouse_up(20, h - 25, Mods::default()).is_empty(), "le menu reste ouvert");
        w.mouse_down(20, h - 25, Mods::default(), &e);
        assert_eq!(w.mouse_up(20, h - 25, Mods::default()), vec![Action::PlaylistMenu(PlMenuItem::AddFile)]);
        // Rendu.
        let skin = Skin::default_skin();
        let mut fb = Image::new(1, 1, 0);
        let v = PlaylistView { entries: &e, current: Some(1), active: true, total: TotalTime { ms: 1_800_000, ..Default::default() }, selected: TotalTime::default(), time_text: "1:23".into() };
        w.render(&mut fb, &skin, &v);
        assert_eq!((fb.width, fb.height), (275, 232));
        assert!(w.text.len() >= w.visible_rows());
        // Redimensionnement.
        w.mouse_down(270, h - 5, Mods::default(), &e);
        let a = w.mouse_move(270 + 26, h - 5 + 30, Mods::default());
        assert_eq!(a, vec![Action::PlaylistResize(300, 261)]);
    }
}
