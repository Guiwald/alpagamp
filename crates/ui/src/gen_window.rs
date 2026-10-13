//! Fenêtres génériques skinnées (`gen.bmp` / `genex.bmp`) : « Aller au morceau » (J), « Ajouter une adresse » et
//! « Choisir un preset » (visualisation).
//!
//! Ces fenêtres servent aussi de base aux futures fenêtres (Bibliothèque Spotify, plugins).

use alpagamp_core::Entry;
use alpagamp_core::time::format_duration;
use alpagamp_skin::sprites::generic::*;
use alpagamp_skin::{Image, Skin};

use crate::paint::Painter;
use crate::text::{self, Align, Rect, TextRun};
use crate::Action;

const FONT: f32 = 9.0;
const ROW_H: i32 = 13;

/// Rôle de la fenêtre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenKind {
    /// Chercher un morceau dans la playlist et le lancer.
    Jump,
    /// Saisir l'adresse d'un flux.
    AddUrl,
    /// Chercher un preset de visualisation et l'afficher.
    Presets,
}

/// Les lignes que la fenêtre peut lister.
#[derive(Clone, Copy)]
pub enum GenList<'a> {
    /// Entrées de playlist (« Aller au morceau »).
    Entries(&'a [Entry]),
    /// (titre, détail à droite).
    Items(&'a [(String, String)]),
}

/// Touches utiles à une fenêtre générique.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenKey {
    Text(String),
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Up,
    Down,
    PageUp,
    PageDown,
    Enter,
    Escape,
}

/// Textes affichés (traduits par l'application).
pub struct GenView<'a> {
    pub list: GenList<'a>,
    pub active: bool,
    pub title: &'a str,
    pub prompt: &'a str,
    pub ok_label: &'a str,
    pub cancel_label: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Zone {
    Close,
    Ok,
    Cancel,
    Field,
    Row(usize),
    TitleBar,
    Background,
}

pub struct GenWindow {
    pub kind: GenKind,
    pub w: u32,
    pub h: u32,
    /// Texte saisi.
    pub query: String,
    caret: usize,
    /// Résultats (index de playlist) pour « Aller au morceau ».
    pub results: Vec<usize>,
    pub sel: usize,
    scroll: usize,
    query_changed: bool,
    pressed: Option<Zone>,
    pressed_inside: bool,
    pub text: Vec<TextRun>,
}

fn inside(x: i32, y: i32, rx: i32, ry: i32, rw: i32, rh: i32) -> bool {
    x >= rx && y >= ry && x < rx + rw && y < ry + rh
}

impl GenWindow {
    pub fn new(kind: GenKind) -> Self {
        let h = match kind {
            GenKind::Jump | GenKind::Presets => 116 + 29 * 4,
            GenKind::AddUrl => 116,
        };
        Self {
            kind,
            w: 275,
            h,
            query: String::new(),
            caret: 0,
            results: Vec::new(),
            sel: 0,
            scroll: 0,
            query_changed: true,
            pressed: None,
            pressed_inside: false,
            text: Vec::new(),
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.w, self.h)
    }

    pub fn is_dragging(&self) -> bool {
        self.pressed.is_some()
    }

    /// Vrai une fois après chaque changement du texte (l'application refait alors la recherche).
    pub fn take_query_changed(&mut self) -> bool {
        std::mem::take(&mut self.query_changed)
    }

    /// Sélectionne le résultat `i` et le rend visible.
    pub fn select(&mut self, i: usize) {
        if i < self.results.len() {
            self.sel = i;
            self.ensure_visible();
        }
    }

    pub fn set_results(&mut self, r: Vec<usize>) {
        self.results = r;
        self.sel = 0;
        self.scroll = 0;
    }

    // --- Géométrie ------------------------------------------------------------------------------

    fn field_rect(&self) -> Rect {
        let y = match self.kind {
            GenKind::Jump | GenKind::Presets => 36,
            GenKind::AddUrl => 40,
        };
        Rect::new(17, y, self.w as i32 - 17 - 13, 14)
    }

    fn list_rect(&self) -> Rect {
        let top = self.field_rect().y + 20;
        Rect::new(17, top, self.w as i32 - 17 - 13, self.h as i32 - 14 - 24 - top)
    }

    fn visible_rows(&self) -> usize {
        ((self.list_rect().h - 2) / ROW_H).max(1) as usize
    }

    fn button_y(&self) -> i32 {
        self.h as i32 - 14 - 19
    }

    fn ok_x(&self) -> i32 {
        self.w as i32 - 13 - 47 * 2 - 4
    }

    fn cancel_x(&self) -> i32 {
        self.w as i32 - 13 - 47
    }

    fn zone_at(&self, x: i32, y: i32) -> Zone {
        let w = self.w as i32;
        if inside(x, y, w - 11, 3, 9, 9) {
            return Zone::Close;
        }
        if y < 20 {
            return Zone::TitleBar;
        }
        let by = self.button_y();
        if inside(x, y, self.ok_x(), by, 47, 15) {
            return Zone::Ok;
        }
        if inside(x, y, self.cancel_x(), by, 47, 15) {
            return Zone::Cancel;
        }
        let f = self.field_rect();
        if inside(x, y, f.x, f.y, f.w, f.h) {
            return Zone::Field;
        }
        if self.has_list() {
            let l = self.list_rect();
            if inside(x, y, l.x, l.y + 1, l.w, l.h - 2) {
                let i = self.scroll + ((y - l.y - 1) / ROW_H) as usize;
                if i < self.results.len() {
                    return Zone::Row(i);
                }
            }
        }
        Zone::Background
    }

    fn has_list(&self) -> bool {
        matches!(self.kind, GenKind::Jump | GenKind::Presets)
    }

    fn ensure_visible(&mut self) {
        let vis = self.visible_rows();
        if self.sel < self.scroll {
            self.scroll = self.sel;
        } else if self.sel >= self.scroll + vis {
            self.scroll = self.sel + 1 - vis;
        }
    }

    fn submit(&self) -> Vec<Action> {
        match self.kind {
            GenKind::Jump => self.results.get(self.sel).map(|&i| vec![Action::JumpTo(i)]).unwrap_or_default(),
            GenKind::Presets => self.results.get(self.sel).map(|&i| vec![Action::SelectPreset(i)]).unwrap_or_default(),
            GenKind::AddUrl => {
                let q = self.query.trim();
                if q.is_empty() { Vec::new() } else { vec![Action::SubmitUrl(q.to_string())] }
            }
        }
    }

    // --- Clavier --------------------------------------------------------------------------------

    pub fn key(&mut self, k: GenKey) -> Vec<Action> {
        let byte_at = |s: &str, c: usize| s.char_indices().nth(c).map(|(b, _)| b).unwrap_or(s.len());
        let len = self.query.chars().count();
        match k {
            GenKey::Text(t) => {
                let t: String = t.chars().filter(|c| !c.is_control()).collect();
                if !t.is_empty() {
                    let b = byte_at(&self.query, self.caret);
                    self.query.insert_str(b, &t);
                    self.caret += t.chars().count();
                    self.query_changed = true;
                }
            }
            GenKey::Backspace if self.caret > 0 => {
                let b = byte_at(&self.query, self.caret - 1);
                self.query.remove(b);
                self.caret -= 1;
                self.query_changed = true;
            }
            GenKey::Delete if self.caret < len => {
                let b = byte_at(&self.query, self.caret);
                self.query.remove(b);
                self.query_changed = true;
            }
            GenKey::Left => self.caret = self.caret.saturating_sub(1),
            GenKey::Right => self.caret = (self.caret + 1).min(len),
            GenKey::Home => self.caret = 0,
            GenKey::End => self.caret = len,
            GenKey::Up | GenKey::Down | GenKey::PageUp | GenKey::PageDown if !self.results.is_empty() => {
                let page = self.visible_rows() as isize;
                let d = match k {
                    GenKey::Up => -1,
                    GenKey::Down => 1,
                    GenKey::PageUp => -page,
                    _ => page,
                };
                self.sel = (self.sel as isize + d).clamp(0, self.results.len() as isize - 1) as usize;
                self.ensure_visible();
            }
            GenKey::Enter => return self.submit(),
            GenKey::Escape => return vec![Action::Close],
            _ => {}
        }
        Vec::new()
    }

    // --- Souris ---------------------------------------------------------------------------------

    pub fn mouse_down(&mut self, x: i32, y: i32) -> Vec<Action> {
        match self.zone_at(x, y) {
            z @ (Zone::Close | Zone::Ok | Zone::Cancel) => {
                self.pressed = Some(z);
                self.pressed_inside = true;
                Vec::new()
            }
            Zone::Field => {
                self.caret = self.query.chars().count();
                Vec::new()
            }
            Zone::Row(i) => {
                self.sel = i;
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
            Zone::Close | Zone::Cancel => vec![Action::Close],
            Zone::Ok => self.submit(),
            _ => Vec::new(),
        }
    }

    pub fn double_click(&mut self, x: i32, y: i32) -> Vec<Action> {
        match self.zone_at(x, y) {
            Zone::Row(i) => {
                self.sel = i;
                self.submit()
            }
            _ => Vec::new(),
        }
    }

    pub fn wheel(&mut self, lines: f32) {
        let max = self.results.len().saturating_sub(self.visible_rows());
        self.scroll = (self.scroll as isize - (lines * 3.0).round() as isize).clamp(0, max as isize) as usize;
    }

    // --- Rendu ----------------------------------------------------------------------------------

    pub fn render(&mut self, fb: &mut Image, skin: &Skin, v: &GenView) {
        let (w, h) = (self.w as i32, self.h as i32);
        if fb.width != self.w || fb.height != self.h {
            *fb = Image::new(self.w, self.h, 0xFF00_0000);
        }
        self.text.clear();
        let colors = skin.genex_colors();
        let mut p = Painter::new(fb, skin);
        draw_frame(&mut p, w, h, v.title, v.active, self.pressed == Some(Zone::Close) && self.pressed_inside);
        p.fill(11, 20, w - 19, h - 34, colors.window_bg());
        // Invite.
        let f = self.field_rect();
        self.text.push(TextRun {
            x: f.x as f32,
            y: (f.y - 13) as f32,
            text: v.prompt.to_string(),
            color: colors.window_text(),
            size: FONT,
            clip: Rect::new(11, 20, w - 19, h - 34),
            align: Align::Left,
        });
        // Champ de saisie.
        sunken(&mut p, f, colors.item_bg(), colors.divider());
        let shown: String = self.query.clone();
        let caret_text: String = self.query.chars().take(self.caret).collect();
        let caret_x = f.x as f32 + 3.0 + text::measure(&caret_text, FONT);
        // Fait défiler le champ si le curseur dépasse.
        let shift = (caret_x - (f.x + f.w - 4) as f32).max(0.0);
        self.text.push(TextRun {
            x: f.x as f32 + 3.0 - shift,
            y: f.y as f32 + 1.0,
            text: shown,
            color: colors.item_fg(),
            size: FONT,
            clip: Rect::new(f.x + 1, f.y + 1, f.w - 2, f.h - 2),
            align: Align::Left,
        });
        if v.active {
            let cx = (caret_x - shift).round() as i32;
            p.fill(cx, f.y + 2, 1, f.h - 4, colors.item_fg());
        }
        // Liste des résultats.
        if self.has_list() {
            let l = self.list_rect();
            sunken(&mut p, l, colors.item_bg(), colors.divider());
            // Comme la playlist : couleur de sélection + texte des éléments (les couleurs « liste sélectionnée »
            // de nombreux skins sont identiques pour le texte et le fond).
            let (sel_fg, sel_bg) = (colors.item_fg(), colors.playlist_selection());
            for r in 0..self.visible_rows() {
                let i = self.scroll + r;
                let Some(&pi) = self.results.get(i) else { break };
                let (title, detail) = match v.list {
                    GenList::Entries(entries) => {
                        let Some(e) = entries.get(pi) else { continue };
                        (format!("{}. {}", pi + 1, e.display_title()), e.duration_ms.map(format_duration).unwrap_or_default())
                    }
                    GenList::Items(items) => {
                        let Some((t, d)) = items.get(pi) else { continue };
                        (t.clone(), d.clone())
                    }
                };
                let ry = l.y + 1 + r as i32 * ROW_H;
                let selected = i == self.sel;
                if selected {
                    p.fill(l.x + 1, ry, l.w - 2, ROW_H, sel_bg);
                }
                let color = if selected { sel_fg } else { colors.item_fg() };
                let clip = Rect::new(l.x + 1, ry, l.w - 2, ROW_H);
                let right = (l.x + l.w - 4) as f32;
                let left = (l.x + 3) as f32;
                // Le détail (durée, dossier) prend au plus 40 % de la largeur.
                let detail = text::ellipsize(&detail, FONT, (right - left) * 0.4);
                let max = right - text::measure(&detail, FONT) - 8.0 - left;
                self.text.push(TextRun { x: left, y: ry as f32 + 1.0, text: text::ellipsize(&title, FONT, max), color, size: FONT, clip, align: Align::Left });
                if !detail.is_empty() {
                    self.text.push(TextRun { x: right, y: ry as f32 + 1.0, text: detail, color, size: FONT, clip, align: Align::Right });
                }
            }
        }
        // Boutons.
        let by = self.button_y();
        for (zone, x, label) in [(Zone::Ok, self.ok_x(), v.ok_label), (Zone::Cancel, self.cancel_x(), v.cancel_label)] {
            let down = self.pressed == Some(zone) && self.pressed_inside;
            p.sprite(BUTTON[down as usize], x, by);
            let tw = text::measure(label, FONT);
            self.text.push(TextRun {
                x: x as f32 + (47.0 - tw) / 2.0 + down as i32 as f32,
                y: by as f32 + 2.0 + down as i32 as f32,
                text: label.to_string(),
                color: colors.button_text(),
                size: FONT,
                clip: Rect::new(x + 1, by + 1, 45, 13),
                align: Align::Left,
            });
        }
    }
}

/// Cadre creusé : fond + bord.
fn sunken(p: &mut Painter, r: Rect, bg: u32, border: u32) {
    p.fill(r.x, r.y, r.w, r.h, border);
    p.fill(r.x + 1, r.y + 1, r.w - 2, r.h - 2, bg);
}

/// Les lettres de `gen.bmp` vont de A à Z : on retire les accents (« VISUALIZACIÓN » → « VISUALIZACION »).
fn fold_title(title: &str) -> String {
    title
        .to_uppercase()
        .chars()
        .flat_map(|c| {
            let base = match c {
                'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "A",
                'Ç' => "C",
                'È' | 'É' | 'Ê' | 'Ë' => "E",
                'Ì' | 'Í' | 'Î' | 'Ï' => "I",
                'Ñ' => "N",
                'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => "O",
                'Ù' | 'Ú' | 'Û' | 'Ü' => "U",
                'Ý' | 'Ÿ' => "Y",
                'ß' => "SS",
                'Æ' => "AE",
                'Œ' => "OE",
                _ => "",
            };
            if base.is_empty() { vec![c] } else { base.chars().collect() }
        })
        .collect()
}

/// Dessine le cadre d'une fenêtre générique (barre de titre avec les lettres de `gen.bmp`, côtés, bas).
pub fn draw_frame(p: &mut Painter, w: i32, h: i32, title: &str, active: bool, close_down: bool) {
    let a = if active { 1 } else { 0 };
    let letters = p.skin.gen_letters();
    let set = &letters[if active { 0 } else { 1 }];
    let row = if active { 88 } else { 96 };
    let title: String = fold_title(title);
    let letter_w = |c: char| -> i32 {
        if c.is_ascii_uppercase() { set[(c as u8 - b'A') as usize].1 } else { 5 }
    };
    let text_w: i32 = title.chars().map(letter_w).sum::<i32>() + title.chars().count().saturating_sub(1) as i32;
    let tw = text_w + 7;
    let ts = ((w - tw) / 2).max(50);
    // Remplissages puis pièces fixes.
    let mut x = 25;
    while x < w - 25 {
        p.sprite(TOP_LEFT_RIGHT_FILL[a], x, 0);
        x += 25;
    }
    let mut x = ts;
    while x < ts + tw {
        p.sprite_part(TOP_CENTER_FILL[a], x, 0, (ts + tw - x).min(25), 20);
        x += 25;
    }
    p.sprite(TOP_LEFT_END[a], ts - 25, 0);
    p.sprite(TOP_RIGHT_END[a], ts + tw, 0);
    p.sprite(TOP_LEFT[a], 0, 0);
    p.sprite(TOP_RIGHT[a], w - 25, 0);
    // Titre.
    let sheet = alpagamp_skin::Sheet::Gen;
    let mut cx = ts + 4;
    for c in title.chars() {
        if c.is_ascii_uppercase() {
            let (lx, lw) = set[(c as u8 - b'A') as usize];
            p.sprite(alpagamp_skin::Sprite { sheet, x: lx, y: row, w: lw, h: 7 }, cx, 4);
        }
        cx += letter_w(c) + 1;
    }
    if close_down {
        p.sprite(CLOSE_DOWN, w - 11, 3);
    }
    // Côtés.
    let mut y = 20;
    while y < h - 14 - 24 {
        p.sprite_part(MIDDLE_LEFT, 0, y, 11, (h - 38 - y).min(29));
        p.sprite_part(MIDDLE_RIGHT, w - 8, y, 8, (h - 38 - y).min(29));
        y += 29;
    }
    p.sprite(MIDDLE_LEFT_BOTTOM, 0, h - 38);
    p.sprite(MIDDLE_RIGHT_BOTTOM, w - 8, h - 38);
    // Bas.
    let mut x = 125;
    while x < w - 125 {
        p.sprite(BOTTOM_FILL, x, h - 14);
        x += 25;
    }
    p.sprite(BOTTOM_LEFT, 0, h - 14);
    p.sprite(BOTTOM_RIGHT, w - 125, h - 14);
}

#[cfg(test)]
mod tests {
    use super::*;
    use alpagamp_core::Source;

    #[test]
    fn typing_navigation_and_submit() {
        let mut g = GenWindow::new(GenKind::Jump);
        g.key(GenKey::Text("abc".into()));
        g.key(GenKey::Left);
        g.key(GenKey::Backspace);
        assert_eq!(g.query, "ac");
        assert!(g.take_query_changed());
        g.set_results(vec![4, 7, 9]);
        g.key(GenKey::Down);
        g.key(GenKey::Down);
        g.key(GenKey::Down);
        assert_eq!(g.key(GenKey::Enter), vec![Action::JumpTo(9)]);
        assert_eq!(g.key(GenKey::Escape), vec![Action::Close]);
        let mut u = GenWindow::new(GenKind::AddUrl);
        u.key(GenKey::Text(" http://radio.example/stream ".into()));
        assert_eq!(u.key(GenKey::Enter), vec![Action::SubmitUrl("http://radio.example/stream".into())]);
    }

    #[test]
    fn renders() {
        let skin = Skin::default_skin();
        let entries: Vec<Entry> = (0..3).map(|i| Entry::with_meta(Source::File(format!("/m/{i}.mp3").into()), Some(format!("Titre {i}")), Some(1000))).collect();
        let mut g = GenWindow::new(GenKind::Jump);
        g.set_results(vec![0, 2]);
        let mut fb = Image::new(1, 1, 0);
        let v = GenView { list: GenList::Entries(&entries), active: true, title: "Jump to file", prompt: "Search:", ok_label: "Play", cancel_label: "Close" };
        g.render(&mut fb, &skin, &v);
        assert_eq!((fb.width, fb.height), (275, 232));
        assert!(g.text.len() >= 4);
        assert_eq!(fold_title("Visualización"), "VISUALIZACION");
    }

    #[test]
    fn presets_list_submits_index() {
        let mut g = GenWindow::new(GenKind::Presets);
        g.set_results(vec![3, 8]);
        g.key(GenKey::Down);
        assert_eq!(g.key(GenKey::Enter), vec![Action::SelectPreset(8)]);
        let items = vec![("a".to_string(), "Dancer".to_string()); 10];
        let mut fb = Image::new(1, 1, 0);
        let v = GenView { list: GenList::Items(&items), active: true, title: "Presets", prompt: "Search:", ok_label: "Show", cancel_label: "Close" };
        g.render(&mut fb, &Skin::default_skin(), &v);
        assert!(g.text.iter().any(|r| r.text == "Dancer"));
    }
}
