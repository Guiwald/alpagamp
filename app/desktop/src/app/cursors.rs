//! Curseurs de souris du skin (`normal.cur`, `close.cur`…), choisis selon la zone survolée (ADR 0016).

use std::collections::HashMap;

use winit::event_loop::ActiveEventLoop;
use winit::window::{CursorIcon, CustomCursor};

use super::{App, Kind};

#[derive(Default)]
pub(super) struct Cursors {
    /// Curseurs déjà créés pour le skin courant.
    made: HashMap<&'static str, Option<CustomCursor>>,
    /// Curseur affiché par fenêtre (`None` : celui du système).
    shown: HashMap<usize, Option<&'static str>>,
}

impl Cursors {
    /// À appeler quand le skin change.
    pub fn reset(&mut self) {
        self.made.clear();
        self.shown.clear();
    }
}

impl App {
    /// Met le curseur du skin qui correspond à la position de la souris dans la fenêtre `k`.
    pub(super) fn update_cursor(&mut self, k: Kind, event_loop: &ActiveEventLoop) {
        let Some(w) = self.slot(k).win.as_ref() else { return };
        let (cx, cy) = self.slot(k).cursor;
        let (x, y) = w.to_skin(cx, cy);
        let name: &'static str = match k {
            Kind::Main => self.main_ui.cursor_at(x, y),
            Kind::Eq => self.eq_ui.cursor_at(x, y),
            Kind::Playlist => self.pl_ui.cursor_at(x, y),
            _ => "normal.cur",
        };
        let want = self.skin.cursors.contains_key(name).then_some(name);
        if self.cursors.shown.get(&(k as usize)) == Some(&want) {
            return;
        }
        self.cursors.shown.insert(k as usize, want);
        let custom = want.and_then(|n| {
            self.cursors
                .made
                .entry(n)
                .or_insert_with(|| {
                    let c = self.skin.cursors.get(n)?;
                    let src = CustomCursor::from_rgba(c.rgba(), c.width as u16, c.height as u16, c.hotspot.0 as u16, c.hotspot.1 as u16).ok()?;
                    Some(event_loop.create_custom_cursor(src))
                })
                .clone()
        });
        let Some(w) = self.slot(k).win.as_ref() else { return };
        match custom {
            Some(c) => w.window.set_cursor(c),
            None => w.window.set_cursor(CursorIcon::Default),
        }
    }
}
