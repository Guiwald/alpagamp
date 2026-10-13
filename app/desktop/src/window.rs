//! Une fenêtre système sans bordure qui affiche une image skin agrandie au pixel près.

use std::num::NonZeroU32;
use std::rc::Rc;

use alpagamp_skin::Image;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowLevel};

pub struct SkinWindow {
    pub window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    _context: softbuffer::Context<Rc<Window>>,
    /// Facteur d'agrandissement entier (double taille × échelle de l'écran).
    pub zoom: u32,
    /// Taille en pixels skin.
    pub skin_size: (u32, u32),
}

impl SkinWindow {
    pub fn new(
        event_loop: &ActiveEventLoop,
        title: &str,
        skin_size: (u32, u32),
        position: Option<(i32, i32)>,
        on_top: bool,
        double: bool,
    ) -> Result<SkinWindow, String> {
        let mut attrs = WindowAttributes::default()
            .with_title(title)
            .with_decorations(false)
            .with_resizable(false)
            .with_visible(false)
            .with_inner_size(PhysicalSize::new(skin_size.0, skin_size.1));
        if let Some((x, y)) = position {
            attrs = attrs.with_position(PhysicalPosition::new(x, y));
        }
        if on_top {
            attrs = attrs.with_window_level(WindowLevel::AlwaysOnTop);
        }
        let icon = alpagamp_skin::app_icon(64);
        let rgba: Vec<u8> = icon.pixels.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, (p >> 24) as u8]).collect();
        if let Ok(i) = winit::window::Icon::from_rgba(rgba, 64, 64) {
            attrs = attrs.with_window_icon(Some(i));
        }
        let window = Rc::new(event_loop.create_window(attrs).map_err(|e| e.to_string())?);
        let context = softbuffer::Context::new(window.clone()).map_err(|e| e.to_string())?;
        let surface = softbuffer::Surface::new(&context, window.clone()).map_err(|e| e.to_string())?;
        let mut w = SkinWindow { window, surface, _context: context, zoom: 1, skin_size };
        w.update_zoom(double);
        w.window.set_visible(true);
        Ok(w)
    }

    /// Recalcule l'agrandissement (écran Retina, double taille) et redimensionne la fenêtre.
    pub fn update_zoom(&mut self, double: bool) {
        let sf = self.window.scale_factor();
        let zoom = ((sf.round().max(1.0)) as u32) * if double { 2 } else { 1 };
        self.zoom = zoom.max(1);
        self.apply_size();
    }

    pub fn set_skin_size(&mut self, size: (u32, u32)) {
        if size != self.skin_size {
            self.skin_size = size;
            self.apply_size();
        }
    }

    fn apply_size(&mut self) {
        let (w, h) = (self.skin_size.0 * self.zoom, self.skin_size.1 * self.zoom);
        let _ = self.window.request_inner_size(PhysicalSize::new(w, h));
        if let (Some(nw), Some(nh)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
            let _ = self.surface.resize(nw, nh);
        }
        self.window.request_redraw();
    }

    /// Coordonnées skin d'un point de la fenêtre (pixels physiques).
    pub fn to_skin(&self, x: f64, y: f64) -> (i32, i32) {
        ((x / self.zoom as f64).floor() as i32, (y / self.zoom as f64).floor() as i32)
    }

    /// Affiche l'image (agrandie au plus proche voisin).
    pub fn present(&mut self, img: &Image) {
        let size = self.window.inner_size();
        let (bw, bh) = (size.width.max(1), size.height.max(1));
        if let (Some(nw), Some(nh)) = (NonZeroU32::new(bw), NonZeroU32::new(bh)) {
            let _ = self.surface.resize(nw, nh);
        }
        let Ok(mut buf) = self.surface.buffer_mut() else { return };
        let z = self.zoom.max(1);
        for y in 0..bh {
            let sy = (y / z).min(img.height.saturating_sub(1));
            let src_row = &img.pixels[(sy * img.width) as usize..((sy + 1) * img.width) as usize];
            let dst_row = &mut buf[(y * bw) as usize..((y + 1) * bw) as usize];
            for (x, d) in dst_row.iter_mut().enumerate() {
                let sx = (x as u32 / z).min(img.width.saturating_sub(1));
                *d = src_row[sx as usize] & 0x00FF_FFFF;
            }
        }
        let _ = buf.present();
    }
}
