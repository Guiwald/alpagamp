//! Icône et informations de version de l'exécutable Windows.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icons/alpagamp.rc");
    println!("cargo:rerun-if-changed=../../assets/icons/alpagamp.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let _ = embed_resource::compile("../../assets/icons/alpagamp.rc", embed_resource::NONE);
    }
}
