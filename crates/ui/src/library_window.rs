//! La Bibliothèque (ADR 0010) : fenêtre générique skinnée avec des onglets (Playlists, Likés, Albums,
//! Recherche, Radios), une liste où l'on entre dans les playlists et albums, et les boutons Lire / Ajouter.

use alpagamp_core::library::{LibraryItem, LibraryKind};
use alpagamp_core::time::format_duration;
use alpagamp_skin::sprites::generic::BUTTON;
use alpagamp_skin::{Image, Skin};

use crate::gen_window::{GenKey, draw_frame};
use crate::paint::Painter;
use crate::text::{self, Align, Rect, TextRun};
use crate::Action;

const FONT: f32 = 9.0;
const ROW_H: i32 = 13;

/// Onglets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibTab {
    Playlists,
    Liked,
    Albums,
    Search,
    Radios,
}

impl LibTab {
    pub const ALL: [LibTab; 5] = [LibTab::Playlists, LibTab::Liked, LibTab::Albums, LibTab::Search, LibTab::Radios];

    /// Onglet avec un champ de recherche.
    pub fn has_query(self) -> bool {
        matches!(self, LibTab::Search | LibTab::Radios)
    }
}

/// Ce que demande la Bibliothèque.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryAction {
    /// Afficher cet onglet (l'application charge la liste).
    Tab(LibTab),
    /// Ouvrir le contenant (playlist, album) : l'application charge ses morceaux.
    Open(LibraryItem),
    /// Jouer : tous les éléments jouables de la vue, en commençant à `start`.
    Play { items: Vec<LibraryItem>, start: usize },
    /// Ajouter à la playlist.
    Enqueue(Vec<LibraryItem>),
    Search { tab: LibTab, query: String },
    /// Ligne d'action (« Se connecter à Spotify »).
    Activate(LibraryItem),
}

/// Textes (traduits par l'application).
pub struct LibraryView<'a> {
    pub active: bool,
    pub title: &'a str,
    pub tabs: [&'a str; 5],
    pub prompt: &'a str,
    pub back: &'a str,
    pub play: &'a str,
    pub add: &'a str,
    /// Ligne d'état (connexion à Spotify…).
    pub status: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Zone {
    Close,
    Tab(usize),
    Field,
    Row(usize),
    Back,
    Play,
    Add,
    TitleBar,
    Background,
}

struct ViewState {
    title: String,
    /// Adresse du contenant affiché (playlist, album), vide à la racine d'un onglet.
    uri: String,
    items: Vec<LibraryItem>,
    sel: Option<usize>,
    scroll: usize,
}

pub struct LibraryWindow {
    pub w: u32,
    pub h: u32,
    pub tab: LibTab,
    pub query: String,
    caret: usize,
    view: ViewState,
    stack: Vec<ViewState>,
    pressed: Option<Zone>,
    pressed_inside: bool,
    pub text: Vec<TextRun>,
}

fn inside(x: i32, y: i32, rx: i32, ry: i32, rw: i32, rh: i32) -> bool {
    x >= rx && y >= ry && x < rx + rw && y < ry + rh
}

impl Default for LibraryWindow {
    fn default() -> Self {
        Self::new(116 + 29 * 6)
    }
}

impl LibraryWindow {
    pub fn new(h: u32) -> Self {
        LibraryWindow {
            w: 275,
            h: h.max(116 + 29 * 2),
            tab: LibTab::Playlists,
            query: String::new(),
            caret: 0,
            view: ViewState { title: String::new(), uri: String::new(), items: Vec::new(), sel: None, scroll: 0 },
            stack: Vec::new(),
            pressed: None,
            pressed_inside: false,
            text: Vec::new(),
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.w, self.h)
    }

    pub fn set_height(&mut self, h: u32) {
        self.h = h.max(116 + 29 * 2);
    }

    pub fn is_dragging(&self) -> bool {
        self.pressed.is_some()
    }

    /// Remplace les éléments de la vue courante.
    pub fn set_items(&mut self, items: Vec<LibraryItem>) {
        self.view.items = items;
        self.view.sel = None;
        self.view.scroll = 0;
    }

    pub fn items(&self) -> &[LibraryItem] {
        &self.view.items
    }

    /// Les éléments de la vue courante, modifiables sur place (comptes à rebours).
    pub fn items_mut(&mut self) -> &mut [LibraryItem] {
        &mut self.view.items
    }

    /// Remplace les éléments de la vue courante en gardant la position (liste actualisée) : l'élément
    /// sélectionné reste sélectionné s'il est toujours là, le défilement est gardé.
    pub fn update_items(&mut self, items: Vec<LibraryItem>) {
        let selected = self.view.sel.and_then(|i| self.view.items.get(i)).map(|it| it.uri.clone()).filter(|u| !u.is_empty());
        self.view.items = items;
        self.view.sel = selected.and_then(|u| self.view.items.iter().position(|it| it.uri == u));
        let max = self.view.items.len().saturating_sub(self.visible_rows());
        self.view.scroll = self.view.scroll.min(max);
    }

    /// Adresse du contenant affiché (playlist, album), s'il y en a un.
    pub fn current_uri(&self) -> Option<&str> {
        Some(self.view.uri.as_str()).filter(|u| !u.is_empty())
    }

    /// Change d'onglet (vide la navigation).
    pub fn set_tab(&mut self, tab: LibTab) {
        self.tab = tab;
        self.stack.clear();
        self.view = ViewState { title: String::new(), uri: String::new(), items: Vec::new(), sel: None, scroll: 0 };
    }

    /// Entre dans un contenant : la vue courante est gardée pour « Retour ».
    pub fn push(&mut self, title: &str, items: Vec<LibraryItem>) {
        self.push_uri(title, "", items);
    }

    /// Entre dans le contenant `uri` (pour pouvoir l'actualiser).
    pub fn push_uri(&mut self, title: &str, uri: &str, items: Vec<LibraryItem>) {
        let prev = std::mem::replace(&mut self.view, ViewState { title: title.to_string(), uri: uri.to_string(), items, sel: None, scroll: 0 });
        self.stack.push(prev);
    }

    pub fn can_go_back(&self) -> bool {
        !self.stack.is_empty()
    }

    pub fn back(&mut self) {
        if let Some(v) = self.stack.pop() {
            self.view = v;
        }
    }

    // --- Géométrie ------------------------------------------------------------------------------

    fn field_rect(&self) -> Rect {
        Rect::new(13, 41, self.w as i32 - 26, 14)
    }

    fn list_rect(&self) -> Rect {
        let top = if self.tab.has_query() { 59 } else { 41 };
        Rect::new(13, top, self.w as i32 - 26, self.h as i32 - 14 - 24 - top)
    }

    fn visible_rows(&self) -> usize {
        ((self.list_rect().h - 2) / ROW_H).max(1) as usize
    }

    fn button_y(&self) -> i32 {
        self.h as i32 - 14 - 19
    }

    fn zone_at(&self, x: i32, y: i32) -> Zone {
        let w = self.w as i32;
        if inside(x, y, w - 11, 3, 9, 9) {
            return Zone::Close;
        }
        if y < 20 {
            return Zone::TitleBar;
        }
        for i in 0..5 {
            if inside(x, y, 13 + 50 * i as i32, 22, 47, 15) {
                return Zone::Tab(i);
            }
        }
        let by = self.button_y();
        if self.can_go_back() && inside(x, y, 13, by, 47, 15) {
            return Zone::Back;
        }
        if inside(x, y, w - 13 - 47 * 2 - 4, by, 47, 15) {
            return Zone::Play;
        }
        if inside(x, y, w - 13 - 47, by, 47, 15) {
            return Zone::Add;
        }
        if self.tab.has_query() {
            let f = self.field_rect();
            if inside(x, y, f.x, f.y, f.w, f.h) {
                return Zone::Field;
            }
        }
        let l = self.list_rect();
        if inside(x, y, l.x, l.y + 1, l.w, l.h - 2) {
            let i = self.view.scroll + ((y - l.y - 1) / ROW_H) as usize;
            if i < self.view.items.len() {
                return Zone::Row(i);
            }
        }
        Zone::Background
    }

    fn ensure_visible(&mut self) {
        let vis = self.visible_rows();
        if let Some(s) = self.view.sel {
            if s < self.view.scroll {
                self.view.scroll = s;
            } else if s >= self.view.scroll + vis {
                self.view.scroll = s + 1 - vis;
            }
        }
    }

    /// Action sur la ligne `i` (double-clic, Entrée).
    fn activate(&self, i: usize) -> Vec<Action> {
        let Some(it) = self.view.items.get(i) else { return Vec::new() };
        let a = match it.kind {
            LibraryKind::Playlist | LibraryKind::Album => LibraryAction::Open(it.clone()),
            LibraryKind::Track | LibraryKind::Radio => self.play_from(i),
            LibraryKind::Action => LibraryAction::Activate(it.clone()),
            LibraryKind::Info => return Vec::new(),
        };
        vec![Action::Library(a)]
    }

    fn play_from(&self, i: usize) -> LibraryAction {
        let it = &self.view.items[i];
        if it.kind == LibraryKind::Radio {
            return LibraryAction::Play { items: vec![it.clone()], start: 0 };
        }
        let playable: Vec<LibraryItem> = self.view.items.iter().filter(|x| x.kind == LibraryKind::Track).cloned().collect();
        let start = self.view.items[..i].iter().filter(|x| x.kind == LibraryKind::Track).count();
        LibraryAction::Play { items: playable, start }
    }

    fn play_button(&self) -> Vec<Action> {
        match self.view.sel {
            Some(i) => self.activate(i),
            None => {
                if self.view.items.iter().any(|x| x.kind == LibraryKind::Track) {
                    let first = self.view.items.iter().position(|x| x.kind == LibraryKind::Track).unwrap_or(0);
                    vec![Action::Library(self.play_from(first))]
                } else {
                    Vec::new()
                }
            }
        }
    }

    fn add_button(&self) -> Vec<Action> {
        let items: Vec<LibraryItem> = match self.view.sel.and_then(|i| self.view.items.get(i)) {
            Some(it) if it.is_container() => return vec![Action::Library(LibraryAction::Open(it.clone()))],
            Some(it) if it.is_playable() => vec![it.clone()],
            _ => self.view.items.iter().filter(|x| x.is_playable()).cloned().collect(),
        };
        if items.is_empty() { Vec::new() } else { vec![Action::Library(LibraryAction::Enqueue(items))] }
    }

    // --- Clavier --------------------------------------------------------------------------------

    pub fn key(&mut self, k: GenKey) -> Vec<Action> {
        let byte_at = |s: &str, c: usize| s.char_indices().nth(c).map(|(b, _)| b).unwrap_or(s.len());
        let len = self.query.chars().count();
        let q = self.tab.has_query();
        match k {
            GenKey::Text(t) if q => {
                let t: String = t.chars().filter(|c| !c.is_control()).collect();
                let b = byte_at(&self.query, self.caret);
                self.query.insert_str(b, &t);
                self.caret += t.chars().count();
            }
            GenKey::Backspace if q && self.caret > 0 => {
                let b = byte_at(&self.query, self.caret - 1);
                self.query.remove(b);
                self.caret -= 1;
            }
            GenKey::Backspace if self.can_go_back() => self.back(),
            GenKey::Delete if q && self.caret < len => {
                let b = byte_at(&self.query, self.caret);
                self.query.remove(b);
            }
            GenKey::Left => self.caret = self.caret.saturating_sub(1),
            GenKey::Right => self.caret = (self.caret + 1).min(len),
            GenKey::Home => self.caret = 0,
            GenKey::End => self.caret = len,
            GenKey::Up | GenKey::Down | GenKey::PageUp | GenKey::PageDown if !self.view.items.is_empty() => {
                let page = self.visible_rows() as isize;
                let d = match k {
                    GenKey::Up => -1,
                    GenKey::Down => 1,
                    GenKey::PageUp => -page,
                    _ => page,
                };
                let cur = self.view.sel.map(|s| s as isize).unwrap_or(-1);
                self.view.sel = Some((cur + d).clamp(0, self.view.items.len() as isize - 1) as usize);
                self.ensure_visible();
            }
            GenKey::Enter => {
                if let Some(i) = self.view.sel {
                    return self.activate(i);
                }
                if q && !self.query.trim().is_empty() {
                    return vec![Action::Library(LibraryAction::Search { tab: self.tab, query: self.query.trim().to_string() })];
                }
            }
            GenKey::Escape => return vec![Action::Close],
            _ => {}
        }
        Vec::new()
    }

    // --- Souris ---------------------------------------------------------------------------------

    pub fn mouse_down(&mut self, x: i32, y: i32) -> Vec<Action> {
        match self.zone_at(x, y) {
            z @ (Zone::Close | Zone::Tab(_) | Zone::Back | Zone::Play | Zone::Add) => {
                self.pressed = Some(z);
                self.pressed_inside = true;
                Vec::new()
            }
            Zone::Field => {
                self.caret = self.query.chars().count();
                self.view.sel = None;
                Vec::new()
            }
            Zone::Row(i) => {
                self.view.sel = Some(i);
                Vec::new()
            }
            Zone::TitleBar | Zone::Background => vec![Action::StartMove],
        }
    }

    pub fn mouse_move(&mut self, x: i32, y: i32) {
        if let Some(p) = self.pressed {
            self.pressed_inside = self.zone_at(x, y) == p;
        }
    }

    pub fn mouse_up(&mut self, x: i32, y: i32) -> Vec<Action> {
        let Some(p) = self.pressed.take() else { return Vec::new() };
        if self.zone_at(x, y) != p {
            return Vec::new();
        }
        match p {
            Zone::Close => vec![Action::Close],
            Zone::Tab(i) => {
                let tab = LibTab::ALL[i];
                self.set_tab(tab);
                vec![Action::Library(LibraryAction::Tab(tab))]
            }
            Zone::Back => {
                self.back();
                Vec::new()
            }
            Zone::Play => self.play_button(),
            Zone::Add => self.add_button(),
            _ => Vec::new(),
        }
    }

    pub fn double_click(&mut self, x: i32, y: i32) -> Vec<Action> {
        match self.zone_at(x, y) {
            Zone::Row(i) => {
                self.view.sel = Some(i);
                self.activate(i)
            }
            _ => Vec::new(),
        }
    }

    /// Ligne sous ce point (pour le toucher).
    pub fn row_at(&self, x: i32, y: i32) -> Option<usize> {
        match self.zone_at(x, y) {
            Zone::Row(i) => Some(i),
            _ => None,
        }
    }

    pub fn in_list(&self, x: i32, y: i32) -> bool {
        let l = self.list_rect();
        inside(x, y, l.x, l.y, l.w, l.h)
    }

    pub fn select(&mut self, i: usize) {
        if i < self.view.items.len() {
            self.view.sel = Some(i);
        }
    }

    /// Toucher ou double-clic sur une ligne.
    pub fn activate_row(&mut self, i: usize) -> Vec<Action> {
        self.view.sel = Some(i);
        self.activate(i)
    }

    pub fn wheel(&mut self, lines: f32) {
        self.scroll_by((-lines * 3.0).round() as isize);
    }

    pub fn scroll_by(&mut self, rows: isize) {
        let max = self.view.items.len().saturating_sub(self.visible_rows());
        self.view.scroll = (self.view.scroll as isize + rows).clamp(0, max as isize) as usize;
    }

    // --- Rendu ----------------------------------------------------------------------------------

    pub fn render(&mut self, fb: &mut Image, skin: &Skin, v: &LibraryView) {
        let (w, h) = (self.w as i32, self.h as i32);
        if fb.width != self.w || fb.height != self.h {
            *fb = Image::new(self.w, self.h, 0xFF00_0000);
        }
        self.text.clear();
        let c = skin.genex_colors();
        let mut p = Painter::new(fb, skin);
        draw_frame(&mut p, w, h, v.title, v.active, self.pressed == Some(Zone::Close) && self.pressed_inside);
        p.fill(11, 20, w - 19, h - 34, c.window_bg());
        let (pressed, pressed_inside) = (self.pressed, self.pressed_inside);
        let down = move |z: Zone| pressed == Some(z) && pressed_inside;
        // Onglets.
        for (i, tab) in LibTab::ALL.iter().enumerate() {
            let x = 13 + 50 * i as i32;
            let on = *tab == self.tab || down(Zone::Tab(i));
            p.sprite(BUTTON[on as usize], x, 22);
            self.button_text(x, 22, v.tabs[i], on, c.button_text());
        }
        // Champ de recherche.
        if self.tab.has_query() {
            let f = self.field_rect();
            p.fill(f.x, f.y, f.w, f.h, c.divider());
            p.fill(f.x + 1, f.y + 1, f.w - 2, f.h - 2, c.item_bg());
            let shown = if self.query.is_empty() { v.prompt.to_string() } else { self.query.clone() };
            let color = if self.query.is_empty() { mix(c.item_fg(), c.item_bg()) } else { c.item_fg() };
            let caret_text: String = self.query.chars().take(self.caret).collect();
            let caret_x = f.x as f32 + 3.0 + text::measure(&caret_text, FONT);
            let shift = (caret_x - (f.x + f.w - 4) as f32).max(0.0);
            self.text.push(TextRun { x: f.x as f32 + 3.0 - shift, y: f.y as f32 + 1.0, text: shown, color, size: FONT, clip: Rect::new(f.x + 1, f.y + 1, f.w - 2, f.h - 2), align: Align::Left });
            if v.active && self.view.sel.is_none() {
                p.fill((caret_x - shift).round() as i32, f.y + 2, 1, f.h - 4, c.item_fg());
            }
        }
        // Liste.
        let l = self.list_rect();
        p.fill(l.x, l.y, l.w, l.h, c.divider());
        p.fill(l.x + 1, l.y + 1, l.w - 2, l.h - 2, c.item_bg());
        let dim = mix(c.item_fg(), c.item_bg());
        let rows: Vec<LibraryItem> = if self.view.items.is_empty() && !v.status.is_empty() {
            vec![LibraryItem::info(v.status)]
        } else {
            self.view.items.clone()
        };
        let offset = if self.view.items.is_empty() { 0 } else { self.view.scroll };
        for (r, it) in rows.iter().enumerate().skip(offset).take(self.visible_rows()) {
            let ry = l.y + 1 + (r - offset) as i32 * ROW_H;
            let clip = Rect::new(l.x + 1, ry, l.w - 2, ROW_H);
            let selected = self.view.sel == Some(r) && !self.view.items.is_empty();
            if selected {
                p.fill(l.x + 1, ry, l.w - 2, ROW_H, c.playlist_selection());
            }
            let color = match it.kind {
                LibraryKind::Info => dim,
                _ => c.item_fg(),
            };
            let right = (l.x + l.w - 4) as f32;
            let dur = it.duration_ms.map(format_duration).unwrap_or_default();
            let marker = match it.kind {
                LibraryKind::Playlist | LibraryKind::Album => "› ",
                LibraryKind::Action => "» ",
                _ => "",
            };
            let main = format!("{marker}{}", it.title);
            let max = right - text::measure(&dur, FONT) - 8.0 - (l.x + 3) as f32;
            let main = text::ellipsize(&main, FONT, max);
            let main_w = text::measure(&main, FONT);
            self.text.push(TextRun { x: (l.x + 3) as f32, y: ry as f32 + 1.0, text: main, color, size: FONT, clip, align: Align::Left });
            if !it.subtitle.is_empty() && main_w + 20.0 < max {
                let sub = text::ellipsize(&format!("  {}", it.subtitle), FONT, max - main_w);
                self.text.push(TextRun { x: (l.x + 3) as f32 + main_w, y: ry as f32 + 1.0, text: sub, color: dim, size: FONT, clip, align: Align::Left });
            }
            if !dur.is_empty() {
                self.text.push(TextRun { x: right, y: ry as f32 + 1.0, text: dur, color, size: FONT, clip, align: Align::Right });
            }
        }
        // Boutons du bas et titre de la vue.
        let by = self.button_y();
        if self.can_go_back() {
            p.sprite(BUTTON[down(Zone::Back) as usize], 13, by);
            self.button_text(13, by, v.back, down(Zone::Back), c.button_text());
        }
        let label = if self.view.title.is_empty() { v.status.to_string() } else { self.view.title.clone() };
        let lx = if self.can_go_back() { 64 } else { 13 };
        let lw = w - 13 - 47 * 2 - 8 - lx;
        if lw > 10 && !label.is_empty() {
            self.text.push(TextRun {
                x: lx as f32,
                y: by as f32 + 2.0,
                text: text::ellipsize(&label, FONT, lw as f32),
                color: c.window_text(),
                size: FONT,
                clip: Rect::new(lx, by, lw, 15),
                align: Align::Left,
            });
        }
        let px = w - 13 - 47 * 2 - 4;
        p.sprite(BUTTON[down(Zone::Play) as usize], px, by);
        self.button_text(px, by, v.play, down(Zone::Play), c.button_text());
        let ax = w - 13 - 47;
        p.sprite(BUTTON[down(Zone::Add) as usize], ax, by);
        self.button_text(ax, by, v.add, down(Zone::Add), c.button_text());
    }

    fn button_text(&mut self, x: i32, y: i32, label: &str, down: bool, color: u32) {
        let label = text::ellipsize(label, FONT, 43.0);
        let tw = text::measure(&label, FONT);
        let d = down as i32 as f32;
        self.text.push(TextRun { x: x as f32 + (47.0 - tw) / 2.0 + d, y: y as f32 + 2.0 + d, text: label, color, size: FONT, clip: Rect::new(x + 1, y + 1, 45, 13), align: Align::Left });
    }
}

fn mix(a: u32, b: u32) -> u32 {
    let m = |s: u32| (((a >> s) & 0xFF) + ((b >> s) & 0xFF)) / 2;
    0xFF00_0000 | (m(16) << 16) | (m(8) << 8) | m(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(n: &str) -> LibraryItem {
        LibraryItem::new(LibraryKind::Track, n, "Artiste", format!("spotify:track:{n}"))
    }

    #[test]
    fn navigation_and_play() {
        let mut w = LibraryWindow::default();
        w.set_items(vec![LibraryItem::new(LibraryKind::Playlist, "Ma liste", "moi", "spotify:playlist:x")]);
        let l = w.list_rect();
        let a = w.double_click(30, l.y + 3);
        assert!(matches!(&a[..], [Action::Library(LibraryAction::Open(it))] if it.uri == "spotify:playlist:x"));
        w.push("Ma liste", vec![track("a"), LibraryItem::info("—"), track("b")]);
        let a = w.double_click(30, l.y + 1 + 2 * ROW_H + 3);
        assert_eq!(a, vec![Action::Library(LibraryAction::Play { items: vec![track("a"), track("b")], start: 1 })]);
        assert!(w.can_go_back());
        w.key(GenKey::Backspace);
        assert_eq!(w.items().len(), 1);
        // Onglet recherche : taper, Entrée.
        let tab_x = 13 + 50 * 3 + 5;
        w.mouse_down(tab_x, 25);
        assert_eq!(w.mouse_up(tab_x, 25), vec![Action::Library(LibraryAction::Tab(LibTab::Search))]);
        w.key(GenKey::Text("daft punk".into()));
        assert_eq!(w.key(GenKey::Enter), vec![Action::Library(LibraryAction::Search { tab: LibTab::Search, query: "daft punk".into() })]);
        let skin = Skin::default_skin();
        let mut fb = Image::new(1, 1, 0);
        let v = LibraryView { active: true, title: "Bibliothèque", tabs: ["Playlists", "Likés", "Albums", "Chercher", "Radios"], prompt: "Chercher", back: "‹", play: "Lire", add: "Ajouter", status: "Chargement…" };
        w.render(&mut fb, &skin, &v);
        assert!(!w.text.is_empty());
    }

    #[test]
    fn refreshed_list_keeps_selection_and_container() {
        let mut w = LibraryWindow::default();
        w.push_uri("Punk", "spotify:playlist:p", (0..40).map(|i| track(&i.to_string())).collect());
        assert_eq!(w.current_uri(), Some("spotify:playlist:p"));
        w.select(20);
        w.scroll_by(15);
        // Liste actualisée : une ligne « Actualiser » en tête, l'élément choisi reste choisi.
        let mut fresh = vec![LibraryItem::new(LibraryKind::Action, "Actualiser", "", "action:refresh")];
        fresh.extend((0..40).map(|i| track(&i.to_string())));
        w.update_items(fresh);
        assert_eq!(w.items().len(), 41);
        assert_eq!(w.view.sel, Some(21));
        assert!(w.view.scroll > 0);
        w.items_mut()[0].title = "Réessayer".into();
        assert_eq!(w.items()[0].title, "Réessayer");
        w.back();
        assert_eq!(w.current_uri(), None);
    }
}
