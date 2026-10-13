//! Rendu hors écran des presets intégrés (passe si aucun GPU n'est disponible, ex. CI sans Vulkan).

use alpagamp_milkdrop::{Gpu, MilkDrop, PresetList, gpu};

fn tone(frame: u32) -> Vec<f32> {
    (0..735)
        .flat_map(|i| {
            let t = (frame as f32 * 735.0 + i as f32) / 44_100.0;
            let kick = (t * 55.0 * std::f32::consts::TAU).sin() * (-(t % 0.5) * 10.0).exp();
            let v = kick * 0.8 + (t * 660.0 * std::f32::consts::TAU).sin() * 0.2;
            [v, v * 0.9]
        })
        .collect()
}

#[test]
fn renders_every_builtin_preset() {
    let Ok(gpu) = Gpu::new(gpu::instance(), None, true) else {
        eprintln!("pas de GPU : test ignoré");
        return;
    };
    let mut md = MilkDrop::new(&gpu, 160, 120, 44_100, PresetList::builtin()).expect("moteur");
    md.shuffle = false;
    for i in 0..md.presets.len() {
        assert!(md.select(i, true), "preset {i} refusé");
        for f in 0..60 {
            md.feed(&tone(f));
            let mut enc = gpu.device.create_command_encoder(&Default::default());
            md.render(&mut enc);
            gpu.queue.submit(Some(enc.finish()));
        }
        let (w, h, px) = md.snapshot(&gpu);
        assert_eq!(px.len(), (w * h * 4) as usize);
        let lit = px.chunks(4).filter(|p| p[0] as u32 + p[1] as u32 + p[2] as u32 > 60).count();
        assert!(lit > 100, "{} : image presque noire ({lit} pixels allumés)", md.current().unwrap().name);
    }
    // Navigation : suivant / précédent / verrou.
    md.shuffle = true;
    let before = md.current_index();
    md.next(true);
    assert_ne!(md.current_index(), before);
    md.previous(true);
    assert_eq!(md.current_index(), before);
    assert!(md.toggle_lock());
}
