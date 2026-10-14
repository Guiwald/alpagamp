//! Une fenêtre système sans bordure qui affiche une image skin agrandie au pixel près, plus le texte vectoriel
//! dessiné à la résolution de l'écran. La fenêtre de visualisation est présentée par le GPU (wgpu) au lieu de
//! softbuffer, et peut passer en plein écran.

use std::num::NonZeroU32;
use std::rc::Rc;

use alpagamp_skin::Image;
use alpagamp_ui::text::{TextRun, draw_runs};
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowLevel};

/// Les positions et tailles de la disposition (aimantation, groupe de fenêtres) sont en **demi-points** sur macOS :
/// l'espace de coordonnées global y est continu d'un écran à l'autre, même quand leurs densités diffèrent
/// (Retina ×2 et écran externe ×1), et le demi-point fait tomber juste les demi-crans (275 × 1,5 = 412,5 points).
/// Ailleurs (Windows, Linux), en pixels physiques.
pub const POINTS: bool = cfg!(target_os = "macos");

/// Unités de disposition par point (macOS).
pub const PER_POINT: f64 = if POINTS { 2.0 } else { 1.0 };

/// Unités de disposition par « point » d'interface, pour les marges et décalages écrits en dur.
pub const PT: i32 = if POINTS { 2 } else { 1 };

/// Agrandissement choisi, en demi-crans : 2 = ×1, 3 = ×1,5, 4 = ×2, 5 = ×2,5, 6 = ×3.
pub fn halves_of(pct: u32) -> u32 {
    (pct / 50).clamp(2, 6)
}

/// `n` pixels skin agrandis `z` fois, arrondis au pixel.
pub fn scaled(n: u32, z: f64) -> u32 {
    (n as f64 * z).round() as u32
}

pub struct SkinWindow {
    pub window: Rc<Window>,
    /// Surface CPU (toutes les fenêtres sauf la visualisation).
    soft: Option<(softbuffer::Surface<Rc<Window>, Rc<Window>>, softbuffer::Context<Rc<Window>>)>,
    /// Surface GPU (visualisation), créée par l'application.
    pub gpu: Option<alpagamp_milkdrop::Presenter>,
    /// En plein écran, la taille est celle de l'écran et non celle du skin.
    pub fullscreen: bool,
    /// Pixels physiques par pixel skin : densité de l'écran (Retina ×2) × agrandissement choisi. Pas toujours
    /// entier (×1,5 sur un écran non Retina : pixels inégaux).
    pub zoom: f64,
    /// Taille en pixels skin.
    pub skin_size: (u32, u32),
    /// Agrandissement choisi, en demi-crans (voir [`halves_of`]), en plus de la densité de l'écran.
    halves: u32,
}

/// Options de création.
pub struct WindowOptions<'a> {
    pub title: &'a str,
    pub skin_size: (u32, u32),
    pub position: Option<(i32, i32)>,
    pub on_top: bool,
    /// Agrandissement choisi, en demi-crans.
    pub halves: u32,
    /// Fenêtre secondaire : pas d'entrée dans la barre des tâches (Windows).
    pub secondary: bool,
    /// Fenêtre propriétaire (Windows) : les fenêtres secondaires suivent la principale (réduction, ordre).
    pub owner: Option<&'a Window>,
    /// Présentée par le GPU (pas de surface CPU).
    pub gpu: bool,
}

impl SkinWindow {
    pub fn new(event_loop: &ActiveEventLoop, o: WindowOptions) -> Result<SkinWindow, String> {
        let mut attrs = WindowAttributes::default()
            .with_title(o.title)
            .with_decorations(false)
            .with_resizable(false)
            .with_visible(false);
        let halves = o.halves.clamp(2, 6);
        // macOS : la fenêtre naît à sa taille finale. Agrandie ensuite, elle grandirait par le haut (macOS garde le
        // coin bas-gauche) et ne serait plus à la position demandée.
        attrs = if POINTS {
            attrs.with_inner_size(LogicalSize::new(o.skin_size.0 as f64 * halves as f64 / 2.0, o.skin_size.1 as f64 * halves as f64 / 2.0))
        } else {
            attrs.with_inner_size(PhysicalSize::new(o.skin_size.0, o.skin_size.1))
        };
        if let Some((x, y)) = o.position {
            attrs = if POINTS { attrs.with_position(LogicalPosition::new(x as f64 / PER_POINT, y as f64 / PER_POINT)) } else { attrs.with_position(PhysicalPosition::new(x, y)) };
        }
        if o.on_top {
            attrs = attrs.with_window_level(WindowLevel::AlwaysOnTop);
        }
        #[cfg(target_os = "windows")]
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            use winit::platform::windows::WindowAttributesExtWindows;
            attrs = attrs.with_skip_taskbar(o.secondary);
            if let Some(owner) = o.owner {
                if let Ok(h) = owner.window_handle() {
                    if let RawWindowHandle::Win32(h) = h.as_raw() {
                        attrs = attrs.with_owner_window(h.hwnd.get());
                    }
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = (o.secondary, o.owner);
        let icon = alpagamp_skin::app_icon(64);
        let rgba: Vec<u8> = icon.pixels.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, (p >> 24) as u8]).collect();
        if let Ok(i) = winit::window::Icon::from_rgba(rgba, 64, 64) {
            attrs = attrs.with_window_icon(Some(i));
        }
        let window = Rc::new(event_loop.create_window(attrs).map_err(|e| e.to_string())?);
        let soft = if o.gpu {
            None
        } else {
            let context = softbuffer::Context::new(window.clone()).map_err(|e| e.to_string())?;
            let surface = softbuffer::Surface::new(&context, window.clone()).map_err(|e| e.to_string())?;
            Some((surface, context))
        };
        let mut w = SkinWindow { window, soft, gpu: None, fullscreen: false, zoom: 1.0, skin_size: o.skin_size, halves };
        w.zoom = w.zoom_for(halves);
        w.apply_size();
        w.window.set_visible(true);
        Ok(w)
    }

    /// Agrandissement en pixels physiques : densité de l'écran (Retina ×2) × choix de l'utilisateur.
    pub fn zoom_for(&self, halves: u32) -> f64 {
        let sf = self.window.scale_factor().round().max(1.0);
        sf * halves.max(2) as f64 / 2.0
    }

    /// Points (macOS) par pixel skin.
    pub fn points_per_skin(&self) -> f64 {
        self.halves as f64 / 2.0
    }

    /// Pixels physiques par unité de disposition.
    pub fn unit(&self) -> f64 {
        if POINTS { self.window.scale_factor() / PER_POINT } else { 1.0 }
    }

    /// Taille (unités de disposition) qu'aurait la fenêtre avec cette taille skin et cet agrandissement.
    pub fn layout_size_for(&self, skin_size: (u32, u32), halves: u32) -> (i32, i32) {
        if POINTS {
            // Demi-points : `skin × halves / 2` points.
            let h = halves.max(2);
            ((skin_size.0 * h) as i32, (skin_size.1 * h) as i32)
        } else {
            let z = self.zoom_for(halves);
            (scaled(skin_size.0, z) as i32, scaled(skin_size.1, z) as i32)
        }
    }

    /// Taille actuelle, en unités de disposition.
    pub fn layout_size(&self) -> (i32, i32) {
        if self.fullscreen {
            let s = self.window.inner_size();
            let u = self.unit();
            return ((s.width as f64 / u).round() as i32, (s.height as f64 / u).round() as i32);
        }
        self.layout_size_for(self.skin_size, self.halves)
    }

    /// Taille actuelle en pixels physiques.
    pub fn phys_size(&self) -> (i32, i32) {
        if self.fullscreen {
            let s = self.window.inner_size();
            return (s.width as i32, s.height as i32);
        }
        (scaled(self.skin_size.0, self.zoom) as i32, scaled(self.skin_size.1, self.zoom) as i32)
    }

    /// Place la fenêtre (unités de disposition).
    pub fn set_layout_position(&self, x: i32, y: i32) {
        if POINTS {
            self.window.set_outer_position(LogicalPosition::new(x as f64 / PER_POINT, y as f64 / PER_POINT));
        } else {
            self.window.set_outer_position(PhysicalPosition::new(x, y));
        }
    }

    /// Position donnée par le système (pixels physiques) → unités de disposition.
    pub fn layout_from_physical(&self, p: PhysicalPosition<i32>) -> (i32, i32) {
        let u = self.unit();
        ((p.x as f64 / u).round() as i32, (p.y as f64 / u).round() as i32)
    }

    /// Position actuelle (unités de disposition).
    pub fn layout_position(&self) -> Option<(i32, i32)> {
        self.window.outer_position().ok().map(|p| self.layout_from_physical(p))
    }

    /// Passe en plein écran (sans bordure, sur l'écran courant) ou en revient.
    pub fn set_fullscreen(&mut self, on: bool) {
        self.fullscreen = on;
        if on {
            // Une fenêtre de taille fixe ne peut pas être agrandie par le système (indications de taille min/max).
            // (winit/X11 retient la taille de création comme minimum et maximum : on les efface.)
            self.window.set_min_inner_size(None::<PhysicalSize<u32>>);
            self.window.set_max_inner_size(None::<PhysicalSize<u32>>);
            self.window.set_resizable(true);
            self.window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
        } else {
            self.window.set_fullscreen(None);
            self.window.set_resizable(false);
            self.apply_size();
        }
        self.window.request_redraw();
    }

    /// Change la taille skin et/ou l'agrandissement.
    pub fn set_geometry(&mut self, skin_size: (u32, u32), halves: u32) {
        let halves = halves.clamp(2, 6);
        let zoom = self.zoom_for(halves);
        if skin_size != self.skin_size || zoom != self.zoom || halves != self.halves {
            self.skin_size = skin_size;
            self.zoom = zoom;
            self.halves = halves;
            self.apply_size();
        }
    }

    /// Après un changement d'écran (facteur d'échelle).
    pub fn rescale(&mut self) {
        self.zoom = self.zoom_for(self.halves);
        self.apply_size();
    }

    fn apply_size(&mut self) {
        if self.fullscreen {
            return;
        }
        let (w, h) = (scaled(self.skin_size.0, self.zoom), scaled(self.skin_size.1, self.zoom));
        if POINTS {
            // macOS change la taille en gardant le coin bas-gauche : on remet le coin haut-gauche où il était, sans
            // quoi la fenêtre monte ou descend et se décolle de son groupe.
            let top_left = self.window.outer_position().ok();
            let p = self.points_per_skin();
            let (lw, lh) = (self.skin_size.0 as f64 * p, self.skin_size.1 as f64 * p);
            let _ = self.window.request_inner_size(LogicalSize::new(lw, lh));
            if let Some(tl) = top_left {
                if self.window.outer_position().ok() != Some(tl) {
                    self.window.set_outer_position(tl);
                }
            }
        } else {
            let _ = self.window.request_inner_size(PhysicalSize::new(w, h));
        }
        if let (Some(nw), Some(nh), Some((surface, _))) = (NonZeroU32::new(w), NonZeroU32::new(h), self.soft.as_mut()) {
            let _ = surface.resize(nw, nh);
        }
        self.window.request_redraw();
    }

    /// Coordonnées skin d'un point de la fenêtre (pixels physiques).
    pub fn to_skin(&self, x: f64, y: f64) -> (i32, i32) {
        let z = self.zoom.max(0.5);
        ((x / z).floor() as i32, (y / z).floor() as i32)
    }

    /// Affiche l'image (agrandie au plus proche voisin) puis le texte.
    pub fn present(&mut self, img: &Image, text: &[TextRun]) {
        let (bw, bh) = (scaled(self.skin_size.0, self.zoom), scaled(self.skin_size.1, self.zoom));
        let (Some(nw), Some(nh)) = (NonZeroU32::new(bw), NonZeroU32::new(bh)) else { return };
        let Some((surface, _)) = self.soft.as_mut() else { return };
        let _ = surface.resize(nw, nh);
        let Ok(mut buf) = surface.buffer_mut() else { return };
        zoom_into(&mut buf, img, bw, bh, 0x00FF_FFFF);
        if !text.is_empty() {
            draw_runs(&mut buf, bw, bh, self.zoom as f32, text);
        }
        let _ = buf.present();
    }
}

/// Agrandit `img` au plus proche voisin dans `buf` (`bw` × `bh`), pixels masqués par `mask`. Avec un facteur entier,
/// chaque pixel skin devient un carré ; sinon (×1,5 sur un écran non Retina), certains sont plus larges que d'autres.
pub fn zoom_into(buf: &mut [u32], img: &Image, bw: u32, bh: u32, mask: u32) {
    if img.width == 0 || img.height == 0 || bw == 0 || bh == 0 || buf.len() < (bw * bh) as usize {
        return;
    }
    let col = |x: u32, n: u32, total: u32| ((x as u64 * n as u64 / total as u64) as u32).min(n - 1);
    let cols: Vec<usize> = (0..bw).map(|x| col(x, img.width, bw) as usize).collect();
    for y in 0..bh {
        let sy = col(y, img.height, bh);
        let src_row = &img.pixels[(sy * img.width) as usize..((sy + 1) * img.width) as usize];
        let dst_row = &mut buf[(y * bw) as usize..((y + 1) * bw) as usize];
        for (d, &sx) in dst_row.iter_mut().zip(&cols) {
            *d = src_row[sx] & mask;
        }
    }
}

/// Image agrandie `z` fois avec son canal alpha, et son texte : un calque pour la composition GPU.
pub fn zoomed_argb(img: &Image, z: f64, text: &[TextRun]) -> (u32, u32, Vec<u32>) {
    let (bw, bh) = (scaled(img.width, z).max(1), scaled(img.height, z).max(1));
    let mut buf = vec![0u32; (bw * bh) as usize];
    zoom_into(&mut buf, img, bw, bh, 0xFFFF_FFFF);
    if !text.is_empty() {
        draw_runs(&mut buf, bw, bh, z as f32, text);
    }
    (bw, bh, buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_whole_and_half_steps() {
        let mut img = Image::new(2, 1, 0);
        img.pixels = vec![1, 2];
        let mut buf = vec![0; 4];
        zoom_into(&mut buf, &img, 4, 1, 0xFFFF_FFFF);
        assert_eq!(buf, vec![1, 1, 2, 2]);
        // ×1,5 : 3 pixels pour 2, le premier pixel skin en occupe 2.
        let mut buf = vec![0; 3];
        zoom_into(&mut buf, &img, 3, 1, 0xFFFF_FFFF);
        assert_eq!(buf, vec![1, 1, 2]);
        assert_eq!(scaled(275, 1.5), 413);
        assert_eq!(halves_of(150), 3);
        assert_eq!(halves_of(0), 2);
    }
}

impl Drop for SkinWindow {
    fn drop(&mut self) {
        // La surface GPU référence la fenêtre : elle part d'abord.
        self.gpu.take();
    }
}
