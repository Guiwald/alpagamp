//! Alpagamp pour Android : la fenêtre principale, l'égaliseur et la playlist empilés à la largeur de l'écran
//! (mode fidèle, ADR 0006), pilotés au toucher. Sur ordinateur, la même appli tourne en simulateur
//! (`cargo run -p alpagamp-android --bin phone-sim`) pour le développement.

pub mod host;
pub mod mobile;
#[cfg(target_os = "android")]
mod platform;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("alpagamp"));
    std::panic::set_hook(Box::new(|info| log::error!("panique : {info}")));
    let event_loop = match winit::event_loop::EventLoop::builder().with_android_app(app.clone()).build() {
        Ok(e) => e,
        Err(e) => {
            log::error!("boucle d'événements : {e}");
            return;
        }
    };
    let mut m = mobile::Mobile::new(Box::new(host::AndroidHost(app)));
    if let Err(e) = event_loop.run_app(&mut m) {
        log::error!("{e}");
    }
}
