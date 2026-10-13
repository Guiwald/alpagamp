//! Simulateur de téléphone : l'appli Android dans une fenêtre de 412×915 (Pixel 8), la souris pour doigt.

#[cfg(not(target_os = "android"))]
fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,symphonia_bundle_mp3=error,symphonia_core=error")).init();
    let event_loop = winit::event_loop::EventLoop::new().expect("affichage");
    let mut m = alpagamp::mobile::Mobile::new(Box::new(alpagamp::host::SimHost));
    m.set_window_size(412, 915);
    event_loop.run_app(&mut m).expect("boucle");
}

#[cfg(target_os = "android")]
fn main() {}
