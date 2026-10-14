//! Le menu Plugins (ADR 0020) : activer, options, à propos, installer, dossier.

use alpagamp_core::i18n::t;
use alpagamp_plugins::{Kind, Source};
use winit::event_loop::ActiveEventLoop;

use super::{App, Kind as Win};
use crate::command::Command;
use crate::dialogs;

impl App {
    /// Les commandes des plugins. Renvoie `false` si `cmd` n'en est pas une.
    pub(super) fn plugin_command(&mut self, cmd: &Command, event_loop: &ActiveEventLoop) -> bool {
        match cmd {
            Command::PluginToggle(id) => {
                let on = !self.player.plugin_enabled(id);
                self.player.set_plugin_enabled(id, on);
                self.after_plugin_change(id, on, event_loop);
            }
            Command::PluginOption(id, i, v) => self.player.set_plugin_option(id, *i, *v),
            Command::PluginAbout(id) => self.plugin_about(id),
            Command::PluginInstall => {
                if let Some(p) = dialogs::pick_plugin() {
                    self.install_plugin(&p, event_loop);
                }
            }
            Command::PluginsFolder => {
                if let Some(dir) = self.player.plugins.dir() {
                    dialogs::open_folder(dir);
                }
            }
            Command::PluginsRescan => {
                self.player.rescan_plugins();
                if !self.player.plugins.broken.is_empty() {
                    let list: Vec<String> = self.player.plugins.broken.iter().map(|(f, e)| format!("{f} : {e}")).collect();
                    dialogs::message(t("menu.plugins"), &format!("{}\n\n{}", t("plugins.unreadable"), list.join("\n")));
                }
            }
            _ => return false,
        }
        self.menus_dirty = true;
        true
    }

    /// Une visualisation de plugin activée ouvre la fenêtre de visualisation.
    fn after_plugin_change(&mut self, id: &str, on: bool, event_loop: &ActiveEventLoop) {
        let is_vis = self.player.plugins.entry(id).is_some_and(|e| e.manifest.kind == Kind::Vis);
        if is_vis && on && self.player.plugins.vis_running() && !self.slot(Win::Vis).open {
            self.milk_set_open(true, event_loop);
        }
        self.milk_reset_chrome();
    }

    pub(super) fn install_plugin(&mut self, path: &std::path::Path, event_loop: &ActiveEventLoop) {
        match self.player.install_plugin(path) {
            Ok(id) => self.after_plugin_change(&id, true, event_loop),
            Err(e) => dialogs::message(t("plugins.install"), &format!("{}\n\n{e}", path.display())),
        }
        self.menus_dirty = true;
    }

    fn plugin_about(&self, id: &str) {
        let Some(e) = self.player.plugins.entry(id) else { return };
        let m = &e.manifest;
        let kind = t(match m.kind {
            Kind::Vis => "plugins.kind_vis",
            Kind::Dsp => "plugins.kind_dsp",
            Kind::General => "plugins.kind_general",
        });
        let mut head = kind.to_string();
        if !m.version.is_empty() {
            head.push_str(&format!(" · {}", m.version));
        }
        let author = m.author.get();
        if !author.is_empty() {
            head.push_str(&format!(" · {} {author}", t("plugins.by")));
        }
        let origin = match &e.source {
            Source::Builtin(_) => t("plugins.builtin").to_string(),
            Source::File(p) => p.display().to_string(),
        };
        dialogs::message(e.name(), &format!("{head}\n\n{}\n\n({origin})", m.description.get()));
    }

    /// Éléments du menu Plugins.
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub(super) fn plugin_menu_items(&self) -> Vec<crate::menu::PluginMenu> {
        self.player
            .plugins
            .catalog
            .iter()
            .map(|e| crate::menu::PluginMenu {
                id: e.id.clone(),
                name: e.name().to_string(),
                kind: e.manifest.kind,
                enabled: self.player.plugin_enabled(&e.id),
                options: e
                    .manifest
                    .options
                    .iter()
                    .enumerate()
                    .map(|(i, o)| {
                        let values = o.values.iter().map(|v| (v.value, v.label.get().to_string())).collect();
                        (o.label.get().to_string(), values, self.player.plugin_option(&e.id, i).unwrap_or(0))
                    })
                    .collect(),
            })
            .collect()
    }
}
