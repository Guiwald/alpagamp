//! Les plugins WebAssembly vus du lecteur : activation selon les réglages, demandes des plugins (commandes,
//! volume, messages), plugins en échec désactivés.

use std::path::{Path, PathBuf};

use alpagamp_audio::PlayState;
use alpagamp_core::i18n::t;
use alpagamp_plugins::{Kind, Request};

use crate::Player;

impl Player {
    /// Dossier des plugins de l'utilisateur ; démarre les plugins activés.
    pub fn init_plugins(&mut self, dir: PathBuf) {
        self.plugins.set_dir(dir);
        // Les plugins qui ne se souviennent pas (minuterie) restent éteints au démarrage.
        let plugins = &self.plugins;
        self.settings.plugins.enabled.retain(|id| plugins.entry(id).is_none_or(|e| e.manifest.remember));
        self.apply_plugins();
    }

    /// Démarre et arrête les plugins selon les réglages.
    pub fn apply_plugins(&mut self) {
        self.plugins.apply(&self.settings.plugins, &self.engine);
        self.handle_plugin_failures();
    }

    /// Couleurs du visualiseur du skin, pour les plugins.
    pub fn set_plugin_colors(&self, colors: &[u32; 24]) {
        self.plugins.set_colors(colors);
    }

    /// Le plugin `id` est activé dans les réglages (pour une visualisation : affichée).
    pub fn plugin_enabled(&self, id: &str) -> bool {
        self.settings.plugins.is_enabled(id)
    }

    /// Active ou désactive un plugin. Une visualisation remplace la précédente (ou MilkDrop).
    pub fn set_plugin_enabled(&mut self, id: &str, on: bool) {
        let Some(kind) = self.plugins.entry(id).map(|e| e.manifest.kind) else { return };
        let s = &mut self.settings.plugins;
        match kind {
            Kind::Vis => {
                if on {
                    s.vis = Some(id.to_string());
                } else if s.vis.as_deref() == Some(id) {
                    s.vis = None;
                }
            }
            _ => {
                s.enabled.retain(|e| e != id);
                if on {
                    s.enabled.push(id.to_string());
                }
            }
        }
        self.apply_plugins();
    }

    /// Valeur actuelle de l'option n° `index` du plugin `id`.
    pub fn plugin_option(&self, id: &str, index: usize) -> Option<i32> {
        let o = self.plugins.entry(id)?.manifest.options.get(index)?;
        Some(alpagamp_plugins::Plugins::option_value(&self.settings.plugins, id, o))
    }

    /// Règle une option (gardée dans les réglages, appliquée tout de suite si le plugin tourne).
    pub fn set_plugin_option(&mut self, id: &str, index: usize, value: i32) {
        let Some(o) = self.plugins.entry(id).and_then(|e| e.manifest.options.get(index)).cloned() else { return };
        if !o.values.iter().any(|v| v.value == value) {
            return;
        }
        self.settings.plugins.options.entry(id.to_string()).or_default().insert(o.id.clone(), value);
        self.plugins.set_option(id, index, value);
        self.handle_plugin_failures();
    }

    /// Relit le dossier des plugins.
    pub fn rescan_plugins(&mut self) {
        self.plugins.rescan();
        self.apply_plugins();
    }

    /// Installe un fichier `.wasm` dans le dossier des plugins et l'active ; renvoie son identifiant.
    pub fn install_plugin(&mut self, file: &Path) -> Result<String, String> {
        let id = self.plugins.install(file)?;
        // Un plugin du même nom qui tournait est relancé avec le nouveau fichier.
        let was_on = self.plugin_enabled(&id);
        if was_on {
            self.set_plugin_enabled(&id, false);
        }
        self.set_plugin_enabled(&id, true);
        if let Some(e) = self.plugins.entry(&id) {
            let msg = format!("{} : {}", e.name(), t("plugins.installed"));
            self.notify(msg);
        }
        Ok(id)
    }

    /// Arrête tous les plugins (fermeture) ; leurs dernières demandes (volume rendu) sont appliquées.
    pub fn shutdown_plugins(&mut self) {
        self.plugins.stop_all(&self.engine);
        self.handle_plugin_requests();
    }

    pub(crate) fn tick_plugins(&mut self) {
        let state = match self.engine.status().state {
            PlayState::Stopped => 0,
            PlayState::Playing => 1,
            PlayState::Paused => 2,
        };
        self.plugins.tick(state, self.settings.volume);
        self.handle_plugin_requests();
        self.handle_plugin_failures();
    }

    fn handle_plugin_requests(&mut self) {
        for r in self.plugins.take_requests() {
            match r {
                Request::Command(1) => self.play_pressed(),
                Request::Command(2) => {
                    if self.state() == PlayState::Playing {
                        self.pause_pressed();
                    }
                }
                Request::Command(3) => self.stop(),
                Request::Command(4) => self.next(),
                Request::Command(5) => self.previous(),
                Request::Command(_) => {}
                Request::Volume(v) => self.set_volume(v),
                Request::Notify(m) => self.notify(m),
            }
        }
    }

    /// Un plugin en échec est désactivé (pour ne pas recommencer au prochain démarrage) et signalé.
    fn handle_plugin_failures(&mut self) {
        for (id, err) in self.plugins.take_failures() {
            let name = self.plugins.entry(&id).map(|e| e.name().to_string()).unwrap_or_else(|| id.clone());
            let s = &mut self.settings.plugins;
            s.enabled.retain(|e| *e != id);
            if s.vis.as_deref() == Some(id.as_str()) {
                s.vis = None;
            }
            self.error(format!("{name} {} : {err}", t("plugins.stopped")));
        }
    }
}
