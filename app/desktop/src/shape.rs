//! Formes de fenêtres des skins (`region.txt`, ADR 0016) : la fenêtre principale et l'égaliseur prennent la
//! forme des polygones du skin.
//!
//! - **Windows** : `SetWindowRgn` (région de la fenêtre : les clics hors de la forme passent au travers).
//! - **macOS** : fenêtre transparente et masque `CAShapeLayer` sur le calque de la vue.
//! - Ailleurs (Linux, développement) : fenêtres rectangulaires.

use alpagamp_skin::config::Polygons;
use winit::window::Window;

/// Les polygones d'un `region.txt` s'additionnent (Winamp remplit en « winding ») ; certains se recouvrent
/// (coins arrondis : deux rectangles décalés d'un pixel). On les oriente tous dans le même sens pour que la règle
/// « non nul » donne leur union.
fn oriented(polys: &Polygons) -> Polygons {
    polys
        .iter()
        .map(|p| {
            let area: i64 = p.iter().zip(p.iter().cycle().skip(1)).map(|(a, b)| a.0 as i64 * b.1 as i64 - b.0 as i64 * a.1 as i64).sum();
            if area < 0 { p.iter().rev().copied().collect() } else { p.clone() }
        })
        .collect()
}

/// Applique `polys` (coordonnées en pixels skin) à la fenêtre ; `None` : rectangulaire.
/// `phys` : pixels physiques par pixel skin ; `points` : points par pixel skin (macOS) ; `height` : hauteur skin.
#[allow(unused_variables)]
pub fn set_shape(window: &Window, polys: Option<&Polygons>, phys: u32, points: u32, size: (u32, u32)) {
    let polys = polys.map(oriented);
    let polys = polys.as_ref();
    #[cfg(target_os = "windows")]
    windows::set(window, polys, phys);
    #[cfg(target_os = "macos")]
    mac::set(window, polys, points as f64, ((size.0 * points) as f64, (size.1 * points) as f64));
}

#[cfg(target_os = "windows")]
mod windows {
    use alpagamp_skin::config::Polygons;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::{HWND, POINT};
    use windows_sys::Win32::Graphics::Gdi::{CreatePolyPolygonRgn, SetWindowRgn, WINDING};
    use winit::window::Window;

    pub fn set(window: &Window, polys: Option<&Polygons>, zoom: u32) {
        let Ok(h) = window.window_handle() else { return };
        let RawWindowHandle::Win32(h) = h.as_raw() else { return };
        let hwnd = h.hwnd.get() as HWND;
        let z = zoom.max(1) as i32;
        unsafe {
            match polys {
                Some(polys) if !polys.is_empty() => {
                    let points: Vec<POINT> = polys.iter().flatten().map(|&(x, y)| POINT { x: x * z, y: y * z }).collect();
                    let counts: Vec<i32> = polys.iter().map(|p| p.len() as i32).collect();
                    let rgn = CreatePolyPolygonRgn(points.as_ptr(), counts.as_ptr(), counts.len() as i32, WINDING);
                    if !rgn.is_null() {
                        // La région appartient ensuite au système.
                        SetWindowRgn(hwnd, rgn, 1);
                    }
                }
                _ => {
                    SetWindowRgn(hwnd, std::ptr::null_mut(), 1);
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use alpagamp_skin::config::Polygons;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSColor, NSView};
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_core_graphics::CGMutablePath;
    use objc2_quartz_core::{CAShapeLayer, kCAFillRuleNonZero};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use winit::window::Window;

    pub fn set(window: &Window, polys: Option<&Polygons>, scale: f64, (width, height): (f64, f64)) {
        if MainThreadMarker::new().is_none() {
            return;
        }
        let Ok(h) = window.window_handle() else { return };
        let RawWindowHandle::AppKit(h) = h.as_raw() else { return };
        // SAFETY : la poignée vient de winit, qui garantit un `NSView` valide.
        let view: &NSView = unsafe { h.ns_view.cast().as_ref() };
        view.setWantsLayer(true);
        let Some(layer) = view.layer() else { return };
        match polys {
            Some(polys) if !polys.is_empty() => {
                if let Some(win) = view.window() {
                    win.setOpaque(false);
                    win.setBackgroundColor(Some(&NSColor::clearColor()));
                    win.setHasShadow(false);
                }
                let flipped = layer.isGeometryFlipped();
                let path = CGMutablePath::new();
                for poly in polys {
                    for (i, &(x, y)) in poly.iter().enumerate() {
                        let px = x as f64 * scale;
                        let py = if flipped { y as f64 * scale } else { height - y as f64 * scale };
                        // SAFETY : pas de transformation (pointeur nul autorisé).
                        unsafe {
                            if i == 0 {
                                CGMutablePath::move_to_point(Some(&path), std::ptr::null(), px, py);
                            } else {
                                CGMutablePath::add_line_to_point(Some(&path), std::ptr::null(), px, py);
                            }
                        }
                    }
                    CGMutablePath::close_subpath(Some(&path));
                }
                let mask = CAShapeLayer::new();
                mask.setFrame(CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(width, height)));
                mask.setPath(Some(&path));
                // SAFETY : constante de Core Animation.
                mask.setFillRule(unsafe { kCAFillRuleNonZero });
                // SAFETY : le masque est un calque neuf, sans parent.
                unsafe { layer.setMask(Some(&mask)) };
            }
            _ => unsafe { layer.setMask(None) },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orients_polygons_the_same_way() {
        let cw = vec![(0, 0), (10, 0), (10, 10), (0, 10)];
        let ccw: Vec<(i32, i32)> = cw.iter().rev().copied().collect();
        let o = oriented(&vec![cw.clone(), ccw]);
        assert_eq!(o[0], cw);
        assert_eq!(o[1], cw);
    }
}
