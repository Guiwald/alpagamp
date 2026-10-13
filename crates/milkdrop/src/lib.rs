//! MilkDrop dans Alpagamp.
//!
//! - [`Visualizer`] : l'interface du moteur (ADR 0011) ; [`LacteaVis`] l'implémente avec lactea (ADR 0019).
//! - [`MilkDrop`] : le pilote commun au bureau et au téléphone — liste de presets, suivant / précédent / au hasard,
//!   verrou, changement automatique avec fondu, alimentation en échantillons, rendu dans une texture.
//! - [`gpu`] : le GPU partagé et la composition des calques dans une fenêtre.
//! - [`pack`] : le pack « Cream of the Crop », téléchargé à la demande.

pub mod gpu;
pub mod pack;
pub mod presets;

use std::collections::HashSet;

pub use gpu::{Draw, Gpu, Layer, Presenter, Rect};
pub use pack::{PackInstaller, PackState};
pub use presets::{Preset, PresetList};

/// Un moteur de visualisation qui dessine dans une texture du GPU.
pub trait Visualizer {
    /// Charge un preset ; `blend_secs` > 0 : fondu depuis le précédent.
    fn load(&mut self, bytes: &[u8], name: &str, blend_secs: f64) -> Result<(), String>;
    /// Échantillons stéréo entrelacés.
    fn push_pcm(&mut self, interleaved: &[f32]);
    fn resize(&mut self, w: u32, h: u32) -> Result<(), String>;
    fn render(&mut self, enc: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, time: f64, frame: u32);
}

/// Le moteur lactea.
pub struct LacteaVis {
    engine: lactea::Engine,
}

impl LacteaVis {
    pub fn new(gpu: &Gpu, w: u32, h: u32, sample_rate: u32) -> Result<LacteaVis, String> {
        let cfg = lactea::EngineConfig {
            width: w.max(1),
            height: h.max(1),
            auto_compat: true,
            sample_rate,
            seed: fastrand::u64(..),
            output_format: TARGET_FORMAT,
            ..Default::default()
        };
        let engine = lactea::Engine::new(&gpu.device, &gpu.queue, cfg).map_err(|e| e.to_string())?;
        Ok(LacteaVis { engine })
    }
}

impl Visualizer for LacteaVis {
    fn load(&mut self, bytes: &[u8], name: &str, blend_secs: f64) -> Result<(), String> {
        let t = if blend_secs > 0.0 { lactea::Transition::Soft { seconds: blend_secs } } else { lactea::Transition::Cut };
        let report = self.engine.load_preset(bytes, name, t).map_err(|e| e.to_string())?;
        for f in &report.shader_fallbacks {
            log::debug!("{name} : shader {:?} remplacé ({})", f.pass, f.reason.lines().next().unwrap_or(""));
        }
        Ok(())
    }

    fn push_pcm(&mut self, interleaved: &[f32]) {
        self.engine.push_pcm_f32(interleaved, 2);
    }

    fn resize(&mut self, w: u32, h: u32) -> Result<(), String> {
        self.engine.resize(w.max(1), h.max(1)).map_err(|e| e.to_string())
    }

    fn render(&mut self, enc: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, time: f64, frame: u32) {
        self.engine.render(enc, target, lactea::FrameClock { time, frame });
    }
}

/// Format de la texture de la visualisation.
pub const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Pilote de MilkDrop.
pub struct MilkDrop {
    vis: Box<dyn Visualizer>,
    target: wgpu::Texture,
    view: wgpu::TextureView,
    layer: Layer,
    pub presets: PresetList,
    current: Option<usize>,
    /// Presets vus (pour « précédent »).
    history: Vec<usize>,
    /// Position dans l'historique quand on recule (0 = au bout).
    back: usize,
    pub locked: bool,
    pub shuffle: bool,
    /// Secondes entre deux changements automatiques (0 : jamais).
    pub auto_secs: f64,
    /// Durée du fondu entre deux presets.
    pub blend_secs: f64,
    changed_at: f64,
    next_change: f64,
    start: std::time::Instant,
    frame: u32,
    failed: HashSet<usize>,
    message: Option<(String, f64)>,
}

impl MilkDrop {
    pub fn new(gpu: &Gpu, w: u32, h: u32, sample_rate: u32, presets: PresetList) -> Result<MilkDrop, String> {
        let vis = Box::new(LacteaVis::new(gpu, w, h, sample_rate)?);
        let (target, view, layer) = make_target(gpu, w, h);
        Ok(MilkDrop {
            vis,
            target,
            view,
            layer,
            presets,
            current: None,
            history: Vec::new(),
            back: 0,
            locked: false,
            shuffle: true,
            auto_secs: 20.0,
            blend_secs: 2.7,
            changed_at: 0.0,
            next_change: 20.0,
            start: std::time::Instant::now(),
            frame: 0,
            failed: HashSet::new(),
            message: None,
        })
    }

    fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// Taille de rendu (pixels).
    pub fn size(&self) -> (u32, u32) {
        (self.target.width(), self.target.height())
    }

    pub fn resize(&mut self, gpu: &Gpu, w: u32, h: u32) {
        let (w, h) = (w.max(16), h.max(16));
        if (w, h) == self.size() {
            return;
        }
        match self.vis.resize(w, h) {
            Ok(()) => {
                let (t, v, l) = make_target(gpu, w, h);
                self.target = t;
                self.view = v;
                self.layer = l;
            }
            Err(e) => log::warn!("MilkDrop : taille {w}×{h} refusée : {e}"),
        }
    }

    pub fn feed(&mut self, interleaved: &[f32]) {
        if !interleaved.is_empty() {
            self.vis.push_pcm(interleaved);
        }
    }

    /// Change de preset si c'est l'heure, puis enregistre le rendu de l'image dans `enc`.
    pub fn render(&mut self, enc: &mut wgpu::CommandEncoder) {
        let now = self.now();
        if self.current.is_none() {
            self.next(true);
        } else if !self.locked && self.auto_secs > 0.0 && now >= self.next_change {
            self.next(false);
        }
        self.frame = self.frame.wrapping_add(1);
        self.vis.render(enc, &self.view, now, self.frame);
    }

    /// Le calque à composer (la texture rendue).
    pub fn layer(&self) -> &Layer {
        &self.layer
    }

    pub fn current(&self) -> Option<&Preset> {
        self.current.and_then(|i| self.presets.get(i))
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current
    }

    /// Message à afficher (nom du preset qui vient d'arriver, verrou…) pendant quelques secondes.
    pub fn message(&self) -> Option<&str> {
        match &self.message {
            Some((m, t)) if self.now() - t < 3.5 => Some(m.as_str()),
            _ => None,
        }
    }

    pub fn show_message(&mut self, m: impl Into<String>) {
        self.message = Some((m.into(), self.now()));
    }

    /// Change les presets (après un nouveau scan) en gardant le preset courant si possible.
    pub fn set_presets(&mut self, presets: PresetList) {
        let key = self.current().map(Preset::key);
        self.presets = presets;
        self.current = key.and_then(|k| self.presets.find_key(&k));
        self.history = self.current.into_iter().collect();
        self.back = 0;
        self.failed.clear();
    }

    /// Suivant (au hasard si `shuffle`). `hard` : sans fondu.
    pub fn next(&mut self, hard: bool) {
        if self.back > 0 {
            self.back -= 1;
            let i = self.history[self.history.len() - 1 - self.back];
            self.load(i, hard, false);
            return;
        }
        for _ in 0..12 {
            let Some(i) = self.pick_next() else { return };
            if self.load(i, hard, true) {
                return;
            }
        }
    }

    fn pick_next(&self) -> Option<usize> {
        let n = self.presets.len();
        if n == 0 || self.failed.len() >= n {
            return None;
        }
        if self.shuffle && n > 1 {
            for _ in 0..64 {
                let i = fastrand::usize(..n);
                if Some(i) != self.current && !self.failed.contains(&i) {
                    return Some(i);
                }
            }
        }
        let mut i = self.current.map(|c| (c + 1) % n).unwrap_or(0);
        for _ in 0..n {
            if !self.failed.contains(&i) {
                return Some(i);
            }
            i = (i + 1) % n;
        }
        None
    }

    /// Précédent : remonte l'historique (ou le preset d'avant dans l'ordre).
    pub fn previous(&mut self, hard: bool) {
        if self.back + 1 < self.history.len() {
            self.back += 1;
            let i = self.history[self.history.len() - 1 - self.back];
            self.load(i, hard, false);
        } else if let Some(c) = self.current {
            let n = self.presets.len();
            let i = (c + n - 1) % n.max(1);
            self.load(i, hard, true);
        }
    }

    /// Choisit un preset précis.
    pub fn select(&mut self, i: usize, hard: bool) -> bool {
        self.back = 0;
        self.load(i, hard, true)
    }

    pub fn select_key(&mut self, key: &str) -> bool {
        match self.presets.find_key(key) {
            Some(i) => self.select(i, true),
            None => false,
        }
    }

    pub fn toggle_lock(&mut self) -> bool {
        self.locked = !self.locked;
        self.locked
    }

    fn load(&mut self, i: usize, hard: bool, record: bool) -> bool {
        let Some(p) = self.presets.get(i).cloned() else { return false };
        let Some(bytes) = p.bytes() else {
            self.failed.insert(i);
            return false;
        };
        let blend = if hard || self.current.is_none() { 0.0 } else { self.blend_secs };
        match self.vis.load(&bytes, &p.name, blend) {
            Ok(()) => {
                self.current = Some(i);
                if record {
                    self.back = 0;
                    self.history.push(i);
                    if self.history.len() > 500 {
                        self.history.remove(0);
                    }
                }
                let now = self.now();
                self.changed_at = now;
                // Comme MilkDrop : un peu de hasard dans la durée de chaque preset.
                self.next_change = now + self.auto_secs * (0.8 + 0.4 * fastrand::f64());
                self.show_message(p.name.clone());
                true
            }
            Err(e) => {
                log::info!("preset refusé ({}) : {e}", p.key());
                self.failed.insert(i);
                false
            }
        }
    }

    /// Copie de l'image rendue (RGBA), pour les tests et les captures.
    pub fn snapshot(&self, gpu: &Gpu) -> (u32, u32, Vec<u8>) {
        let (w, h) = self.size();
        (w, h, gpu::read_rgba(gpu, &self.target))
    }

    /// Secondes depuis le dernier changement.
    pub fn since_change(&self) -> f64 {
        self.now() - self.changed_at
    }
}

fn make_target(gpu: &Gpu, w: u32, h: u32) -> (wgpu::Texture, wgpu::TextureView, Layer) {
    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("milkdrop"),
        size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TARGET_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let layer = Layer::from_view(gpu, &target, true);
    (target, view, layer)
}

/// Taille de rendu pour une zone d'affichage : au plus `max_dim` pixels de côté (le GPU du téléphone n'a pas à
/// calculer 2 400 lignes).
pub fn render_size(w: u32, h: u32, max_dim: u32) -> (u32, u32) {
    let m = w.max(h).max(1);
    if m <= max_dim {
        return (w.max(1), h.max(1));
    }
    let s = max_dim as f64 / m as f64;
    (((w as f64 * s).round() as u32).max(1), ((h as f64 * s).round() as u32).max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_size_caps_longest_side() {
        assert_eq!(render_size(640, 480, 1280), (640, 480));
        assert_eq!(render_size(1080, 2400, 1200), (540, 1200));
    }
}
