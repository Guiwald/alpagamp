//! Petits appels à AppKit (macOS) : zones utiles des écrans et ordre des fenêtres. Liaisons Rust (objc2), sans
//! code C.

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSScreen, NSView, NSWindow, NSWindowOrderingMode};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::layout::R;
use crate::window::PER_POINT;

fn ns_window(window: &Window) -> Option<Retained<NSWindow>> {
    let h = window.window_handle().ok()?;
    let RawWindowHandle::AppKit(h) = h.as_raw() else { return None };
    // SAFETY : la poignée vient de winit, qui garantit un `NSView` valide sur le fil principal.
    let view: &NSView = unsafe { h.ns_view.cast().as_ref() };
    view.window()
}

/// Place `window` juste derrière `front`, sans l'activer.
pub fn order_below(window: &Window, front: &Window) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    let (Some(w), Some(f)) = (ns_window(window), ns_window(front)) else { return };
    w.orderWindow_relativeTo(NSWindowOrderingMode::Below, f.windowNumber());
}

/// Zones utiles des écrans (sans la barre de menus ni le Dock), en unités de disposition : demi-points, origine en
/// haut à gauche de l'écran principal, comme les positions de winit.
pub fn work_areas() -> Vec<R> {
    let Some(mtm) = MainThreadMarker::new() else { return Vec::new() };
    let screens = NSScreen::screens(mtm).to_vec();
    // L'écran d'indice 0 porte la barre de menus : c'est lui qui définit l'origine des coordonnées.
    let Some(main_h) = screens.first().map(|s| s.frame().size.height) else { return Vec::new() };
    let u = |v: f64| (v * PER_POINT).round() as i32;
    screens
        .iter()
        .map(|s| {
            let v = s.visibleFrame();
            let top = main_h - (v.origin.y + v.size.height);
            R::new(u(v.origin.x), u(top), u(v.size.width), u(v.size.height))
        })
        .collect()
}
