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

/// Les positions et tailles de la disposition (aimantation, groupe de fenêtres) sont en **points** sur macOS :
/// l'espace de coordonnées global y est continu d'un écran à l'autre, même quand leurs densités diffèrent
/// (Retina ×2 et écran externe ×1). Ailleurs (Windows, Linux), en pixels physiques.
pub const POINTS: bool = cfg!(target_os = "macos");

pub struct SkinWindow {
    pub window: Rc<Window>,
    /// Surface CPU (toutes les fenêtres sauf la visualisation).
    soft: Option<(softbuffer::Surface<Rc<Window>, Rc<Window>>, softbuffer::Context<Rc<Window>>)>,
    /// Surface GPU (visualisation), créée par l'application.
    pub gpu: Option<alpagamp_milkdrop::Presenter>,
    /// En plein écran, la taille est celle de l'écran et non celle du skin.
    pub fullscreen: bool,
    /// Facteur d'agrandissement entier (double taille × échelle de l'écran).
    pub zoom: u32,
    /// Taille en pixels skin.
    pub skin_size: (u32, u32),
    /// Agrandissement choisi par l'utilisateur (×1, ×2, ×3), en plus de la densité de l'écran.
    user_zoom: u32,
}

/// Options de création.
pub struct WindowOptions<'a> {
    pub title: &'a str,
    pub skin_size: (u32, u32),
    pub position: Option<(i32, i32)>,
    pub on_top: bool,
    /// Agrandissement choisi (×1, ×2, ×3).
    pub user_zoom: u32,
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
            .with_visible(false)
            .with_inner_size(PhysicalSize::new(o.skin_size.0, o.skin_size.1));
        if let Some((x, y)) = o.position {
            attrs = if POINTS { attrs.with_position(LogicalPosition::new(x as f64, y as f64)) } else { attrs.with_position(PhysicalPosition::new(x, y)) };
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
        let user_zoom = o.user_zoom.clamp(1, 4);
        let mut w = SkinWindow { window, soft, gpu: None, fullscreen: false, zoom: 1, skin_size: o.skin_size, user_zoom };
        w.zoom = w.zoom_for(user_zoom);
        w.apply_size();
        w.window.set_visible(true);
        Ok(w)
    }

    /// Agrandissement en pixels physiques : densité de l'écran (Retina ×2) × choix de l'utilisateur.
    pub fn zoom_for(&self, user_zoom: u32) -> u32 {
        let sf = self.window.scale_factor();
        ((sf.round().max(1.0)) as u32 * user_zoom.max(1)).max(1)
    }

    /// Agrandissement choisi par l'utilisateur.
    pub fn user_zoom(&self) -> u32 {
        self.user_zoom
    }

    /// Pixels physiques par unité de disposition.
    pub fn unit(&self) -> f64 {
        if POINTS { self.window.scale_factor() } else { 1.0 }
    }

    /// Taille (unités de disposition) qu'aurait la fenêtre avec cette taille skin et cet agrandissement.
    pub fn layout_size_for(&self, skin_size: (u32, u32), user_zoom: u32) -> (i32, i32) {
        let z = if POINTS { user_zoom.max(1) } else { self.zoom_for(user_zoom) };
        ((skin_size.0 * z) as i32, (skin_size.1 * z) as i32)
    }

    /// Taille actuelle, en unités de disposition.
    pub fn layout_size(&self) -> (i32, i32) {
        if self.fullscreen {
            let s = self.window.inner_size();
            let u = self.unit();
            return ((s.width as f64 / u).round() as i32, (s.height as f64 / u).round() as i32);
        }
        self.layout_size_for(self.skin_size, self.user_zoom)
    }

    /// Taille actuelle en pixels physiques.
    pub fn phys_size(&self) -> (i32, i32) {
        if self.fullscreen {
            let s = self.window.inner_size();
            return (s.width as i32, s.height as i32);
        }
        ((self.skin_size.0 * self.zoom) as i32, (self.skin_size.1 * self.zoom) as i32)
    }

    /// Place la fenêtre (unités de disposition).
    pub fn set_layout_position(&self, x: i32, y: i32) {
        if POINTS {
            self.window.set_outer_position(LogicalPosition::new(x as f64, y as f64));
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
    pub fn set_geometry(&mut self, skin_size: (u32, u32), user_zoom: u32) {
        let user_zoom = user_zoom.clamp(1, 4);
        let zoom = self.zoom_for(user_zoom);
        if skin_size != self.skin_size || zoom != self.zoom || user_zoom != self.user_zoom {
            self.skin_size = skin_size;
            self.zoom = zoom;
            self.user_zoom = user_zoom;
            self.apply_size();
        }
    }

    /// Après un changement d'écran (facteur d'échelle).
    pub fn rescale(&mut self) {
        self.zoom = self.zoom_for(self.user_zoom);
        self.apply_size();
    }

    fn apply_size(&mut self) {
        if self.fullscreen {
            return;
        }
        let (w, h) = (self.skin_size.0 * self.zoom, self.skin_size.1 * self.zoom);
        if POINTS {
            let (lw, lh) = (self.skin_size.0 * self.user_zoom, self.skin_size.1 * self.user_zoom);
            let _ = self.window.request_inner_size(LogicalSize::new(lw as f64, lh as f64));
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
        ((x / self.zoom as f64).floor() as i32, (y / self.zoom as f64).floor() as i32)
    }

    /// Affiche l'image (agrandie au plus proche voisin) puis le texte.
    pub fn present(&mut self, img: &Image, text: &[TextRun]) {
        let (bw, bh) = (self.skin_size.0 * self.zoom, self.skin_size.1 * self.zoom);
        let (Some(nw), Some(nh)) = (NonZeroU32::new(bw), NonZeroU32::new(bh)) else { return };
        let Some((surface, _)) = self.soft.as_mut() else { return };
        let _ = surface.resize(nw, nh);
        let Ok(mut buf) = surface.buffer_mut() else { return };
        zoom_into(&mut buf, img, self.zoom.max(1), 0x00FF_FFFF);
        if !text.is_empty() {
            draw_runs(&mut buf, bw, bh, self.zoom.max(1) as f32, text);
        }
        let _ = buf.present();
    }
}

/// Agrandit `img` d'un facteur entier `z` dans `buf` (taille `img × z`), pixels masqués par `mask`.
pub fn zoom_into(buf: &mut [u32], img: &Image, z: u32, mask: u32) {
    let (bw, bh) = (img.width * z, img.height * z);
    for y in 0..bh {
        let sy = (y / z).min(img.height.saturating_sub(1));
        let src_row = &img.pixels[(sy * img.width) as usize..((sy + 1) * img.width) as usize];
        let dst_row = &mut buf[(y * bw) as usize..((y + 1) * bw) as usize];
        for (x, d) in dst_row.iter_mut().enumerate() {
            let sx = (x as u32 / z).min(img.width.saturating_sub(1));
            *d = src_row[sx as usize] & mask;
        }
    }
}

/// Image agrandie avec son canal alpha, et son texte : un calque pour la composition GPU.
pub fn zoomed_argb(img: &Image, z: u32, text: &[TextRun]) -> (u32, u32, Vec<u32>) {
    let (bw, bh) = (img.width * z, img.height * z);
    let mut buf = vec![0u32; (bw * bh) as usize];
    zoom_into(&mut buf, img, z, 0xFFFF_FFFF);
    if !text.is_empty() {
        draw_runs(&mut buf, bw, bh, z as f32, text);
    }
    (bw, bh, buf)
}

impl Drop for SkinWindow {
    fn drop(&mut self) {
        // La surface GPU référence la fenêtre : elle part d'abord.
        self.gpu.take();
    }
}
