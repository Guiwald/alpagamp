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
