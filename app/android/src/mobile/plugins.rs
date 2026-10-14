//! Le menu Plugins du téléphone (ADR 0020) : activer, options, à propos ; options de la visualisation de plugin.

use alpagamp_core::i18n::t;
use alpagamp_plugins::{Kind, Source};
use alpagamp_ui::{MenuItem, MenuWindow};

use super::{MenuKind, Mobile, Overlay};

/// Coupe un texte en lignes d'au plus `width` caractères (aux espaces).
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.lines() {
        let mut line = String::new();
        for word in para.split_whitespace() {
            if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines
}

impl Mobile {
    /// Options d'un plugin (sous-menus à choix).
    fn plugin_option_items(&self, id: &str) -> Vec<MenuItem> {
        let Some(e) = self.player.plugins.entry(id) else { return Vec::new() };
        e.manifest
            .options
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let cur = self.player.plugin_option(id, i).unwrap_or(0);
                let values = o.values.iter().map(|v| MenuItem::check(&format!("plg_opt_{i}_{}_{id}", v.value), v.label.get(), v.value == cur)).collect();
                MenuItem::sub(o.label.get(), values)
            })
            .collect()
    }

    /// Le sous-menu Plugins du menu principal.
    pub(super) fn plugins_menu(&self) -> Vec<MenuItem> {
        let mut v = Vec::new();
        for e in &self.player.plugins.catalog {
            let on = self.player.plugin_enabled(&e.id);
            let label = if e.manifest.kind == Kind::Vis { t("plugins.show") } else { t("plugins.enabled") };
            let mut items = vec![MenuItem::check(&format!("plg_on_{}", e.id), label, on)];
            items.extend(self.plugin_option_items(&e.id));
            items.push(MenuItem::new(&format!("plg_about_{}", e.id), t("plugins.about")));
            let title = if on { format!("✓ {}", e.name()) } else { e.name().to_string() };
            v.push(MenuItem::sub(&title, items));
        }
        for (f, _) in &self.player.plugins.broken {
            v.push(MenuItem::new("", &format!("⚠ {f} ({})", t("plugins.unreadable"))).disabled());
        }
        v.push(MenuItem::separator());
        v.push(MenuItem::new("plg_rescan", t("plugins.rescan")));
        v
    }

    /// Menu de la visualisation quand un plugin l'affiche : ses options, retour à MilkDrop.
    pub(super) fn plugin_vis_menu(&self) -> Option<Vec<MenuItem>> {
        let id = self.player.settings.plugins.vis.clone().filter(|_| self.player.plugins.vis_running())?;
        let mut v = self.plugin_option_items(&id);
        v.push(MenuItem::separator());
        v.push(MenuItem::new(&format!("plg_on_{id}"), t("plugins.none_vis")));
        v.push(MenuItem::separator());
        v.push(MenuItem::new("milk_exit", t("milk.exit")));
        Some(v)
    }

    fn plugin_about(&mut self, id: &str) {
        let Some(e) = self.player.plugins.entry(id) else { return };
        let m = &e.manifest;
        let kind = t(match m.kind {
            Kind::Vis => "plugins.kind_vis",
            Kind::Dsp => "plugins.kind_dsp",
            Kind::General => "plugins.kind_general",
        });
        let mut lines = vec![format!("{kind} · {}", m.version)];
        if !m.author.get().is_empty() {
            lines.push(format!("{} {}", t("plugins.by"), m.author.get()));
        }
        lines.push(String::new());
        lines.extend(wrap(m.description.get(), 40));
        lines.push(String::new());
        lines.push(match &e.source {
            Source::Builtin(_) => t("plugins.builtin").to_string(),
            Source::File(p) => p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        });
        let items = lines.iter().map(|l| MenuItem::new("", if l.is_empty() { " " } else { l })).collect();
        let name = e.name().to_string();
        self.overlay = Some(Overlay::Menu(MenuWindow::new(&name, items, 275, self.overlay_max_h()), MenuKind::Info));
    }

    /// Commandes de menu des plugins ; `false` si `id` n'en est pas une.
    pub(super) fn plugin_menu_command(&mut self, id: &str) -> bool {
        if id == "plg_rescan" {
            self.player.rescan_plugins();
            let n = self.player.plugins.catalog.len();
            self.main_ui.marquee.flash(&format!("{n} {}", t("menu.plugins").to_uppercase()));
        } else if let Some(pid) = id.strip_prefix("plg_on_") {
            let on = !self.player.plugin_enabled(pid);
            self.player.set_plugin_enabled(pid, on);
            let is_vis = self.player.plugins.entry(pid).is_some_and(|e| e.manifest.kind == Kind::Vis);
            if is_vis && on && self.player.plugins.vis_running() && !self.milk.on {
                self.milk_enter();
            }
        } else if let Some(pid) = id.strip_prefix("plg_about_") {
            let pid = pid.to_string();
            self.plugin_about(&pid);
        } else if let Some(rest) = id.strip_prefix("plg_opt_") {
            let mut it = rest.splitn(3, '_');
            let parsed = (it.next().and_then(|i| i.parse::<usize>().ok()), it.next().and_then(|v| v.parse::<i32>().ok()), it.next());
            if let (Some(i), Some(v), Some(pid)) = parsed {
                let pid = pid.to_string();
                self.player.set_plugin_option(&pid, i, v);
            }
        } else {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn wraps_text() {
        assert_eq!(super::wrap("un deux trois quatre", 9), vec!["un deux", "trois", "quatre"]);
    }
}
