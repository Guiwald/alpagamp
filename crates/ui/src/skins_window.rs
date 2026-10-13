//! Navigateur de skins (Alt+S, ADR 0012) : fenêtre générique skinnée avec deux onglets — le Musée (skins du
//! Winamp Skin Museum, avec recherche et aperçus) et Mes skins (ceux déjà rangés) — ; un clic applique le skin.

use alpagamp_skin::sprites::generic::BUTTON;
use alpagamp_skin::{Image, Skin};

use crate::Action;
use crate::gen_window::{GenKey, draw_frame};
use crate::paint::Painter;
use crate::text::{self, Align, Rect, TextRun};

const FONT: f32 = 9.0;
/// Hauteur d'une ligne : aperçu de 58 px (fenêtre principale réduite de moitié) + marges.
pub const ROW_H: i32 = 62;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkinsTab {
    Museum,
    Mine,
}

/// Ce que demande le navigateur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkinsAction {
    /// Afficher cet onglet (l'application charge la liste).
    Tab(SkinsTab),
    /// Chercher dans le musée (texte vide : les skins mis en avant).
    Search(String),
    /// Appliquer le skin de la ligne `i`.
    Apply(usize),
    /// Choisir un fichier `.wsz` sur l'ordinateur.
    OpenFile,
}

/// Une ligne.
#[derive(Clone, Debug, Default)]
pub struct SkinRow {
    /// Identifiant (md5 du musée, chemin local…).
    pub key: String,
    pub name: String,
    /// Deuxième ligne : état (téléchargement, appliqué…) ou origine.
    pub detail: String,
    pub thumb: Option<Image>,
}

/// Textes (traduits par l'application).
pub struct SkinsView<'a> {
    pub active: bool,
    pub title: &'a str,
    pub tabs: [&'a str; 2],
    pub prompt: &'a str,
    /// Bouton « Ouvrir… » (absent sur téléphone si vide).
    pub open: &'a str,
    pub close: &'a str,
    /// Ligne d'état en bas (chargement, erreur, mention du musée…).
    pub status: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Zone {
    Close,
    Tab(usize),
    Field,
    Row(usize),
    Open,
    CloseButton,
    TitleBar,
    Background,
}

pub struct SkinsWindow {
    pub w: u32,
    pub h: u32,
    pub tab: SkinsTab,
    pub query: String,
    caret: usize,
    rows: Vec<SkinRow>,
    pub sel: Option<usize>,
    scroll: usize,
    /// Nombre de lignes au moment de la dernière demande de suite.
    more_asked_at: usize,
    pressed: Option<Zone>,
    pressed_inside: bool,
    pub text: Vec<TextRun>,
}

fn inside(x: i32, y: i32, rx: i32, ry: i32, rw: i32, rh: i32) -> bool {
    x >= rx && y >= ry && x < rx + rw && y < ry + rh
}

impl Default for SkinsWindow {
    fn default() -> Self {
        Self::new(116 + 29 * 8)
    }
}

impl SkinsWindow {
    pub fn new(h: u32) -> Self {
        SkinsWindow {
            w: 275,
            h: h.max(116 + 29 * 3),
            tab: SkinsTab::Museum,
            query: String::new(),
            caret: 0,
            rows: Vec::new(),
            sel: None,
            scroll: 0,
            more_asked_at: usize::MAX,
            pressed: None,
            pressed_inside: false,
            text: Vec::new(),
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.w, self.h)
    }

    pub fn set_height(&mut self, h: u32) {
        self.h = h.max(116 + 29 * 3);
    }

    pub fn is_dragging(&self) -> bool {
        self.pressed.is_some()
    }

    pub fn rows(&self) -> &[SkinRow] {
        &self.rows
    }

    /// Remplace les lignes (nouvelle liste).
    pub fn set_rows(&mut self, rows: Vec<SkinRow>) {
        self.rows = rows;
        self.sel = None;
        self.scroll = 0;
        self.more_asked_at = usize::MAX;
    }

    /// Ajoute une page de lignes (défilement infini).
    pub fn append_rows(&mut self, rows: Vec<SkinRow>) {
        self.rows.extend(rows);
    }

    pub fn set_thumb(&mut self, key: &str, img: Image) {
        for r in self.rows.iter_mut().filter(|r| r.key == key) {
            r.thumb = Some(img.clone());
        }
    }

    pub fn set_detail(&mut self, key: &str, detail: &str) {
        for r in self.rows.iter_mut().filter(|r| r.key == key) {
            r.detail = detail.to_string();
        }
    }

    pub fn set_tab(&mut self, tab: SkinsTab) {
        self.tab = tab;
        self.set_rows(Vec::new());
    }

    /// Lignes visibles (ou presque) sans aperçu : l'application va les chercher.
    pub fn visible_without_thumb(&self) -> Vec<usize> {
        let end = (self.scroll + self.visible_rows() + 2).min(self.rows.len());
        (self.scroll..end).filter(|&i| self.rows[i].thumb.is_none()).collect()
    }

    /// Vrai une fois quand la liste du musée arrive à sa fin à l'écran : l'application charge la page suivante.
    pub fn wants_more(&mut self) -> bool {
        let n = self.rows.len();
        if self.tab == SkinsTab::Museum && n > 0 && self.scroll + self.visible_rows() + 3 >= n && self.more_asked_at != n {
            self.more_asked_at = n;
            return true;
        }
        false
    }

    // --- Géométrie ------------------------------------------------------------------------------

    fn field_rect(&self) -> Rect {
        Rect::new(113, 22, self.w as i32 - 126, 15)
    }

    fn list_rect(&self) -> Rect {
        Rect::new(13, 41, self.w as i32 - 26, self.h as i32 - 14 - 24 - 41)
    }

    pub fn visible_rows(&self) -> usize {
        (((self.list_rect().h - 2) + ROW_H - 1) / ROW_H).max(1) as usize
    }

    fn full_rows(&self) -> usize {
        ((self.list_rect().h - 2) / ROW_H).max(1) as usize
    }

    fn button_y(&self) -> i32 {
        self.h as i32 - 14 - 19
    }

    fn zone_at(&self, x: i32, y: i32, with_open: bool) -> Zone {
        let w = self.w as i32;
        if inside(x, y, w - 11, 3, 9, 9) {
            return Zone::Close;
        }
        if y < 20 {
            return Zone::TitleBar;
        }
        for i in 0..2 {
            if inside(x, y, 13 + 50 * i as i32, 22, 47, 15) {
                return Zone::Tab(i);
            }
        }
        let f = self.field_rect();
        if self.tab == SkinsTab::Museum && inside(x, y, f.x, f.y, f.w, f.h) {
            return Zone::Field;
        }
        let by = self.button_y();
        if with_open && inside(x, y, w - 13 - 47 * 2 - 4, by, 47, 15) {
            return Zone::Open;
        }
        if inside(x, y, w - 13 - 47, by, 47, 15) {
            return Zone::CloseButton;
        }
        if let Some(i) = self.row_at(x, y) {
            return Zone::Row(i);
        }
        Zone::Background
    }

    /// Ligne sous ce point.
    pub fn row_at(&self, x: i32, y: i32) -> Option<usize> {
        let l = self.list_rect();
        if !inside(x, y, l.x + 1, l.y + 1, l.w - 2, l.h - 2) {
            return None;
        }
        let i = self.scroll + ((y - l.y - 1) / ROW_H) as usize;
        (i < self.rows.len()).then_some(i)
    }

    /// Le point est-il dans la liste (pour faire défiler au doigt) ?
    pub fn in_list(&self, x: i32, y: i32) -> bool {
        let l = self.list_rect();
        inside(x, y, l.x, l.y, l.w, l.h)
    }

    pub fn scroll_by(&mut self, rows: isize) {
        let max = self.rows.len().saturating_sub(self.full_rows());
        self.scroll = (self.scroll as isize + rows).clamp(0, max as isize) as usize;
    }

    pub fn wheel(&mut self, lines: f32) {
        self.scroll_by(-(lines.round() as isize));
    }

    fn ensure_visible(&mut self, i: usize) {
        let vis = self.full_rows();
        if i < self.scroll {
            self.scroll = i;
        } else if i >= self.scroll + vis {
            self.scroll = i + 1 - vis;
        }
    }

    // --- Clavier --------------------------------------------------------------------------------

    pub fn key(&mut self, k: GenKey) -> Vec<Action> {
        let byte_at = |s: &str, c: usize| s.char_indices().nth(c).map(|(b, _)| b).unwrap_or(s.len());
        let len = self.query.chars().count();
        let typing = self.tab == SkinsTab::Museum;
        let edits = matches!(k, GenKey::Text(_) | GenKey::Backspace | GenKey::Delete);
        match k {
            GenKey::Text(t) if typing => {
                let t: String = t.chars().filter(|c| !c.is_control()).collect();
                let b = byte_at(&self.query, self.caret);
                self.query.insert_str(b, &t);
                self.caret += t.chars().count();
            }
            GenKey::Backspace if typing && self.caret > 0 => {
                let b = byte_at(&self.query, self.caret - 1);
                self.query.remove(b);
                self.caret -= 1;
            }
            GenKey::Delete if typing && self.caret < len => {
                let b = byte_at(&self.query, self.caret);
                self.query.remove(b);
            }
            GenKey::Left => self.caret = self.caret.saturating_sub(1),
            GenKey::Right => self.caret = (self.caret + 1).min(len),
            GenKey::Home => self.caret = 0,
            GenKey::End => self.caret = len,
            GenKey::Up | GenKey::Down | GenKey::PageUp | GenKey::PageDown if !self.rows.is_empty() => {
                let page = self.full_rows() as isize;
                let d = match k {
                    GenKey::Up => -1,
                    GenKey::Down => 1,
                    GenKey::PageUp => -page,
                    _ => page,
                };
                let i = (self.sel.map(|s| s as isize).unwrap_or(-1) + d).clamp(0, self.rows.len() as isize - 1) as usize;
                self.sel = Some(i);
                self.ensure_visible(i);
            }
            GenKey::Enter => {
                return match self.sel {
                    Some(i) => vec![Action::Skins(SkinsAction::Apply(i))],
                    None if typing => vec![Action::Skins(SkinsAction::Search(self.query.trim().to_string()))],
                    None => Vec::new(),
                };
            }
            GenKey::Escape => return vec![Action::Close],
            _ => {}
        }
        // Taper remet la sélection sur le champ (Entrée lance la recherche).
        if self.sel.is_some() && typing && edits {
            self.sel = None;
        }
        Vec::new()
    }

    // --- Souris ---------------------------------------------------------------------------------

    pub fn mouse_down(&mut self, x: i32, y: i32, with_open: bool) -> Vec<Action> {
        match self.zone_at(x, y, with_open) {
            z @ (Zone::Close | Zone::Tab(_) | Zone::Open | Zone::CloseButton | Zone::Row(_)) => {
                if let Zone::Row(i) = z {
                    self.sel = Some(i);
                }
                self.pressed = Some(z);
                self.pressed_inside = true;
                Vec::new()
            }
            Zone::Field => {
                self.sel = None;
                self.caret = self.query.chars().count();
                Vec::new()
            }
            Zone::TitleBar | Zone::Background => vec![Action::StartMove],
        }
    }

    pub fn mouse_move(&mut self, x: i32, y: i32, with_open: bool) {
        if let Some(p) = self.pressed {
            self.pressed_inside = self.zone_at(x, y, with_open) == p;
        }
    }

    /// Annule l'appui en cours (le doigt fait défiler la liste).
    pub fn cancel_press(&mut self) {
        self.pressed = None;
    }

    pub fn mouse_up(&mut self, x: i32, y: i32, with_open: bool) -> Vec<Action> {
        let Some(p) = self.pressed.take() else { return Vec::new() };
        if self.zone_at(x, y, with_open) != p {
            return Vec::new();
        }
        match p {
            Zone::Close | Zone::CloseButton => vec![Action::Close],
            Zone::Open => vec![Action::Skins(SkinsAction::OpenFile)],
            Zone::Tab(i) => {
                let tab = if i == 0 { SkinsTab::Museum } else { SkinsTab::Mine };
                if tab == self.tab {
                    return Vec::new();
                }
                self.set_tab(tab);
                vec![Action::Skins(SkinsAction::Tab(tab))]
            }
            Zone::Row(i) => vec![Action::Skins(SkinsAction::Apply(i))],
            _ => Vec::new(),
        }
    }

    // --- Rendu ----------------------------------------------------------------------------------

    pub fn render(&mut self, fb: &mut Image, skin: &Skin, v: &SkinsView) {
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
        for i in 0..2 {
            let x = 13 + 50 * i as i32;
            let on = (i == 0) == (self.tab == SkinsTab::Museum) || down(Zone::Tab(i));
            p.sprite(BUTTON[on as usize], x, 22);
            self.button_text(x, 22, v.tabs[i], on, c.button_text());
        }
        // Champ de recherche (musée).
        if self.tab == SkinsTab::Museum {
            let f = self.field_rect();
            p.fill(f.x, f.y, f.w, f.h, c.divider());
            p.fill(f.x + 1, f.y + 1, f.w - 2, f.h - 2, c.item_bg());
            let shown = if self.query.is_empty() { v.prompt.to_string() } else { self.query.clone() };
            let color = if self.query.is_empty() { mix(c.item_fg(), c.item_bg()) } else { c.item_fg() };
            let caret_text: String = self.query.chars().take(self.caret).collect();
            let caret_x = f.x as f32 + 3.0 + text::measure(&caret_text, FONT);
            let shift = (caret_x - (f.x + f.w - 4) as f32).max(0.0);
            self.text.push(TextRun { x: f.x as f32 + 3.0 - shift, y: f.y as f32 + 1.0, text: shown, color, size: FONT, clip: Rect::new(f.x + 1, f.y + 1, f.w - 2, f.h - 2), align: Align::Left });
            if v.active && self.sel.is_none() {
                p.fill((caret_x - shift).round() as i32, f.y + 2, 1, f.h - 4, c.item_fg());
            }
        }
        // Liste.
        let l = self.list_rect();
        p.fill(l.x, l.y, l.w, l.h, c.divider());
        p.fill(l.x + 1, l.y + 1, l.w - 2, l.h - 2, c.item_bg());
        let dim = mix(c.item_fg(), c.item_bg());
        let bottom = l.y + l.h - 1;
        for (r, row) in self.rows.iter().enumerate().skip(self.scroll).take(self.visible_rows()) {
            let ry = l.y + 1 + (r - self.scroll) as i32 * ROW_H;
            let rh = ROW_H.min(bottom - ry);
            if rh <= 0 {
                break;
            }
            if self.sel == Some(r) {
                p.fill(l.x + 1, ry, l.w - 2, rh, c.playlist_selection());
            }
            let (tx, ty) = (l.x + 3, ry + 2);
            match &row.thumb {
                Some(t) => p.fb.blit(t, 0, 0, t.width as i32, (t.height as i32).min(bottom - ty), tx, ty),
                None => p.fill(tx, ty, 137, 58.min(bottom - ty), mix(c.item_bg(), 0xFF00_0000)),
            }
            let text_x = tx + 137 + 6;
            let text_w = (l.x + l.w - 4 - text_x).max(10);
            let clip = Rect::new(text_x, ry, text_w, rh);
            self.text.push(TextRun { x: text_x as f32, y: (ry + 8) as f32, text: text::ellipsize(&row.name, FONT, text_w as f32), color: c.item_fg(), size: FONT, clip, align: Align::Left });
            if !row.detail.is_empty() {
                self.text.push(TextRun { x: text_x as f32, y: (ry + 22) as f32, text: text::ellipsize(&row.detail, FONT, text_w as f32), color: dim, size: FONT, clip, align: Align::Left });
            }
        }
        // Bas : état, Ouvrir…, Fermer.
        let by = self.button_y();
        let with_open = !v.open.is_empty();
        let lw = w - 13 - 47 * if with_open { 2 } else { 1 } - 8 - 13;
        if lw > 10 && !v.status.is_empty() {
            self.text.push(TextRun {
                x: 13.0,
                y: by as f32 + 2.0,
                text: text::ellipsize(v.status, FONT, lw as f32),
                color: c.window_text(),
                size: FONT,
                clip: Rect::new(13, by, lw, 15),
                align: Align::Left,
            });
        }
        if with_open {
            let ox = w - 13 - 47 * 2 - 4;
            p.sprite(BUTTON[down(Zone::Open) as usize], ox, by);
            self.button_text(ox, by, v.open, down(Zone::Open), c.button_text());
        }
        let cx = w - 13 - 47;
        p.sprite(BUTTON[down(Zone::CloseButton) as usize], cx, by);
        self.button_text(cx, by, v.close, down(Zone::CloseButton), c.button_text());
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

    fn rows(n: usize) -> Vec<SkinRow> {
        (0..n).map(|i| SkinRow { key: format!("k{i}"), name: format!("Skin {i}"), detail: String::new(), thumb: None }).collect()
    }

    #[test]
    fn search_apply_tabs_and_paging() {
        let mut s = SkinsWindow::default();
        s.key(GenKey::Text("zelda".into()));
        assert_eq!(s.key(GenKey::Enter), vec![Action::Skins(SkinsAction::Search("zelda".into()))]);
        s.set_rows(rows(10));
        assert!(s.visible_without_thumb().contains(&0));
        let l = s.list_rect();
        s.mouse_down(40, l.y + ROW_H + 10, true);
        assert_eq!(s.mouse_up(40, l.y + ROW_H + 10, true), vec![Action::Skins(SkinsAction::Apply(1))]);
        // Fin de la liste à l'écran : une seule demande de suite par longueur.
        s.scroll_by(100);
        assert!(s.wants_more());
        assert!(!s.wants_more());
        s.append_rows(rows(5));
        s.scroll_by(100);
        assert!(s.wants_more());
        s.set_thumb("k1", Image::new(137, 58, 0xFF12_3456));
        assert!(s.rows()[1].thumb.is_some());
        // Onglet « Mes skins ».
        s.mouse_down(13 + 50 + 5, 25, true);
        assert_eq!(s.mouse_up(13 + 50 + 5, 25, true), vec![Action::Skins(SkinsAction::Tab(SkinsTab::Mine))]);
        assert!(s.rows().is_empty());
        let skin = Skin::default_skin();
        let mut fb = Image::new(1, 1, 0);
        s.set_rows(rows(3));
        s.set_thumb("k0", Image::new(137, 58, 0xFF12_3456));
        let v = SkinsView { active: true, title: "Skins", tabs: ["Musée", "Mes skins"], prompt: "Chercher", open: "Ouvrir…", close: "Fermer", status: "skins.webamp.org" };
        s.render(&mut fb, &skin, &v);
        assert_eq!(fb.pixels[((s.list_rect().y + 3) * 275 + 16) as usize], 0xFF12_3456);
    }
}
