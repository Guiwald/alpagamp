//! Le téléphone : écran composé par le GPU (wgpu) quand c'est possible, et visualisation MilkDrop en plein écran.
//!
//! Une surface Android ne peut avoir qu'un seul producteur (le CPU de softbuffer **ou** Vulkan) : le choix se fait
//! au démarrage. Avec le GPU, l'image des fenêtres (dessinée par le CPU) est envoyée telle quelle (BGRA) dans un
//! calque ; la visualisation est rendue dessous en plein écran. Sans GPU, softbuffer comme avant, et pas de
//! visualisation.

use std::rc::Rc;
use std::time::Instant;

use alpagamp_core::i18n::t;
use alpagamp_milkdrop::{Draw, Gpu, Layer, MilkDrop, PackInstaller, PackState, PresetList, Presenter, Rect, gpu, pack, render_size};
use alpagamp_skin::Image;
use alpagamp_ui::milk_window::message_box;
use alpagamp_ui::paint::Painter;
use alpagamp_ui::text::{Rect as SkinRect, draw_runs};
use alpagamp_ui::{GenKind, GenWindow, MenuItem};

use super::{MenuKind, Mobile, Overlay};

/// Plus grand côté de l'image calculée par MilkDrop sur téléphone (le GPU l'agrandit à l'écran).
const MAX_RENDER: u32 = 1280;
/// Fraction de la largeur à parcourir pour qu'un glissement change de preset.
const SWIPE: f64 = 0.15;

pub(super) enum Screen {
    Soft(softbuffer::Surface<Rc<winit::window::Window>, Rc<winit::window::Window>>),
    Gpu(Presenter),
}

#[derive(Default)]
pub(super) struct MobileMilk {
    pub gpu: Option<Rc<Gpu>>,
    /// Le GPU a refusé : on reste en softbuffer pour toute la session.
    pub gpu_failed: bool,
    pub engine: Option<MilkDrop>,
    /// Visualisation en plein écran.
    pub on: bool,
    pub ui_layer: Option<Layer>,
    pub ui_buf: Vec<u32>,
    note: Option<(String, Layer)>,
    /// Aide (gestes) affichée en bas jusqu'à cet instant.
    hint_until: Option<Instant>,
    hint: Option<(String, Layer)>,
    pcm_cursor: u64,
    pcm: Vec<f32>,
    pub pack: PackInstaller,
    pack_seen: Option<PackState>,
    pub items: Vec<(String, String)>,
    /// Toucher en cours sur la visualisation : (id, départ, instant, appui long déjà traité).
    pub touch: Option<(u64, (f64, f64), Instant, bool)>,
    last_tap: Option<Instant>,
    /// Visualisation de plugin : dernière image, sa taille, son calque GPU.
    plugin_px: Vec<u32>,
    plugin_size: (u32, u32),
    plugin_layer: Option<Layer>,
}

impl MobileMilk {
    /// Agrandit l'image de la visualisation de plugin sur tout l'écran (gros pixels).
    pub(super) fn blit_plugin(&self, buf: &mut [u32], w: u32, h: u32) {
        let (pw, ph) = self.plugin_size;
        if pw == 0 || ph == 0 || self.plugin_px.len() < (pw * ph) as usize {
            buf.fill(0);
            return;
        }
        let xmap: Vec<usize> = (0..w).map(|x| (x as u64 * pw as u64 / w.max(1) as u64) as usize).collect();
        for y in 0..h as usize {
            let sy = (y as u64 * ph as u64 / h.max(1) as u64) as usize;
            let src = &self.plugin_px[sy * pw as usize..(sy + 1) * pw as usize];
            let dst = &mut buf[y * w as usize..(y + 1) * w as usize];
            for (d, &sx) in dst.iter_mut().zip(&xmap) {
                *d = src[sx];
            }
        }
    }
}

impl Mobile {
    /// Surface de la fenêtre : GPU si possible (et voulu), sinon softbuffer.
    pub(super) fn make_screen(&mut self, w: Rc<winit::window::Window>) -> Option<Screen> {
        if self.player.settings.milkdrop.gpu && !self.milk.gpu_failed {
            match self.gpu_screen(&w) {
                Ok(p) => return Some(Screen::Gpu(p)),
                Err(e) => {
                    log::warn!("GPU indisponible, affichage logiciel : {e}");
                    self.milk.gpu_failed = true;
                }
            }
        }
        match softbuffer::Context::new(w.clone()).and_then(|c| softbuffer::Surface::new(&c, w.clone())) {
            Ok(s) => Some(Screen::Soft(s)),
            Err(e) => {
                log::error!("surface : {e}");
                None
            }
        }
    }

    fn gpu_screen(&mut self, w: &winit::window::Window) -> Result<Presenter, String> {
        let instance = match &self.milk.gpu {
            Some(g) => g.instance.clone(),
            None => gpu::instance(),
        };
        // SAFETY : la surface (dans `screen`) est détruite à la suspension, avant la fenêtre native.
        let surface = unsafe { gpu::surface(&instance, w) }?;
        if self.milk.gpu.is_none() {
            self.milk.gpu = Some(Rc::new(Gpu::new(instance, Some(&surface), true)?));
        }
        let g = self.milk.gpu.as_ref().expect("gpu");
        let size = w.inner_size();
        Presenter::new(g, surface, size.width, size.height, true)
    }

    /// MilkDrop demande le GPU ; une visualisation de plugin marche aussi sans.
    pub(super) fn milk_available(&self) -> bool {
        matches!(self.screen, Some(Screen::Gpu(_))) || self.player.plugins.vis_running()
    }

    /// Calcule une image de la visualisation de plugin pour un écran `w × h`.
    pub(super) fn plugin_vis_frame(&mut self, w: u32, h: u32) -> bool {
        let (pw, ph) = alpagamp_plugins::vis_size(w, h);
        let ok = self.player.plugins.vis_frame(pw, ph, &self.player.engine, &mut self.milk.plugin_px);
        self.milk.plugin_size = if ok { (pw, ph) } else { (0, 0) };
        ok
    }

    /// Entre en visualisation plein écran (en créant le moteur au premier besoin).
    pub(super) fn milk_enter(&mut self) {
        if !self.milk_available() {
            self.main_ui.marquee.flash(t("milk.no_gpu"));
            return;
        }
        if self.milk.engine.is_none() && matches!(self.screen, Some(Screen::Gpu(_))) {
            let Some(g) = self.milk.gpu.clone() else { return };
            let presets = PresetList::scan(&self.data.join("presets"));
            match MilkDrop::new(&g, 256, 256, self.player.engine.output_rate(), presets) {
                Ok(mut md) => {
                    self.apply_milk_settings_to(&mut md);
                    if let Some(k) = self.player.settings.milkdrop.last_preset.clone() {
                        md.select_key(&k);
                    }
                    self.milk.items = md.presets.items.iter().map(|p| (p.name.clone(), p.folder.clone())).collect();
                    self.milk.engine = Some(md);
                }
                Err(e) => {
                    log::warn!("visualisation : {e}");
                    self.main_ui.marquee.flash(t("milk.no_gpu"));
                    return;
                }
            }
        }
        self.close_overlay();
        self.milk.on = true;
        self.milk.hint_until = Some(Instant::now() + std::time::Duration::from_secs(5));
        let name = self.milk_name();
        self.milk_message(name);
    }

    /// Nom de ce qui est affiché : visualisation de plugin ou preset MilkDrop.
    fn milk_name(&self) -> String {
        match self.player.plugins.vis_name() {
            Some(n) => n.to_string(),
            None => self.milk.engine.as_ref().and_then(|m| m.current()).map(|p| p.name.clone()).unwrap_or_default(),
        }
    }

    pub(super) fn milk_leave(&mut self) {
        self.milk.on = false;
        self.milk.touch = None;
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

    fn milk_message(&mut self, m: impl Into<String>) {
        if let Some(md) = self.milk.engine.as_mut() {
            md.show_message(m);
        }
    }

    // --- Toucher ---------------------------------------------------------------------------------

    pub(super) fn milk_touch_start(&mut self, id: u64, x: f64, y: f64) {
        if self.milk.touch.is_none() {
            self.milk.touch = Some((id, (x, y), Instant::now(), false));
        }
    }

    pub(super) fn milk_touch_end(&mut self, id: u64, x: f64, y: f64, cancelled: bool) {
        let Some((tid, (sx, sy), _, long_done)) = self.milk.touch.take() else { return };
        if tid != id || cancelled || long_done {
            return;
        }
        let (dx, dy) = (x - sx, y - sy);
        let w = self.layout.w.max(1) as f64;
        if dx.abs() > w * SWIPE && dx.abs() > dy.abs() {
            self.milk.last_tap = None;
            if self.player.plugins.vis_running() {
                return;
            }
            if let Some(md) = self.milk.engine.as_mut() {
                if dx < 0.0 { md.next(false) } else { md.previous(false) }
            }
            return;
        }
        if dx.abs() > 20.0 || dy.abs() > 20.0 {
            return;
        }
        let now = Instant::now();
        if self.milk.last_tap.is_some_and(|t| now - t < super::DOUBLE_TAP) {
            self.milk.last_tap = None;
            self.milk_leave();
            return;
        }
        self.milk.last_tap = Some(now);
        let name = self.milk_name();
        self.milk_message(name);
        self.milk.hint_until = Some(now + std::time::Duration::from_secs(4));
    }

    /// Appui long sur la visualisation : son menu.
    pub(super) fn milk_long_press(&mut self) -> bool {
        let Some((_, (sx, sy), at, long_done)) = self.milk.touch.as_mut() else { return false };
        if *long_done || at.elapsed() < super::LONG_PRESS {
            return false;
        }
        let _ = (sx, sy);
        *long_done = true;
        self.open_menu(MenuKind::Milk);
        true
    }

    // --- Menus et commandes ----------------------------------------------------------------------

    pub(super) fn milk_menu(&self) -> Vec<MenuItem> {
        if let Some(v) = self.plugin_vis_menu() {
            return v;
        }
        let s = &self.player.settings.milkdrop;
        let mut auto = vec![MenuItem::check("milk_auto_0", t("milk.never"), s.auto_secs == 0)];
        auto.extend([10u32, 15, 20, 30, 45, 60, 120].iter().map(|n| MenuItem::check(&format!("milk_auto_{n}"), &format!("{n} s"), s.auto_secs == *n)));
        let mut blend = vec![MenuItem::check("milk_blend_0", t("milk.cut"), s.blend_tenths == 0)];
        blend.extend([10u32, 20, 27, 40, 60].iter().map(|n| MenuItem::check(&format!("milk_blend_{n}"), &format!("{:.1} s", *n as f32 / 10.0), s.blend_tenths == *n)));
        let installed = pack::installed(&self.data.join("presets")) || self.milk.pack.busy();
        let mut pack_item = MenuItem::new("milk_pack", t("milk.download"));
        if installed {
            pack_item = pack_item.disabled();
        }
        vec![
            MenuItem::new("milk_next", t("milk.next")),
            MenuItem::new("milk_cut", t("milk.next_cut")),
            MenuItem::new("milk_prev", t("milk.previous")),
            MenuItem::new("milk_choose", t("milk.choose")),
            MenuItem::separator(),
            MenuItem::check("milk_random", t("milk.random"), s.shuffle),
            MenuItem::check("milk_lock", t("milk.lock"), s.locked),
            MenuItem::sub(t("milk.auto"), auto),
            MenuItem::sub(t("milk.blend"), blend),
            MenuItem::separator(),
            pack_item,
            MenuItem::separator(),
            MenuItem::new("milk_exit", t("milk.exit")),
        ]
    }

    /// Commandes de menu de la visualisation ; `false` si `id` n'en est pas une.
    pub(super) fn milk_menu_command(&mut self, id: &str) -> bool {
        let num = |prefix: &str| id.strip_prefix(prefix).and_then(|n| n.parse::<u32>().ok());
        match id {
            "milk" => self.milk_enter(),
            "milk_exit" => self.milk_leave(),
            "milk_next" | "milk_cut" => {
                if let Some(md) = self.milk.engine.as_mut() {
                    md.next(id == "milk_cut");
                }
            }
            "milk_prev" => {
                if let Some(md) = self.milk.engine.as_mut() {
                    md.previous(false);
                }
            }
            "milk_choose" => self.milk_open_picker(),
            "milk_random" => {
                let s = &mut self.player.settings.milkdrop;
                s.shuffle = !s.shuffle;
                self.apply_milk_settings();
            }
            "milk_lock" => {
                let s = &mut self.player.settings.milkdrop;
                s.locked = !s.locked;
                let m = t(if s.locked { "milk.locked" } else { "milk.unlocked" });
                self.apply_milk_settings();
                self.milk_message(m);
            }
            "milk_pack" => {
                if !pack::installed(&self.data.join("presets")) {
                    self.milk.pack.start(self.data.join("presets"));
                }
            }
            "milk_gpu" => {
                let s = &mut self.player.settings.milkdrop;
                s.gpu = !s.gpu;
            }
            _ => {
                if let Some(v) = num("milk_auto_") {
                    self.player.settings.milkdrop.auto_secs = v;
                    self.apply_milk_settings();
                } else if let Some(v) = num("milk_blend_") {
                    self.player.settings.milkdrop.blend_tenths = v;
                    self.apply_milk_settings();
                } else {
                    return false;
                }
            }
        }
        true
    }

    pub(super) fn milk_open_picker(&mut self) {
        if self.milk.engine.is_none() {
            return;
        }
        let mut g = GenWindow::new(GenKind::Presets);
        g.set_results((0..self.milk.items.len()).collect());
        g.take_query_changed();
        if let Some(i) = self.milk.engine.as_ref().and_then(|m| m.current_index()) {
            g.select(i);
        }
        self.overlay = Some(Overlay::Gen(g));
        self.host.show_keyboard(true);
    }

    pub(super) fn milk_select(&mut self, i: usize) {
        if let Some(md) = self.milk.engine.as_mut() {
            md.select(i, false);
        }
    }

    /// Suit le téléchargement du pack (message à l'écran, liste rechargée à la fin).
    pub(super) fn milk_tick(&mut self) {
        let st = self.milk.pack.state();
        if self.milk.pack_seen.as_ref() == Some(&st) {
            return;
        }
        self.milk.pack_seen = Some(st.clone());
        let msg = match &st {
            PackState::Downloading { done, .. } => Some(format!("{} {:.1} Mo", t("milk.downloading"), *done as f64 / 1_048_576.0)),
            PackState::Extracting { done, total } => Some(format!("{} {}%", t("milk.extracting"), done * 100 / (*total).max(1))),
            PackState::Failed(e) => Some(format!("{} : {e}", t("milk.failed"))),
            PackState::Ready { presets } => {
                let list = PresetList::scan(&self.data.join("presets"));
                self.milk.items = list.items.iter().map(|p| (p.name.clone(), p.folder.clone())).collect();
                if let Some(md) = self.milk.engine.as_mut() {
                    md.set_presets(list);
                }
                Some(format!("{presets} {}", t("milk.ready")))
            }
            PackState::Idle => None,
        };
        if let Some(m) = msg {
            self.milk_message(m.clone());
            self.main_ui.marquee.flash(&m);
        }
    }

    // --- Rendu -----------------------------------------------------------------------------------

    /// Un message dans un cadre noir, à l'échelle des fenêtres (texte net à la résolution de l'écran).
    fn note_layer(&self, g: &Gpu, text: &str) -> Layer {
        let z = (self.layout.scale.round() as u32).max(1);
        let w = self.layout.w;
        let mw = (w / z).max(60);
        let mut img = Image::new(mw, 19, 0);
        let mut runs = Vec::new();
        message_box(&mut Painter::new(&mut img, &self.skin), &mut runs, SkinRect::new(0, 0, mw as i32, 19), text, false);
        let (bw, bh) = (mw * z, 19 * z);
        let mut px = vec![0u32; (bw * bh) as usize];
        for y in 0..bh {
            for x in 0..bw {
                px[(y * bw + x) as usize] = img.pixels[((y / z) * mw + x / z) as usize];
            }
        }
        draw_runs(&mut px, bw, bh, z as f32, &runs);
        let mut l = Layer::new(g, bw, bh, false);
        l.upload_argb(g, bw, bh, &px);
        l
    }

    /// Une image avec le GPU : la visualisation (si elle est allumée), puis les fenêtres (ou la seule
    /// surimpression, par-dessus la visualisation).
    pub(super) fn redraw_gpu(&mut self) {
        let Some(g) = self.milk.gpu.clone() else { return };
        let (w, h) = (self.layout.w, self.layout.h);
        let overlay_text = self.render_windows();
        let plugin = self.milk.on && self.player.plugins.vis_running();
        let milk_on = self.milk.on && (self.milk.engine.is_some() || plugin);
        // Les fenêtres (ou la surimpression seule) dans un tampon CPU, puis dans un calque.
        let show_ui = !milk_on || self.overlay.is_some();
        if show_ui {
            let n = (w * h) as usize;
            let mut buf = std::mem::take(&mut self.milk.ui_buf);
            buf.resize(n, 0);
            if milk_on {
                buf.fill(0);
            }
            self.compose(&mut buf, w, h, &overlay_text, milk_on, if milk_on { 0xFFFF_FFFF } else { 0x00FF_FFFF });
            let layer = self.milk.ui_layer.get_or_insert_with(|| Layer::new(&g, w, h, false));
            layer.upload_argb(&g, w, h, &buf);
            self.milk.ui_buf = buf;
        }
        // Visualisation.
        let mut enc = g.device.create_command_encoder(&Default::default());
        let plugin_ok = plugin && self.plugin_vis_frame(w, h);
        if plugin_ok {
            let (pw, ph) = self.milk.plugin_size;
            let layer = match self.milk.plugin_layer.take() {
                Some(l) if l.size == (pw, ph) => l,
                _ => Layer::new(&g, pw, ph, false),
            };
            let layer = self.milk.plugin_layer.insert(layer);
            layer.upload_argb(&g, pw, ph, &self.milk.plugin_px);
        }
        if milk_on {
            self.milk.pcm.clear();
            self.player.engine.vis_since(&mut self.milk.pcm_cursor, &mut self.milk.pcm);
            if !plugin {
                if let Some(md) = self.milk.engine.as_mut() {
                    md.feed(&self.milk.pcm);
                    let (rw, rh) = render_size(w, h, MAX_RENDER);
                    md.resize(&g, rw, rh);
                    md.render(&mut enc);
                }
            }
            // Message (nom du preset…) en haut, sous la barre d'état ; aide en bas.
            let msg = self.milk.engine.as_ref().and_then(|md| md.message()).map(str::to_string);
            self.milk.note = match msg {
                Some(m) => match self.milk.note.take() {
                    Some((k, l)) if k == m => Some((k, l)),
                    _ => Some((m.clone(), self.note_layer(&g, &m))),
                },
                None => None,
            };
            let hint_on = self.milk.hint_until.is_some_and(|u| Instant::now() < u);
            let hint_text = t(if plugin { "plugins.hint" } else { "milk.hint" });
            self.milk.hint = match (hint_on, self.milk.hint.take()) {
                (false, _) => None,
                (true, Some(h)) if h.0 == hint_text => Some(h),
                (true, _) => {
                    let text = hint_text.to_string();
                    let l = self.note_layer(&g, &text);
                    Some((text, l))
                }
            };
            if let Some(md) = self.milk.engine.as_ref().and_then(|m| m.current()).filter(|_| !plugin) {
                let key = md.key();
                if self.player.settings.milkdrop.last_preset.as_deref() != Some(key.as_str()) {
                    self.player.settings.milkdrop.last_preset = Some(key);
                }
            }
        }
        g.queue.submit(Some(enc.finish()));
        let top = self.host.content_rect().map(|(t, _)| t.max(0) as f32).unwrap_or(0.0);
        let full = Rect::new(0.0, 0.0, w as f32, h as f32);
        let mut draws = Vec::new();
        if milk_on {
            if plugin {
                if let (true, Some(l)) = (plugin_ok, &self.milk.plugin_layer) {
                    draws.push(Draw { layer: l, dst: full, blend: false });
                }
            } else if let Some(md) = self.milk.engine.as_ref() {
                draws.push(Draw { layer: md.layer(), dst: full, blend: false });
            }
            if let Some((_, l)) = &self.milk.note {
                draws.push(Draw { layer: l, dst: Rect::new(0.0, top + 8.0, l.size.0 as f32, l.size.1 as f32), blend: true });
            }
            if let Some((_, l)) = &self.milk.hint {
                let bottom = self.host.content_rect().map(|(_, b)| b as f32).filter(|b| *b > 0.0).unwrap_or(h as f32);
                draws.push(Draw { layer: l, dst: Rect::new(0.0, bottom - l.size.1 as f32 - 8.0, l.size.0 as f32, l.size.1 as f32), blend: true });
            }
        }
        if show_ui {
            if let Some(l) = &self.milk.ui_layer {
                draws.push(Draw { layer: l, dst: full, blend: milk_on });
            }
        }
        if let Some(Screen::Gpu(p)) = self.screen.as_mut() {
            p.resize(&g, w, h);
            p.frame(&g, |_| {}, &draws);
        }
    }
}
