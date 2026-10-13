//! Le navigateur de skins (Alt+S) : le Musée (Winamp Skin Museum) et les skins déjà rangés.

use alpagamp_core::i18n::t;
use alpagamp_museum::{Apply, Browser};
use alpagamp_ui::{SkinsAction, SkinsView, SkinsWindow};
use winit::event_loop::ActiveEventLoop;

use super::{App, Kind};
use crate::command::Command;

pub(super) fn new_browser(skins_dir: std::path::PathBuf) -> Browser {
    Browser::new(skins_dir, SkinsWindow::default())
}

impl App {
    pub(super) fn skins_toggle(&mut self, event_loop: &ActiveEventLoop) {
        let open = !self.slot(Kind::Skins).open;
        self.set_open(Kind::Skins, open, event_loop);
        if open && self.skins.is_empty() {
            let cur = self.player.settings.skin.clone();
            self.skins.load_tab(cur.as_deref());
        }
    }

    fn skins_apply(&mut self, a: Apply, event_loop: &ActiveEventLoop) {
        match a {
            Apply::Default => self.command(Command::DefaultSkin, event_loop),
            Apply::File(p) => self.load_skin(&p),
            Apply::OpenDialog => self.command(Command::LoadSkin, event_loop),
        }
        self.menus_dirty = true;
    }

    pub(super) fn skins_action(&mut self, a: SkinsAction, event_loop: &ActiveEventLoop) {
        let cur = self.player.settings.skin.clone();
        if let Some(apply) = self.skins.action(a, cur.as_deref()) {
            self.skins_apply(apply, event_loop);
        }
    }

    pub(super) fn skins_tick(&mut self, event_loop: &ActiveEventLoop) {
        let visible = self.slot(Kind::Skins).visible();
        if let Some(apply) = self.skins.tick(visible, &self.default_skin) {
            self.skins_apply(apply, event_loop);
        }
    }

    pub(super) fn skins_redraw(&mut self) {
        let status = self.skins.status();
        let active = self.focused == Some(Kind::Skins);
        let open = if super::SKINS_OPEN { t("skins.open") } else { "" };
        let view = SkinsView {
            active,
            title: t("skins.title"),
            tabs: [t("skins.museum"), t("skins.mine")],
            prompt: t("skins.search"),
            open,
            close: t("dialog.close"),
            status: &status,
        };
        let s = &mut self.slots[Kind::Skins as usize];
        self.skins.ui.render(&mut s.fb, &self.skin, &view);
        if let Some(w) = s.win.as_mut() {
            w.present(&s.fb, &self.skins.ui.text);
        }
    }
}
