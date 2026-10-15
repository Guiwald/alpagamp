//! Alpagamp — lecteur de musique à skins classiques.

#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app;
mod command;
mod dialogs;
mod keys;
mod layout;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod menu;
mod paths;
mod shape;
mod window;

use std::path::PathBuf;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,symphonia_bundle_mp3=error,symphonia_core=error")).init();
    let args: Vec<PathBuf> = std::env::args_os().skip(1).filter(|a| !a.to_string_lossy().starts_with("-psn_")).map(PathBuf::from).collect();
    // `alpagamp --diagnostic-spotify [rapport.txt]` : diagnostic de Spotify sans ouvrir de fenêtre (ADR 0022).
    if args.first().is_some_and(|a| a.as_os_str() == "--diagnostic-spotify") {
        let report = alpagamp_spotify::diagnostic::run(&paths::Paths::new().config_dir.join("spotify"));
        match args.get(1) {
            Some(out) => {
                if let Err(e) = std::fs::write(out, &report) {
                    eprintln!("Alpagamp : {}: {e}", out.display());
                    std::process::exit(1);
                }
            }
            None => print!("{report}"),
        }
        return;
    }
    let event_loop = match winit::event_loop::EventLoop::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Alpagamp : impossible d'ouvrir l'affichage ({e})");
            std::process::exit(1);
        }
    };
    let mut app = app::App::new(args);
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("Alpagamp : {e}");
    }
}
