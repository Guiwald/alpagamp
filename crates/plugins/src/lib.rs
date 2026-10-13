//! Plugins WebAssembly d'Alpagamp (API v1, ADR 0020) : catalogue (plugins livrés + dossier de l'utilisateur),
//! activation selon les réglages, effets audio branchés sur le moteur, visualisation, plugins généraux.

pub mod analysis;
pub mod host;
pub mod manifest;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use alpagamp_audio::{Effect, Engine};
use alpagamp_core::settings::PluginSettings;
use parking_lot::Mutex;

pub use analysis::Analyzer;
pub use host::{Instance, Link, Request, Sound};
pub use manifest::{Kind, Manifest, OptionDef, OptionValue, Text};

/// Plugins livrés avec le lecteur (construits par `cargo xtask plugins` depuis `plugin-examples/`).
pub const BUILTIN: &[(&str, &[u8])] = &[
    ("barres-plein-ecran", include_bytes!("../../../assets/plugins/barres-plein-ecran.wasm")),
    ("enhancer", include_bytes!("../../../assets/plugins/enhancer.wasm")),
    ("minuterie-de-sommeil", include_bytes!("../../../assets/plugins/minuterie-de-sommeil.wasm")),
];

/// Nombre maximal de fichiers lus dans le dossier des plugins.
const MAX_USER_PLUGINS: usize = 64;

#[derive(Debug, Clone)]
pub enum Source {
    Builtin(&'static [u8]),
    File(PathBuf),
}

impl Source {
    fn bytes(&self) -> Result<std::borrow::Cow<'static, [u8]>, String> {
        match self {
            Source::Builtin(b) => Ok((*b).into()),
            Source::File(p) => {
                let len = std::fs::metadata(p).map_err(|e| e.to_string())?.len();
                if len > host::MAX_FILE as u64 {
                    return Err("fichier trop gros".into());
                }
                std::fs::read(p).map(Into::into).map_err(|e| e.to_string())
            }
        }
    }
}

/// Un plugin disponible.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Identifiant : nom du fichier sans `.wasm`.
    pub id: String,
    pub source: Source,
    pub manifest: Manifest,
}

impl Entry {
    pub fn builtin(&self) -> bool {
        matches!(self.source, Source::Builtin(_))
    }

    pub fn name(&self) -> &str {
        self.manifest.name.get()
    }
}

struct Running {
    id: String,
    inst: Instance,
    started: Instant,
}

struct DspEffect(Arc<Mutex<Instance>>);

impl Effect for DspEffect {
    fn process(&mut self, samples: &mut [f32], rate: u32) -> bool {
        self.0.lock().dsp(samples, rate)
    }
}

/// Identifiant tiré d'un nom de fichier : minuscules, lettres, chiffres et tirets.
pub fn id_from_name(name: &str) -> String {
    let stem = name.strip_suffix(".wasm").unwrap_or(name);
    let mut id = String::new();
    for c in stem.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            id.push(c);
        } else if !id.ends_with('-') {
            id.push('-');
        }
    }
    let id = id.trim_matches('-').to_string();
    if id.is_empty() { "plugin".into() } else { id }
}

/// Taille de l'image demandée à une visualisation pour une zone de `w × h` pixels : au plus ~320 × 240, les
/// proportions gardées (l'image est ensuite agrandie à l'écran).
pub fn vis_size(w: u32, h: u32) -> (u32, u32) {
    let (w, h) = (w.max(1) as f64, h.max(1) as f64);
    let s = (76_800.0 / (w * h)).sqrt().min(1.0);
    (((w * s).round() as u32).max(16), ((h * s).round() as u32).max(16))
}

pub struct Plugins {
    pub link: Arc<Link>,
    dir: Option<PathBuf>,
    /// Plugins disponibles, triés par nom.
    pub catalog: Vec<Entry>,
    /// Fichiers du dossier illisibles : (fichier, raison).
    pub broken: Vec<(String, String)>,
    generals: Vec<Running>,
    dsp: Vec<(String, Arc<Mutex<Instance>>)>,
    vis: Option<Running>,
    analyzer: Analyzer,
    last_tick: Option<Instant>,
    /// Plugins arrêtés après une erreur : (identifiant, message), à signaler une fois.
    failures: Vec<(String, String)>,
}

impl Default for Plugins {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugins {
    /// Catalogue des plugins livrés (sans dossier de l'utilisateur).
    pub fn new() -> Plugins {
        let mut p = Plugins {
            link: Arc::default(),
            dir: None,
            catalog: Vec::new(),
            broken: Vec::new(),
            generals: Vec::new(),
            dsp: Vec::new(),
            vis: None,
            analyzer: Analyzer::default(),
            last_tick: None,
            failures: Vec::new(),
        };
        p.rescan();
        p
    }

    /// Dossier des plugins de l'utilisateur ; relit le catalogue.
    pub fn set_dir(&mut self, dir: PathBuf) {
        self.dir = Some(dir);
        self.rescan();
    }

    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    fn probe(&self, id: &str, source: &Source) -> Result<Manifest, String> {
        let bytes = source.bytes()?;
        let mut inst = Instance::load(&bytes, Arc::new(Link::default()), id)?;
        let m = inst.manifest.clone();
        // Simple lecture de la description : pas d'`alp_quit`.
        inst.forget();
        Ok(m)
    }

    /// Relit le catalogue (plugins livrés, puis dossier : un fichier du même nom remplace un plugin livré).
    pub fn rescan(&mut self) {
        let mut sources: Vec<(String, Source)> = BUILTIN.iter().map(|(id, b)| (id.to_string(), Source::Builtin(b))).collect();
        self.broken.clear();
        if let Some(dir) = &self.dir {
            let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
                .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("wasm"))).collect())
                .unwrap_or_default();
            files.sort();
            for p in files.into_iter().take(MAX_USER_PLUGINS) {
                let id = id_from_name(&p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
                sources.retain(|(i, _)| *i != id);
                sources.push((id, Source::File(p)));
            }
        }
        let mut catalog = Vec::new();
        for (id, source) in sources {
            match self.probe(&id, &source) {
                Ok(manifest) => catalog.push(Entry { id, source, manifest }),
                Err(e) => {
                    log::warn!("plugin {id} illisible : {e}");
                    let file = match &source {
                        Source::File(p) => p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(id),
                        Source::Builtin(_) => id,
                    };
                    self.broken.push((file, e));
                }
            }
        }
        catalog.sort_by(|a, b| a.name().to_lowercase().cmp(&b.name().to_lowercase()));
        self.catalog = catalog;
    }

    pub fn entry(&self, id: &str) -> Option<&Entry> {
        self.catalog.iter().find(|e| e.id == id)
    }

    /// Valeur d'une option : celle des réglages si elle est permise, sinon la valeur par défaut.
    pub fn option_value(settings: &PluginSettings, id: &str, o: &OptionDef) -> i32 {
        settings.options.get(id).and_then(|m| m.get(&o.id)).copied().filter(|v| o.values.iter().any(|x| x.value == *v)).unwrap_or_else(|| o.default_value())
    }

    fn start(&mut self, id: &str, settings: &PluginSettings) -> Option<Instance> {
        let entry = self.entry(id)?.clone();
        let started = entry.source.bytes().and_then(|b| Instance::load(&b, self.link.clone(), id));
        match started {
            Ok(mut inst) => {
                for (i, o) in entry.manifest.options.iter().enumerate() {
                    inst.set_option(i, Self::option_value(settings, id, o));
                }
                match inst.failed() {
                    Some(e) => {
                        self.failures.push((id.to_string(), e.to_string()));
                        None
                    }
                    None => Some(inst),
                }
            }
            Err(e) => {
                self.failures.push((id.to_string(), e));
                None
            }
        }
    }

    /// Démarre et arrête les plugins pour suivre les réglages ; branche les effets audio sur le moteur.
    pub fn apply(&mut self, settings: &PluginSettings, engine: &Engine) {
        let wanted = |kind: Kind| -> Vec<String> {
            self.catalog.iter().filter(|e| e.manifest.kind == kind && settings.enabled.contains(&e.id)).map(|e| e.id.clone()).collect()
        };
        let (want_gen, want_dsp) = (wanted(Kind::General), wanted(Kind::Dsp));
        // Plugins généraux.
        self.generals.retain(|r| want_gen.contains(&r.id));
        for id in want_gen {
            if !self.generals.iter().any(|r| r.id == id) {
                if let Some(inst) = self.start(&id, settings) {
                    self.generals.push(Running { id, inst, started: Instant::now() });
                }
            }
        }
        // Effets audio (dans l'ordre du catalogue).
        let before: Vec<String> = self.dsp.iter().map(|(id, _)| id.clone()).collect();
        self.dsp.retain(|(id, _)| want_dsp.contains(id));
        for id in &want_dsp {
            if !self.dsp.iter().any(|(i, _)| i == id) {
                if let Some(inst) = self.start(id, settings) {
                    self.dsp.push((id.clone(), Arc::new(Mutex::new(inst))));
                }
            }
        }
        self.dsp.sort_by_key(|(id, _)| want_dsp.iter().position(|w| w == id));
        let after: Vec<String> = self.dsp.iter().map(|(id, _)| id.clone()).collect();
        if before != after || engine.effect_count() != after.len() {
            engine.set_effects(self.dsp.iter().map(|(_, i)| Box::new(DspEffect(i.clone())) as Box<dyn Effect>).collect());
        }
        // Visualisation.
        let want_vis = settings.vis.clone().filter(|id| self.entry(id).is_some_and(|e| e.manifest.kind == Kind::Vis));
        if self.vis.as_ref().map(|r| &r.id) != want_vis.as_ref() {
            self.vis = None;
            if let Some(id) = want_vis {
                if let Some(inst) = self.start(&id, settings) {
                    self.vis = Some(Running { id, inst, started: Instant::now() });
                }
            }
        }
    }

    /// Arrête tous les plugins (fermeture du lecteur) : ils reçoivent `alp_quit`.
    pub fn stop_all(&mut self, engine: &Engine) {
        engine.set_effects(Vec::new());
        self.generals.clear();
        self.dsp.clear();
        self.vis = None;
    }

    /// Le plugin `id` tourne.
    pub fn is_running(&self, id: &str) -> bool {
        self.generals.iter().any(|r| r.id == id) || self.dsp.iter().any(|(i, _)| i == id) || self.vis.as_ref().is_some_and(|r| r.id == id)
    }

    /// Change une option d'un plugin qui tourne.
    pub fn set_option(&mut self, id: &str, index: usize, value: i32) {
        if let Some(r) = self.generals.iter_mut().find(|r| r.id == id) {
            r.inst.set_option(index, value);
        } else if let Some((_, i)) = self.dsp.iter().find(|(i, _)| i == id) {
            i.lock().set_option(index, value);
        } else if let Some(r) = self.vis.as_mut().filter(|r| r.id == id) {
            r.inst.set_option(index, value);
        }
    }

    /// Couleurs du visualiseur du skin (`0xRRGGBB` ou `0xAARRGGBB`).
    pub fn set_colors(&self, colors: &[u32; 24]) {
        *self.link.colors.lock() = *colors;
    }

    /// À appeler souvent : état du lecteur, tics des plugins généraux (dix par seconde), plugins en échec.
    pub fn tick(&mut self, state: u8, volume: u8) {
        self.link.state.store(state, Ordering::Relaxed);
        self.link.volume.store(volume, Ordering::Relaxed);
        let now = Instant::now();
        if self.last_tick.is_none_or(|t| now.duration_since(t).as_millis() >= 100) {
            self.last_tick = Some(now);
            for r in &mut self.generals {
                let ms = now.duration_since(r.started).as_millis() as i64;
                r.inst.tick(ms);
            }
        }
        let mut failed = Vec::new();
        self.generals.retain(|r| match r.inst.failed() {
            Some(e) => {
                failed.push((r.id.clone(), e.to_string()));
                false
            }
            None => true,
        });
        self.dsp.retain(|(id, i)| match i.lock().failed() {
            Some(e) => {
                failed.push((id.clone(), e.to_string()));
                false
            }
            None => true,
        });
        self.failures.extend(failed);
    }

    /// Une visualisation de plugin est choisie et tourne.
    pub fn vis_running(&self) -> bool {
        self.vis.is_some()
    }

    /// Nom de la visualisation de plugin en cours.
    pub fn vis_name(&self) -> Option<&str> {
        self.vis.as_ref().map(|r| r.inst.manifest.name.get())
    }

    /// Calcule une image de la visualisation de plugin (`w × h`), avec le son joué par `engine`.
    pub fn vis_frame(&mut self, w: u32, h: u32, engine: &Engine, out: &mut Vec<u32>) -> bool {
        let Some(r) = self.vis.as_mut() else { return false };
        self.analyzer.update(|l, rr| engine.vis_latest(l, rr));
        let ms = r.started.elapsed().as_millis() as u32;
        if r.inst.vis(w, h, ms, &self.analyzer.sound, out) {
            return true;
        }
        let e = r.inst.failed().unwrap_or("?").to_string();
        let id = r.id.clone();
        self.vis = None;
        self.failures.push((id, e));
        false
    }

    /// Plugins arrêtés après une erreur depuis le dernier appel : (identifiant, message).
    pub fn take_failures(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.failures)
    }

    /// Demandes des plugins au lecteur depuis le dernier appel.
    pub fn take_requests(&self) -> Vec<Request> {
        self.link.take_requests()
    }

    /// Installe un plugin (copie dans le dossier après vérification) ; renvoie son identifiant.
    pub fn install(&mut self, file: &Path) -> Result<String, String> {
        let dir = self.dir.clone().ok_or("pas de dossier de plugins")?;
        let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let id = id_from_name(&name);
        let source = Source::File(file.to_path_buf());
        self.probe(&id, &source)?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let dest = dir.join(format!("{id}.wasm"));
        if dest != file {
            std::fs::copy(file, &dest).map_err(|e| e.to_string())?;
        }
        self.rescan();
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_sizes() {
        assert_eq!(id_from_name("Mon Plugin (v2).wasm"), "mon-plugin-v2");
        assert_eq!(id_from_name("..wasm"), "plugin");
        assert_eq!(vis_size(256, 80), (256, 80));
        let (w, h) = vis_size(1920, 1080);
        assert!(w * h <= 77_000 && (w as f32 / h as f32 - 16.0 / 9.0).abs() < 0.05, "{w}×{h}");
    }
}
