//! Plugins livrés et plugins qui se conduisent mal.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use alpagamp_plugins::{BUILTIN, Instance, Kind, Link, Plugins, Request, Sound};

fn builtin(id: &str) -> &'static [u8] {
    BUILTIN.iter().find(|(i, _)| *i == id).unwrap().1
}

#[test]
fn builtin_plugins_load_with_their_descriptions() {
    let p = Plugins::new();
    assert_eq!(p.catalog.len(), 3, "{:?}", p.broken);
    let kinds: Vec<Kind> = ["barres-plein-ecran", "enhancer", "minuterie-de-sommeil"].iter().map(|id| p.entry(id).unwrap().manifest.kind).collect();
    assert_eq!(kinds, [Kind::Vis, Kind::Dsp, Kind::General]);
    let e = p.entry("enhancer").unwrap();
    assert_eq!(e.manifest.options.len(), 3);
    assert_eq!(e.manifest.options[0].default_value(), 50);
    assert!(!p.entry("minuterie-de-sommeil").unwrap().manifest.description.get().is_empty());
}

#[test]
fn bars_draw_a_spectrum_fast_enough() {
    let link = Arc::new(Link::default());
    link.colors.lock()[2] = 0xFF0000;
    link.colors.lock()[17] = 0x00FF00;
    let mut p = Instance::load(builtin("barres-plein-ecran"), link, "barres").unwrap();
    let mut sound = Sound::default();
    for k in 0..576 {
        sound.spectrum[k] = 0.5 / (1.0 + k as f32 / 10.0);
        sound.spectrum[576 + k] = sound.spectrum[k];
    }
    let mut out = Vec::new();
    let t = Instant::now();
    for i in 0..30 {
        assert!(p.vis(320, 180, i * 16, &sound, &mut out), "{:?}", p.failed());
    }
    let per_frame = t.elapsed() / 30;
    eprintln!("barres : {per_frame:?} par image 320×180");
    assert_eq!(out.len(), 320 * 180);
    // Des diodes vertes (bas) allumées, et pas toute l'image de la même couleur.
    assert!(out.iter().any(|&c| c == 0xFF00FF00));
    assert!(out.iter().any(|&c| c != out[0]));
    // Option : couleurs « feu », plus de vert.
    p.set_option(1, 1);
    p.vis(320, 180, 600, &sound, &mut out);
    assert!(!out.iter().any(|&c| c == 0xFF00FF00));
}

#[test]
fn enhancer_processes_audio() {
    let mut p = Instance::load(builtin("enhancer"), Arc::new(Link::default()), "enhancer").unwrap();
    let mut s: Vec<f32> = (0..4096).flat_map(|i| {
        let x = (i as f32 * 0.05).sin() * 0.3;
        [x, -x * 0.5]
    }).collect();
    let orig = s.clone();
    let t = Instant::now();
    assert!(p.dsp(&mut s, 44_100), "{:?}", p.failed());
    eprintln!("enhancer : {:?} pour 4096 trames", t.elapsed());
    assert_ne!(s, orig);
    assert!(s.iter().all(|x| x.abs() <= 1.0));
    // Tout éteint : le son ressort presque identique.
    for (i, v) in [0, 0, 0].into_iter().enumerate() {
        p.set_option(i, v);
    }
    let mut s2 = orig.clone();
    p.dsp(&mut s2, 44_100);
    let diff = s2.iter().zip(&orig).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
    assert!(diff < 1e-3, "{diff}");
}

#[test]
fn sleep_timer_talks_to_the_player() {
    let link = Arc::new(Link::default());
    link.state.store(1, Ordering::Relaxed);
    let mut p = Instance::load(builtin("minuterie-de-sommeil"), link.clone(), "minuterie").unwrap();
    // 5 minutes, fondu 30 s.
    p.set_option(0, 5);
    p.set_option(1, 30);
    let mut t = 0i64;
    let mut reqs = Vec::new();
    while t <= 5 * 60_000 + 200 {
        assert!(p.tick(t), "{:?}", p.failed());
        // Le lecteur lit les demandes à chaque tic.
        reqs.extend(link.take_requests());
        t += 100;
    }
    assert!(matches!(&reqs[0], Request::Notify(m) if m.contains('5')), "{reqs:?}");
    assert!(reqs.iter().any(|r| matches!(r, Request::Volume(v) if *v < 20)));
    assert!(reqs.contains(&Request::Command(3)));
    // Volume rendu après l'arrêt.
    let last_volume = reqs.iter().rev().find_map(|r| if let Request::Volume(v) = r { Some(*v) } else { None });
    assert_eq!(last_volume, Some(200));
}

#[test]
fn quit_restores_the_volume_during_a_fade() {
    let link = Arc::new(Link::default());
    link.state.store(1, Ordering::Relaxed);
    let mut p = Instance::load(builtin("minuterie-de-sommeil"), link.clone(), "minuterie").unwrap();
    p.set_option(0, 5);
    p.set_option(1, 300);
    p.tick(0);
    p.tick(150_000);
    assert!(link.volume.load(Ordering::Relaxed) < 120);
    drop(p);
    assert_eq!(link.volume.load(Ordering::Relaxed), 200);
}

fn wat(src: &str) -> Vec<u8> {
    wat::parse_str(src).unwrap()
}

const INFO_DSP: &str = r#"(data (i32.const 1024) "{\"name\": \"Test\", \"kind\": \"dsp\"}\00")"#;

#[test]
fn an_endless_loop_is_stopped() {
    let m = wat(&format!(
        r#"(module (memory (export "memory") 1) {INFO_DSP}
            (func (export "alp_api") (result i32) i32.const 1)
            (func (export "alp_info") (result i32) i32.const 1024)
            (func (export "alp_alloc") (param i32) (result i32) i32.const 4096)
            (func (export "alp_dsp") (param i32 i32 i32) (loop br 0)))"#
    ));
    let mut p = Instance::load(&m, Arc::new(Link::default()), "boucle").unwrap();
    let mut s = vec![0.5f32; 1024];
    let t = Instant::now();
    assert!(!p.dsp(&mut s, 44_100));
    assert!(p.failed().unwrap().contains("trop lent"));
    eprintln!("boucle arrêtée en {:?}", t.elapsed());
    assert!(t.elapsed().as_secs() < 5);
}

#[test]
fn bad_plugins_are_refused() {
    // Pas un module.
    assert!(Instance::load(b"pas du wasm", Arc::new(Link::default()), "x").is_err());
    // Mauvaise version d'API.
    let m = wat(&format!(
        r#"(module (memory (export "memory") 1) {INFO_DSP}
            (func (export "alp_api") (result i32) i32.const 7)
            (func (export "alp_info") (result i32) i32.const 1024)
            (func (export "alp_alloc") (param i32) (result i32) i32.const 4096))"#
    ));
    assert!(Instance::load(&m, Arc::new(Link::default()), "x").unwrap_err().contains("version"));
    // DSP sans alp_dsp.
    let m = wat(&format!(
        r#"(module (memory (export "memory") 1) {INFO_DSP}
            (func (export "alp_api") (result i32) i32.const 1)
            (func (export "alp_info") (result i32) i32.const 1024)
            (func (export "alp_alloc") (param i32) (result i32) i32.const 4096))"#
    ));
    assert!(Instance::load(&m, Arc::new(Link::default()), "x").unwrap_err().contains("alp_dsp"));
    // Import inconnu (accès disque, par exemple).
    let m = wat(r#"(module (import "wasi" "fd_write" (func (param i32))) (memory (export "memory") 1))"#);
    assert!(Instance::load(&m, Arc::new(Link::default()), "x").is_err());
    // Trop de mémoire demandée (128 Mo).
    let m = wat(&format!(
        r#"(module (memory (export "memory") 2048) {INFO_DSP}
            (func (export "alp_api") (result i32) i32.const 1))"#
    ));
    assert!(Instance::load(&m, Arc::new(Link::default()), "x").is_err());
}

#[test]
fn a_crash_and_a_bad_pointer_are_contained() {
    let m = wat(r#"(module (memory (export "memory") 1)
            (data (i32.const 1024) "{\"name\": \"V\", \"kind\": \"vis\"}\00")
            (func (export "alp_api") (result i32) i32.const 1)
            (func (export "alp_info") (result i32) i32.const 1024)
            (func (export "alp_alloc") (param i32) (result i32) i32.const 8192)
            (func (export "alp_vis") (param i32 i32 i32 i32 i32) (result i32) i32.const 60000))"#);
    let mut p = Instance::load(&m, Arc::new(Link::default()), "v").unwrap();
    let mut out = Vec::new();
    assert!(!p.vis(64, 64, 0, &Sound::default(), &mut out));
    assert!(p.failed().unwrap().contains("hors"));
    let m = wat(r#"(module (memory (export "memory") 1)
            (data (i32.const 1024) "{\"name\": \"V\", \"kind\": \"general\"}\00")
            (func (export "alp_api") (result i32) i32.const 1)
            (func (export "alp_info") (result i32) i32.const 1024)
            (func (export "alp_alloc") (param i32) (result i32) i32.const 8192)
            (func (export "alp_tick") (param i64) unreachable))"#);
    let mut p = Instance::load(&m, Arc::new(Link::default()), "g").unwrap();
    assert!(!p.tick(0));
    assert!(p.failed().is_some());
}
