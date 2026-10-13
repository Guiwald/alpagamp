//! Kit pour écrire un plugin Alpagamp en Rust (API v1, voir `docs/adr/0020-api-des-plugins.md`).
//!
//! Un plugin est une crate `cdylib` compilée pour `wasm32-unknown-unknown`. On décrit le plugin (`Plugin`), on
//! écrit ce qu'il fait (`Vis`, `Dsp` ou `General`), puis une macro fabrique les fonctions exportées :
//!
//! ```ignore
//! use alpagamp_plugin::{General, Plugin, host};
//!
//! struct Bonjour;
//! impl Plugin for Bonjour {
//!     const INFO: &'static str = r#"{"name": "Bonjour", "kind": "general", "author": "Moi", "version": "1.0"}"#;
//!     fn new() -> Self { host::notify("Bonjour !"); Bonjour }
//! }
//! impl General for Bonjour {
//!     fn tick(&mut self, _ms: i64) {}
//! }
//! alpagamp_plugin::general_plugin!(Bonjour);
//! ```

use std::cell::RefCell;
use std::sync::OnceLock;

/// Ce que tout plugin fournit.
pub trait Plugin: 'static {
    /// Description JSON du plugin (nom, sorte, auteur, version, description, options).
    const INFO: &'static str;
    /// Crée le plugin (appelé une fois, au chargement).
    fn new() -> Self;
    /// Option n° `index` (rang dans la liste `options`) réglée à `value`. Appelé au chargement pour chaque
    /// option, puis à chaque changement.
    fn option(&mut self, index: u32, value: i32) {
        let _ = (index, value);
    }
    /// Le plugin va être arrêté (désactivé, ou fermeture du lecteur).
    fn quit(&mut self) {}
}

/// Une image à dessiner (pixels `0xAARRGGBB`, ligne par ligne).
pub struct Image<'a> {
    pub width: usize,
    pub height: usize,
    pub pixels: &'a mut [u32],
}

impl Image<'_> {
    pub fn fill(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    /// Rectangle plein, découpé aux bords de l'image.
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        let x0 = x.max(0) as usize;
        let y0 = y.max(0) as usize;
        let x1 = (x + w).clamp(0, self.width as i32) as usize;
        let y1 = (y + h).clamp(0, self.height as i32) as usize;
        for row in y0..y1 {
            let base = row * self.width;
            if x0 < x1 {
                self.pixels[base + x0..base + x1].fill(color);
            }
        }
    }

    pub fn set(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.pixels[y as usize * self.width + x as usize] = color;
        }
    }
}

/// Le son à visualiser : 576 valeurs par voie (gauche, droite).
pub struct Sound<'a> {
    /// Amplitude 0–1 par bande de fréquence linéaire (bande `k` ≈ `k × 44100 / 1152` Hz).
    pub spectrum: [&'a [f32]; 2],
    /// Forme d'onde, entre −1 et 1.
    pub wave: [&'a [f32]; 2],
}

/// Une visualisation : dessine une image à chaque trame.
pub trait Vis: Plugin {
    /// `time_ms` : temps écoulé depuis le chargement.
    fn render(&mut self, image: &mut Image, time_ms: u32, sound: &Sound);
}

/// Un effet audio : transforme le son en place.
pub trait Dsp: Plugin {
    /// `samples` : échantillons stéréo entrelacés (gauche, droite, gauche…), entre −1 et 1.
    fn process(&mut self, samples: &mut [f32], rate: u32);
}

/// Un plugin général : réagit au temps et pilote le lecteur.
pub trait General: Plugin {
    /// Appelé dix fois par seconde ; `time_ms` : temps écoulé depuis le chargement.
    fn tick(&mut self, time_ms: i64);
}

/// Ce que le lecteur offre aux plugins.
pub mod host {
    #[cfg(target_arch = "wasm32")]
    mod ffi {
        #[link(wasm_import_module = "alpagamp")]
        unsafe extern "C" {
            pub fn log(ptr: *const u8, len: i32);
            pub fn notify(ptr: *const u8, len: i32);
            pub fn player_state() -> i32;
            pub fn command(c: i32);
            pub fn volume() -> i32;
            pub fn set_volume(v: i32);
            pub fn skin_color(i: i32) -> i32;
            pub fn lang(ptr: *mut u8, cap: i32) -> i32;
        }
    }

    /// Hors WebAssembly (tests sur l'ordinateur) : un lecteur imaginaire.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::missing_safety_doc)]
    mod ffi {
        use std::cell::Cell;
        thread_local! {
            pub static VOLUME: Cell<i32> = const { Cell::new(200) };
            pub static STATE: Cell<i32> = const { Cell::new(1) };
        }
        pub unsafe fn log(_: *const u8, _: i32) {}
        pub unsafe fn notify(_: *const u8, _: i32) {}
        pub unsafe fn player_state() -> i32 {
            STATE.with(Cell::get)
        }
        pub unsafe fn command(c: i32) {
            STATE.with(|s| s.set(match c {
                1 => 1,
                2 => 2,
                3 => 0,
                _ => s.get(),
            }));
        }
        pub unsafe fn volume() -> i32 {
            VOLUME.with(Cell::get)
        }
        pub unsafe fn set_volume(v: i32) {
            VOLUME.with(|c| c.set(v));
        }
        pub unsafe fn skin_color(i: i32) -> i32 {
            (i * 10) << 8
        }
        pub unsafe fn lang(ptr: *mut u8, cap: i32) -> i32 {
            if cap >= 2 {
                unsafe { std::ptr::copy_nonoverlapping(b"fr".as_ptr(), ptr, 2) };
            }
            2
        }
    }

    /// État de la lecture.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum State {
        Stopped,
        Playing,
        Paused,
    }

    /// Ordres possibles au lecteur.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Command {
        Play = 1,
        Pause = 2,
        Stop = 3,
        Next = 4,
        Previous = 5,
    }

    /// Écrit dans le journal du lecteur.
    pub fn log(msg: &str) {
        unsafe { ffi::log(msg.as_ptr(), msg.len() as i32) }
    }

    /// Affiche un message dans le titre défilant.
    pub fn notify(msg: &str) {
        unsafe { ffi::notify(msg.as_ptr(), msg.len() as i32) }
    }

    pub fn state() -> State {
        match unsafe { ffi::player_state() } {
            1 => State::Playing,
            2 => State::Paused,
            _ => State::Stopped,
        }
    }

    pub fn command(c: Command) {
        unsafe { ffi::command(c as i32) }
    }

    /// Volume 0–255.
    pub fn volume() -> u8 {
        unsafe { ffi::volume() }.clamp(0, 255) as u8
    }

    pub fn set_volume(v: u8) {
        unsafe { ffi::set_volume(v as i32) }
    }

    /// Couleur n° `i` (0–23) du visualiseur du skin, `0xFFRRGGBB`. 0 : fond ; 2 (haut) à 17 (bas) : analyseur ;
    /// 18–22 : oscilloscope ; 23 : crêtes.
    pub fn skin_color(i: u32) -> u32 {
        0xFF00_0000 | unsafe { ffi::skin_color(i as i32) } as u32
    }

    /// Code de la langue de l'interface (`fr`, `en`, `es`, `de`, `it`, `ca-valencia`, `ga`).
    pub fn lang() -> String {
        let mut buf = [0u8; 16];
        let n = unsafe { ffi::lang(buf.as_mut_ptr(), buf.len() as i32) }.clamp(0, 16) as usize;
        String::from_utf8_lossy(&buf[..n]).into_owned()
    }

    /// Choisit le texte de la langue de l'interface parmi `[en, fr, es, de, it, ca, ga]`.
    pub fn tr<'a>(texts: [&'a str; 7]) -> &'a str {
        let l = lang();
        let i = match l.split('-').next().unwrap_or("") {
            "fr" => 1,
            "es" => 2,
            "de" => 3,
            "it" => 4,
            "ca" | "va" => 5,
            "ga" => 6,
            _ => 0,
        };
        texts[i]
    }
}

/// Couleur opaque.
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    0xFF00_0000 | (r as u32) << 16 | (g as u32) << 8 | b as u32
}

/// Mélange de deux couleurs (`t` entre 0 et 1).
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |s: u32| {
        let x = ((a >> s) & 0xFF) as f32;
        let y = ((b >> s) & 0xFF) as f32;
        ((x + (y - x) * t).round() as u32) << s
    };
    0xFF00_0000 | ch(16) | ch(8) | ch(0)
}

// --- Mécanique interne utilisée par les macros. ---

#[doc(hidden)]
pub fn __info(info: &'static str) -> i32 {
    static C: OnceLock<usize> = OnceLock::new();
    *C.get_or_init(|| {
        let s: &'static mut str = Box::leak(format!("{info}\0").into_boxed_str());
        s.as_ptr() as usize
    }) as i32
}

#[doc(hidden)]
pub fn __alloc(bytes: i32) -> i32 {
    // Alignée sur 8 octets : l'hôte y écrit des `f32`.
    let mut v: Vec<u64> = Vec::with_capacity((bytes.max(0) as usize).div_ceil(8));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p as usize as i32
}

thread_local! {
    static IMAGE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

#[doc(hidden)]
pub fn __vis<T: Vis>(p: &mut T, w: i32, h: i32, time_ms: i32, spectrum: i32, wave: i32) -> i32 {
    let (w, h) = (w.clamp(1, 4096) as usize, h.clamp(1, 4096) as usize);
    // SAFETY : l'hôte a écrit 2 × 576 f32 à chacune de ces adresses, réservées par `alp_alloc`.
    let (s, o) = unsafe {
        (std::slice::from_raw_parts(spectrum as usize as *const f32, 1152), std::slice::from_raw_parts(wave as usize as *const f32, 1152))
    };
    let sound = Sound { spectrum: [&s[..576], &s[576..]], wave: [&o[..576], &o[576..]] };
    IMAGE.with(|img| {
        let mut img = img.borrow_mut();
        img.resize(w * h, 0);
        let mut image = Image { width: w, height: h, pixels: &mut img[..] };
        p.render(&mut image, time_ms as u32, &sound);
        img.as_ptr() as usize as i32
    })
}

#[doc(hidden)]
pub fn __dsp<T: Dsp>(p: &mut T, ptr: i32, frames: i32, rate: i32) {
    // SAFETY : l'hôte a écrit `frames` trames stéréo à cette adresse, réservée par `alp_alloc`.
    let s = unsafe { std::slice::from_raw_parts_mut(ptr as usize as *mut f32, frames.max(0) as usize * 2) };
    p.process(s, rate.max(1) as u32);
}

#[doc(hidden)]
#[macro_export]
macro_rules! __common {
    ($t:ty) => {
        thread_local! {
            static __ALP: ::core::cell::RefCell<Option<$t>> = const { ::core::cell::RefCell::new(None) };
        }
        fn __alp_with<R>(f: impl FnOnce(&mut $t) -> R) -> R {
            __ALP.with(|c| {
                let mut b = c.borrow_mut();
                f(b.get_or_insert_with(<$t as $crate::Plugin>::new))
            })
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn alp_api() -> i32 {
            1
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn alp_info() -> i32 {
            $crate::__info(<$t as $crate::Plugin>::INFO)
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn alp_alloc(bytes: i32) -> i32 {
            $crate::__alloc(bytes)
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn alp_init() {
            __alp_with(|_| ())
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn alp_option(index: i32, value: i32) {
            __alp_with(|p| $crate::Plugin::option(p, index.max(0) as u32, value))
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn alp_quit() {
            __alp_with(|p| $crate::Plugin::quit(p))
        }
    };
}

/// Fabrique les exports d'une visualisation.
#[macro_export]
macro_rules! vis_plugin {
    ($t:ty) => {
        $crate::__common!($t);
        #[unsafe(no_mangle)]
        pub extern "C" fn alp_vis(w: i32, h: i32, time_ms: i32, spectrum: i32, wave: i32) -> i32 {
            __alp_with(|p| $crate::__vis(p, w, h, time_ms, spectrum, wave))
        }
    };
}

/// Fabrique les exports d'un effet audio.
#[macro_export]
macro_rules! dsp_plugin {
    ($t:ty) => {
        $crate::__common!($t);
        #[unsafe(no_mangle)]
        pub extern "C" fn alp_dsp(ptr: i32, frames: i32, rate: i32) {
            __alp_with(|p| $crate::__dsp(p, ptr, frames, rate))
        }
    };
}

/// Fabrique les exports d'un plugin général.
#[macro_export]
macro_rules! general_plugin {
    ($t:ty) => {
        $crate::__common!($t);
        #[unsafe(no_mangle)]
        pub extern "C" fn alp_tick(time_ms: i64) {
            __alp_with(|p| $crate::General::tick(p, time_ms))
        }
    };
}
