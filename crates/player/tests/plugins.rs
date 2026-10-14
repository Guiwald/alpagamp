//! Les plugins vus du lecteur : activation, options, effets branchés sur le moteur, minuterie.

use alpagamp_core::Settings;
use alpagamp_player::Player;

fn player() -> (Player, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("alpagamp-plugins-{}-{:?}", std::process::id(), std::thread::current().id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut p = Player::new(Settings::default());
    p.init_plugins(dir.clone());
    (p, dir)
}

#[test]
fn enabling_the_enhancer_plugs_an_effect() {
    let (mut p, _) = player();
    assert_eq!(p.engine.effect_count(), 0);
    p.set_plugin_enabled("enhancer", true);
    assert!(p.plugin_enabled("enhancer"));
    assert_eq!(p.engine.effect_count(), 1);
    p.set_plugin_option("enhancer", 1, 9);
    assert_eq!(p.plugin_option("enhancer", 1), Some(9));
    assert_eq!(p.settings.plugins.options["enhancer"]["bass"], 9);
    // Valeur non prévue : ignorée.
    p.set_plugin_option("enhancer", 1, 42);
    assert_eq!(p.plugin_option("enhancer", 1), Some(9));
    p.set_plugin_enabled("enhancer", false);
    assert_eq!(p.engine.effect_count(), 0);
}

#[test]
fn one_visualization_at_a_time() {
    let (mut p, _) = player();
    p.set_plugin_enabled("barres-plein-ecran", true);
    assert_eq!(p.settings.plugins.vis.as_deref(), Some("barres-plein-ecran"));
    assert!(p.plugins.vis_running());
    let mut px = Vec::new();
    assert!(p.plugins.vis_frame(64, 32, &p.engine, &mut px));
    assert_eq!(px.len(), 64 * 32);
    p.set_plugin_enabled("barres-plein-ecran", false);
    assert!(!p.plugins.vis_running());
    assert!(p.settings.plugins.vis.is_none());
}

#[test]
fn the_sleep_timer_is_not_remembered() {
    let mut s = Settings::default();
    s.plugins.enabled = vec!["minuterie-de-sommeil".into(), "enhancer".into()];
    let mut p = Player::new(s);
    p.init_plugins(std::env::temp_dir().join("alpagamp-plugins-vide"));
    assert_eq!(p.settings.plugins.enabled, vec!["enhancer".to_string()]);
    // Activée à la main : le compte à rebours part et s'annonce dans le titre.
    p.set_plugin_enabled("minuterie-de-sommeil", true);
    p.tick();
    let title = p.main_view(0).title;
    assert!(title.contains("30"), "{title}");
}

#[test]
fn a_user_plugin_overrides_and_bad_files_are_listed() {
    let (mut p, dir) = player();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Cassé.wasm"), b"pas du wasm").unwrap();
    let src = dir.join("source");
    std::fs::create_dir_all(&src).unwrap();
    let enh = alpagamp_plugins::BUILTIN.iter().find(|(i, _)| *i == "enhancer").unwrap().1;
    std::fs::write(src.join("Mon Effet.wasm"), enh).unwrap();
    p.rescan_plugins();
    assert_eq!(p.plugins.broken.len(), 1);
    let id = p.install_plugin(&src.join("Mon Effet.wasm")).unwrap();
    assert_eq!(id, "mon-effet");
    assert!(dir.join("mon-effet.wasm").exists());
    assert!(p.plugin_enabled("mon-effet"));
    assert!(p.install_plugin(&dir.join("Cassé.wasm")).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
