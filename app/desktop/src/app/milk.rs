//! La fenêtre de visualisation (presets MilkDrop) : GPU, composition du cadre de skin et de l'image, plein écran,
//! commandes, pack de presets.

use std::rc::Rc;

use alpagamp_core::i18n::t;
use alpagamp_milkdrop::{Draw, Gpu, Layer, MilkDrop, PackInstaller, PackState, PresetList, Presenter, Rect, gpu, pack, render_size};
use alpagamp_skin::Image;
use alpagamp_ui::milk_window::message_box;
use alpagamp_ui::paint::Painter;
use alpagamp_ui::text::Rect as SkinRect;
use alpagamp_ui::{GenKind, MilkView, MilkWindow};
use winit::event_loop::ActiveEventLoop;

use super::{App, Kind};
use crate::command::Command;
use crate::dialogs;
use crate::window::zoomed_argb;

/// Plus grand côté de l'image calculée par MilkDrop (au-delà, elle est agrandie par le GPU).
const MAX_RENDER: u32 = 1920;

type ChromeKey = (u32, u32, bool, bool, String, String, String, u32);

#[derive(Default)]
pub(super) struct MilkState {
    pub gpu: Option<Rc<Gpu>>,
    pub engine: Option<MilkDrop>,
    pub ui: MilkWindow,
    chrome: Option<(ChromeKey, Layer)>,
    note: Option<(String, u32, Layer)>,
    pcm_cursor: u64,
    pcm: Vec<f32>,
    pub pack: PackInstaller,
    pack_seen: Option<PackState>,
    /// (nom, dossier) des presets, pour « Choisir un preset ».
    pub items: Vec<(String, String)>,
    /// Position de la fenêtre avant le plein écran.
    restore_pos: Option<(i32, i32)>,
    /// Visualisation de plugin : pixels de la dernière image et calque GPU.
    plugin_px: Vec<u32>,
    plugin_layer: Option<Layer>,
}

impl MilkState {
    pub fn new(w: u32, h: u32) -> MilkState {
        MilkState { ui: MilkWindow::new(w, h), ..Default::default() }
    }

    pub fn fullscreen(app: &App) -> bool {
        app.slot(Kind::Vis).win.as_ref().is_some_and(|w| w.fullscreen)
    }
}

impl App {
    /// Ouvre ou ferme la fenêtre de visualisation (en créant le GPU et le moteur au premier besoin).
    pub(super) fn milk_set_open(&mut self, open: bool, event_loop: &ActiveEventLoop) {
        if !open {
            if MilkState::fullscreen(self) {
                self.milk_fullscreen(false);
            }
            self.set_open(Kind::Vis, false, event_loop);
            return;
        }
        self.set_open(Kind::Vis, true, event_loop);
        if let Err(e) = self.milk_attach() {
            log::warn!("visualisation : {e}");
            self.set_open(Kind::Vis, false, event_loop);
            dialogs::message(t("milk.title"), &format!("{}\n\n{e}", t("milk.no_gpu")));
        }
    }

    /// Crée le GPU, la surface de la fenêtre et le moteur s'ils manquent.
    fn milk_attach(&mut self) -> Result<(), String> {
        let rate = self.player.engine.output_rate();
        let Some(win) = self.slots[Kind::Vis as usize].win.as_mut() else { return Err("fenêtre absente".into()) };
        if win.gpu.is_none() {
            let instance = match &self.milk.gpu {
                Some(g) => g.instance.clone(),
                None => gpu::instance(),
            };
            // SAFETY : la surface est rangée dans la `SkinWindow`, qui la détruit avant sa fenêtre.
            let surface = unsafe { gpu::surface(&instance, &*win.window) }?;
            if self.milk.gpu.is_none() {
                self.milk.gpu = Some(Rc::new(Gpu::new(instance, Some(&surface), false)?));
            }
            let g = self.milk.gpu.as_ref().expect("gpu");
            let (pw, ph) = win.phys_size();
            win.gpu = Some(Presenter::new(g, surface, pw as u32, ph as u32, true)?);
        }
        if self.milk.engine.is_none() {
            let g = self.milk.gpu.clone().expect("gpu");
            let vp = self.milk.ui.viewport();
            let z = self.slot(Kind::Vis).win.as_ref().map(|w| w.zoom).unwrap_or(1);
            let presets = PresetList::scan(&self.paths.presets_dir);
            let mut md = MilkDrop::new(&g, (vp.w as u32 * z).max(16), (vp.h as u32 * z).max(16), rate, presets)?;
            self.apply_milk_settings_to(&mut md);
            if let Some(k) = self.player.settings.milkdrop.last_preset.clone() {
                md.select_key(&k);
            }
            self.milk.items = md.presets.items.iter().map(|p| (p.name.clone(), p.folder.clone())).collect();
            self.milk.engine = Some(md);
        }
        Ok(())
    }

    fn apply_milk_settings_to(&self, md: &mut MilkDrop) {
        let s = &self.player.settings.milkdrop;
        md.auto_secs = s.auto_secs as f64;
        md.blend_secs = s.blend_tenths as f64 / 10.0;
        md.shuffle = s.shuffle;
        md.locked = s.locked;
    }

    fn apply_milk_settings(&mut self) {
        if let Some(mut md) = self.milk.engine.take() {
            self.apply_milk_settings_to(&mut md);
            self.milk.engine = Some(md);
        }
    }

    pub(super) fn milk_fullscreen(&mut self, on: bool) {
        let pos = self.slot(Kind::Vis).pos;
        let Some(w) = self.slots[Kind::Vis as usize].win.as_mut() else { return };
        if w.fullscreen == on {
            return;
        }
        if on {
            self.milk.restore_pos = pos;
        }
        w.set_fullscreen(on);
        if !on {
            if let Some((x, y)) = self.milk.restore_pos.take() {
                w.set_layout_position(x, y);
                self.slots[Kind::Vis as usize].pos = Some((x, y));
            }
        }
        self.milk.note = None;
    }

    /// Oublie le cadre déjà envoyé au GPU (après un changement d'agrandissement).
    pub(super) fn milk_reset_chrome(&mut self) {
        self.milk.chrome = None;
        self.milk.note = None;
    }

    /// Texte d'état (téléchargement du pack…).
    fn milk_status(&self) -> Option<String> {
        match self.milk.pack.state() {
            PackState::Downloading { done, .. } => Some(format!("{} {:.1} Mo", t("milk.downloading"), done as f64 / 1_048_576.0)),
            PackState::Extracting { done, total } => Some(format!("{} {}%", t("milk.extracting"), done * 100 / total.max(1))),
            PackState::Failed(e) => Some(format!("{} : {e}", t("milk.failed"))),
            _ => None,
        }
    }

    /// Suit l'installation du pack ; à la fin, recharge la liste des presets.
    pub(super) fn milk_tick(&mut self) {
        let st = self.milk.pack.state();
        if self.milk.pack_seen.as_ref() == Some(&st) {
            return;
        }
        let finished = matches!(st, PackState::Ready { .. }) && !matches!(self.milk.pack_seen, Some(PackState::Ready { .. }));
        self.milk.pack_seen = Some(st.clone());
        if let (true, PackState::Ready { presets }) = (finished, &st) {
            self.milk_rescan();
            if let Some(md) = self.milk.engine.as_mut() {
                md.show_message(format!("{presets} {}", t("milk.ready")));
            }
            self.menus_dirty = true;
        }
    }

    fn milk_rescan(&mut self) {
        let list = PresetList::scan(&self.paths.presets_dir);
        self.milk.items = list.items.iter().map(|p| (p.name.clone(), p.folder.clone())).collect();
        if let Some(md) = self.milk.engine.as_mut() {
            md.set_presets(list);
        }
        if self.gen_ui.kind == GenKind::Presets {
            let q = self.gen_ui.query.clone();
            self.milk_search(&q);
        }
    }

    pub(super) fn milk_search(&mut self, q: &str) {
        let r = match self.milk.engine.as_ref() {
            Some(md) => md.presets.search(q),
            None => Vec::new(),
        };
        self.gen_ui.set_results(r);
    }

    /// Les commandes de la visualisation. Renvoie `false` si `cmd` n'en est pas une.
    pub(super) fn milk_command(&mut self, cmd: &Command, event_loop: &ActiveEventLoop) -> bool {
        use Command::*;
        match cmd {
            ToggleMilk => {
                let open = !self.slot(Kind::Vis).open;
                self.milk_set_open(open, event_loop);
            }
            MilkNext(hard) => {
                if let Some(md) = self.milk.engine.as_mut() {
                    md.next(*hard);
                }
            }
            MilkPrevious => {
                if let Some(md) = self.milk.engine.as_mut() {
                    md.previous(false);
                }
            }
            MilkToggleShuffle => {
                let s = &mut self.player.settings.milkdrop;
                s.shuffle = !s.shuffle;
                let msg = format!("{} : {}", t("milk.random"), if s.shuffle { "✓" } else { "–" });
                self.apply_milk_settings();
                if let Some(md) = self.milk.engine.as_mut() {
                    md.show_message(msg);
                }
            }
            MilkToggleLock => {
                let s = &mut self.player.settings.milkdrop;
                s.locked = !s.locked;
                let msg = t(if s.locked { "milk.locked" } else { "milk.unlocked" });
                self.apply_milk_settings();
                if let Some(md) = self.milk.engine.as_mut() {
                    md.show_message(msg);
                }
            }
            MilkPresets => {
                if !self.slot(Kind::Vis).open {
                    self.milk_set_open(true, event_loop);
                }
                if self.milk.engine.is_some() {
                    self.open_gen(GenKind::Presets, event_loop);
                    let all: Vec<usize> = (0..self.milk.items.len()).collect();
                    self.gen_ui.set_results(all);
                    self.gen_ui.take_query_changed();
                    if let Some(i) = self.milk.engine.as_ref().and_then(|m| m.current_index()) {
                        self.gen_ui.select(i);
                    }
                }
            }
            MilkFullscreen => {
                if !self.slot(Kind::Vis).open {
                    self.milk_set_open(true, event_loop);
                }
                let on = !MilkState::fullscreen(self);
                self.milk_fullscreen(on);
            }
            MilkLeaveFullscreen => self.milk_fullscreen(false),
            MilkAuto(secs) => {
                self.player.settings.milkdrop.auto_secs = *secs;
                self.apply_milk_settings();
            }
            MilkBlend(tenths) => {
                self.player.settings.milkdrop.blend_tenths = *tenths;
                self.apply_milk_settings();
            }
            MilkDownloadPack => {
                if !pack::installed(&self.paths.presets_dir) {
                    self.milk.pack.start(self.paths.presets_dir.clone());
                }
                if !self.slot(Kind::Vis).open {
                    self.milk_set_open(true, event_loop);
                }
            }
            MilkOpenFolder => dialogs::open_folder(&self.paths.presets_dir),
            MilkRescan => self.milk_rescan(),
            _ => return false,
        }
        self.menus_dirty = true;
        true
    }

    /// Dessine une image de la visualisation : MilkDrop (ou la visualisation de plugin), puis le cadre (ou le
    /// message, en plein écran).
    pub(super) fn milk_redraw(&mut self) {
        let active = self.focused == Some(Kind::Vis);
        let status = self.milk_status();
        let Some(g) = self.milk.gpu.clone() else { return };
        let plugin = self.player.plugins.vis_running();
        let plugin_name = self.player.plugins.vis_name().map(str::to_string);
        // Échantillons joués depuis l'image précédente.
        self.milk.pcm.clear();
        self.player.engine.vis_since(&mut self.milk.pcm_cursor, &mut self.milk.pcm);
        let slot = &mut self.slots[Kind::Vis as usize];
        let (Some(win), Some(md)) = (slot.win.as_mut(), self.milk.engine.as_mut()) else { return };
        if !plugin {
            md.feed(&self.milk.pcm);
        }
        let (pw, ph) = win.phys_size();
        let (pw, ph) = (pw.max(1) as u32, ph.max(1) as u32);
        let z = win.zoom.max(1);
        let message = if plugin { None } else { md.message().map(str::to_string) };
        let full = win.fullscreen;
        let (vp, overlay) = if full {
            // Message en haut à gauche, à une taille lisible.
            let text = message.clone().or(status.clone());
            let nz = (ph / 540).max(1).max(z);
            let layer = match text {
                Some(m) => {
                    let keep = matches!(&self.milk.note, Some((k, kz, _)) if *k == m && *kz == nz);
                    if !keep {
                        let w = (pw / nz).clamp(60, 420);
                        let mut img = Image::new(w, 19, 0);
                        let mut runs = Vec::new();
                        message_box(&mut Painter::new(&mut img, &self.skin), &mut runs, SkinRect::new(0, 0, w as i32, 19), &m, false);
                        let (bw, bh, px) = zoomed_argb(&img, nz, &runs);
                        let mut l = Layer::new(&g, bw, bh, false);
                        l.upload_argb(&g, bw, bh, &px);
                        self.milk.note = Some((m, nz, l));
                    }
                    self.milk.note.as_ref().map(|(_, _, l)| (Rect::new(16.0, 16.0, l.size.0 as f32, l.size.1 as f32), l))
                }
                None => None,
            };
            (Rect::new(0.0, 0.0, pw as f32, ph as f32), layer)
        } else {
            let title = plugin_name.as_deref().unwrap_or(t("milk.title"));
            let view = MilkView { active, title, message: message.as_deref(), status: status.as_deref() };
            let k = self.milk.ui.chrome_key(&view);
            let key: ChromeKey = (k.0, k.1, k.2, k.3, k.4, k.5, k.6, z);
            if self.milk.chrome.as_ref().map(|(ck, _)| ck) != Some(&key) {
                self.milk.ui.render(&mut slot.fb, &self.skin, &view);
                let (bw, bh, px) = zoomed_argb(&slot.fb, z, &self.milk.ui.text);
                let mut l = match self.milk.chrome.take() {
                    Some((_, l)) => l,
                    None => Layer::new(&g, bw, bh, false),
                };
                l.upload_argb(&g, bw, bh, &px);
                self.milk.chrome = Some((key, l));
            }
            let v = self.milk.ui.viewport();
            let vp = Rect::new((v.x as u32 * z) as f32, (v.y as u32 * z) as f32, (v.w as u32 * z) as f32, (v.h as u32 * z) as f32);
            let chrome = self.milk.chrome.as_ref().map(|(_, l)| (Rect::new(0.0, 0.0, l.size.0 as f32, l.size.1 as f32), l));
            (vp, chrome)
        };
        let Some(presenter) = win.gpu.as_mut() else { return };
        presenter.resize(&g, pw, ph);
        // Visualisation de plugin : petite image calculée par le plugin, agrandie en gros pixels.
        if plugin {
            let (w, h) = alpagamp_plugins::vis_size(vp.w as u32, vp.h as u32);
            if self.player.plugins.vis_frame(w, h, &self.player.engine, &mut self.milk.plugin_px) {
                let layer = match self.milk.plugin_layer.take() {
                    Some(l) if l.size == (w, h) => l,
                    _ => Layer::new(&g, w, h, false),
                };
                let layer = self.milk.plugin_layer.insert(layer);
                layer.upload_argb(&g, w, h, &self.milk.plugin_px);
                let mut draws = vec![Draw { layer, dst: vp, blend: false }];
                if let Some((dst, l)) = overlay {
                    draws.push(Draw { layer: l, dst, blend: true });
                }
                presenter.frame(&g, |_| {}, &draws);
            }
            return;
        }
        let (rw, rh) = render_size(vp.w as u32, vp.h as u32, MAX_RENDER);
        md.resize(&g, rw, rh);
        // MilkDrop d'abord (il lui faut `&mut md`), puis la composition qui emprunte son calque.
        let mut enc = g.device.create_command_encoder(&Default::default());
        md.render(&mut enc);
        g.queue.submit(Some(enc.finish()));
        let mut draws = vec![Draw { layer: md.layer(), dst: vp, blend: false }];
        if let Some((dst, layer)) = overlay {
            draws.push(Draw { layer, dst, blend: true });
        }
        presenter.frame(&g, |_| {}, &draws);
        if let Some(p) = md.current() {
            let key = p.key();
            if self.player.settings.milkdrop.last_preset.as_deref() != Some(key.as_str()) {
                self.player.settings.milkdrop.last_preset = Some(key);
            }
        }
    }
}
