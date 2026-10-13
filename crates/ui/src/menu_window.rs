//! Menu skinné (fenêtre générique) pour les écrans tactiles, où il n'y a pas de menus natifs : une liste
//! d'entrées, avec coches et sous-menus (on y entre, « ‹ » pour revenir).

use alpagamp_skin::{Image, Skin};

use crate::gen_window::draw_frame;
use crate::paint::Painter;
use crate::text::{self, Align, Rect, TextRun};

const FONT: f32 = 9.0;
const ROW_H: i32 = 16;
const TOP: i32 = 20;
const BOTTOM: i32 = 14;

/// Une entrée de menu.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuItem {
    pub label: String,
    /// Identifiant renvoyé quand on la choisit (vide pour un sous-menu ou un séparateur).
    pub id: String,
    pub checked: Option<bool>,
    pub enabled: bool,
    pub children: Vec<MenuItem>,
}

impl MenuItem {
    pub fn new(id: &str, label: &str) -> MenuItem {
        MenuItem { label: label.to_string(), id: id.to_string(), checked: None, enabled: true, children: Vec::new() }
    }
    pub fn check(id: &str, label: &str, on: bool) -> MenuItem {
        MenuItem { checked: Some(on), ..MenuItem::new(id, label) }
    }
    pub fn sub(label: &str, children: Vec<MenuItem>) -> MenuItem {
        MenuItem { children, ..MenuItem::new("", label) }
    }
    pub fn disabled(mut self) -> MenuItem {
        self.enabled = false;
        self
    }
    pub fn separator() -> MenuItem {
        MenuItem { enabled: false, ..MenuItem::new("", "") }
    }
    fn is_separator(&self) -> bool {
        self.label.is_empty() && self.children.is_empty()
    }
}

/// Ce que renvoie un toucher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuResult {
    /// Rien (défilement, entrée dans un sous-menu…).
    None,
    Chosen(String),
    Closed,
}

pub struct MenuWindow {
    pub title: String,
    root: Vec<MenuItem>,
    /// Chemin dans l'arbre (index des sous-menus ouverts).
    path: Vec<usize>,
    pub w: u32,
    pub h: u32,
    scroll: usize,
    pressed_row: Option<usize>,
    pub text: Vec<TextRun>,
}

impl MenuWindow {
    /// `h` : hauteur disponible en pixels skin (le menu s'y adapte).
    pub fn new(title: &str, items: Vec<MenuItem>, w: u32, max_h: u32) -> MenuWindow {
        let mut m = MenuWindow { title: title.to_string(), root: items, path: Vec::new(), w, h: 116, scroll: 0, pressed_row: None, text: Vec::new() };
        m.fit(max_h);
        m
    }

    fn items(&self) -> &[MenuItem] {
        let mut items = &self.root[..];
        for &i in &self.path {
            items = &items[i].children;
        }
        items
    }

    /// Nombre de lignes affichées (y compris « ‹ » dans un sous-menu).
    fn rows(&self) -> usize {
        self.items().len() + usize::from(!self.path.is_empty())
    }

    fn visible_rows(&self) -> usize {
        ((self.h as i32 - TOP - BOTTOM - 4) / ROW_H).max(1) as usize
    }

    fn fit(&mut self, max_h: u32) {
        let want = TOP + BOTTOM + 4 + ROW_H * self.rows().max(3) as i32;
        // Hauteur au pas de 29 du format, bornée par la place disponible.
        let mut h = 116;
        while (h as i32) < want && h + 29 <= max_h.max(116) {
            h += 29;
        }
        self.h = h;
        self.scroll = 0;
    }

    pub fn size(&self) -> (u32, u32) {
        (self.w, self.h)
    }

    fn row_at(&self, x: i32, y: i32) -> Option<usize> {
        if x < 11 || x >= self.w as i32 - 8 || y < TOP + 2 || y >= self.h as i32 - BOTTOM - 2 {
            return None;
        }
        let r = self.scroll + ((y - TOP - 2) / ROW_H) as usize;
        (r < self.rows()).then_some(r)
    }

    pub fn press(&mut self, x: i32, y: i32) {
        self.pressed_row = self.row_at(x, y);
    }

    pub fn cancel_press(&mut self) {
        self.pressed_row = None;
    }

    /// Fait défiler de `rows` lignes (glisser).
    pub fn scroll_by(&mut self, rows: isize) {
        let max = self.rows().saturating_sub(self.visible_rows());
        self.scroll = (self.scroll as isize + rows).clamp(0, max as isize) as usize;
        self.pressed_row = None;
    }

    /// Relâché en (x, y) : choisit l'entrée touchée.
    pub fn release(&mut self, x: i32, y: i32, max_h: u32) -> MenuResult {
        let pressed = self.pressed_row.take();
        if x >= self.w as i32 - 11 && y < 14 {
            return MenuResult::Closed;
        }
        let Some(r) = self.row_at(x, y).filter(|r| Some(*r) == pressed) else { return MenuResult::None };
        if !self.path.is_empty() {
            if r == 0 {
                self.path.pop();
                self.fit(max_h);
                return MenuResult::None;
            }
            return self.choose(r - 1, max_h);
        }
        self.choose(r, max_h)
    }

    fn choose(&mut self, i: usize, max_h: u32) -> MenuResult {
        let Some(item) = self.items().get(i).cloned() else { return MenuResult::None };
        if !item.enabled || item.is_separator() {
            return MenuResult::None;
        }
        if !item.children.is_empty() {
            self.path.push(i);
            self.fit(max_h);
            return MenuResult::None;
        }
        MenuResult::Chosen(item.id)
    }

    pub fn render(&mut self, fb: &mut Image, skin: &Skin) {
        let (w, h) = (self.w as i32, self.h as i32);
        if fb.width != self.w || fb.height != self.h {
            *fb = Image::new(self.w, self.h, 0xFF00_0000);
        }
        self.text.clear();
        let colors = skin.genex_colors();
        let mut p = Painter::new(fb, skin);
        let title = match self.path.last() {
            Some(_) => {
                let mut items = &self.root[..];
                let mut name = String::new();
                for &i in &self.path {
                    name = items[i].label.clone();
                    items = &items[i].children;
                }
                name
            }
            None => self.title.clone(),
        };
        draw_frame(&mut p, w, h, &title, true, false);
        let list = Rect::new(11, TOP, w - 19, h - TOP - BOTTOM);
        p.fill(list.x, list.y, list.w, list.h, colors.item_bg());
        let mut rows: Vec<(String, Option<bool>, bool, bool)> = Vec::new();
        if !self.path.is_empty() {
            rows.push(("‹".into(), None, true, false));
        }
        for it in self.items() {
            rows.push((it.label.clone(), it.checked, it.enabled, !it.children.is_empty()));
        }
        let fg = colors.item_fg();
        let dim = mix(colors.item_fg(), colors.item_bg());
        for (k, (label, checked, enabled, sub)) in rows.iter().enumerate().skip(self.scroll).take(self.visible_rows()) {
            let ry = TOP + 2 + (k - self.scroll) as i32 * ROW_H;
            let clip = Rect::new(list.x, ry, list.w, ROW_H);
            if label.is_empty() && !sub {
                p.fill(list.x + 4, ry + ROW_H / 2, list.w - 8, 1, colors.divider());
                continue;
            }
            if self.pressed_row == Some(k) && *enabled {
                p.fill(list.x, ry, list.w, ROW_H, colors.playlist_selection());
            }
            let color = if *enabled { fg } else { dim };
            if let Some(on) = checked {
                let mark = if *on { "●" } else { "○" };
                self.text.push(TextRun { x: (list.x + 4) as f32, y: ry as f32 + 3.0, text: mark.into(), color, size: FONT, clip, align: Align::Left });
            }
            self.text.push(TextRun {
                x: (list.x + 16) as f32,
                y: ry as f32 + 3.0,
                text: text::ellipsize(label, FONT, (list.w - 30) as f32),
                color,
                size: FONT,
                clip,
                align: Align::Left,
            });
            if *sub {
                self.text.push(TextRun { x: (list.x + list.w - 5) as f32, y: ry as f32 + 3.0, text: "›".into(), color, size: FONT, clip, align: Align::Right });
            }
        }
    }
}

fn mix(a: u32, b: u32) -> u32 {
    let m = |s: u32| (((a >> s) & 0xFF) + ((b >> s) & 0xFF)) / 2;
    0xFF00_0000 | (m(16) << 16) | (m(8) << 8) | m(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigate_and_choose() {
        let items = vec![
            MenuItem::new("a", "Alpha"),
            MenuItem::sub("Options", vec![MenuItem::check("rep", "Répéter", true), MenuItem::new("shuf", "Aléatoire")]),
            MenuItem::separator(),
            MenuItem::new("x", "Désactivé").disabled(),
        ];
        let mut m = MenuWindow::new("Menu", items, 275, 400);
        let y = |row: i32| TOP + 2 + row * ROW_H + 4;
        m.press(50, y(1));
        assert_eq!(m.release(50, y(1), 400), MenuResult::None, "entre dans le sous-menu");
        m.press(50, y(1));
        assert_eq!(m.release(50, y(1), 400), MenuResult::Chosen("rep".into()));
        m.press(50, y(0));
        m.release(50, y(0), 400);
        m.press(50, y(3));
        assert_eq!(m.release(50, y(3), 400), MenuResult::None, "entrée désactivée");
        m.press(50, y(0));
        assert_eq!(m.release(50, y(0), 400), MenuResult::Chosen("a".into()));
        let skin = Skin::default_skin();
        let mut fb = Image::new(1, 1, 0);
        m.render(&mut fb, &skin);
        assert!(!m.text.is_empty());
    }
}
