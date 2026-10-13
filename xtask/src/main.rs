//! Tâches de développement d'Alpagamp : `cargo xtask <commande>`.

use std::path::{Path, PathBuf};

use alpagamp_skin::{Image, Sheet, Skin};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let res = match cmd {
        "skin-sheets" => skin_sheets(args.get(1).map(PathBuf::from), args.get(2).map(PathBuf::from)),
        "export-default-skin" => export_default(args.get(1).map(PathBuf::from)),
        "render-main" => render_main(args.get(1).map(PathBuf::from), args.get(2).map(PathBuf::from)),
        _ => {
            eprintln!(
                "Commandes :\n  skin-sheets <dossier> [skin.wsz]   écrit chaque planche en PNG (×3)\n  export-default-skin <fichier.wsz>  exporte le skin par défaut"
            );
            Ok(())
        }
    };
    if let Err(e) = res {
        eprintln!("erreur : {e}");
        std::process::exit(1);
    }
}

type Res = Result<(), Box<dyn std::error::Error>>;

fn skin_sheets(out: Option<PathBuf>, wsz: Option<PathBuf>) -> Res {
    let out = out.ok_or("dossier de sortie manquant")?;
    std::fs::create_dir_all(&out)?;
    let def = Skin::default_skin();
    let skin = match wsz {
        Some(p) => Skin::load_wsz(&p.display().to_string(), &std::fs::read(&p)?, &def)?,
        None => def,
    };
    for sheet in Sheet::ALL {
        let img = skin.sheet(sheet);
        write_png(&out.join(format!("{}.png", sheet.file_name().trim_end_matches(".bmp"))), img, 3)?;
    }
    println!("{} planches écrites dans {}", Sheet::ALL.len(), out.display());
    Ok(())
}

/// Rend la fenêtre principale (normale et bandeau) avec une lecture fictive, en PNG ×3.
fn render_main(out: Option<PathBuf>, wsz: Option<PathBuf>) -> Res {
    use alpagamp_ui::{MainView, MainWindow, Transport};
    let out = out.ok_or("fichier de sortie manquant")?;
    let def = Skin::default_skin();
    let skin = match wsz {
        Some(p) => Skin::load_wsz("x", &std::fs::read(&p)?, &def)?,
        None => def,
    };
    let view = MainView {
        transport: Transport::Playing,
        position_ms: 83_000,
        duration_ms: Some(296_000),
        seekable: true,
        title: "3. Alpagamp - Chanson de test (4:56)".into(),
        kbps: Some(320),
        khz: Some(44),
        stereo: Some(true),
        volume: 200,
        balance: -40,
        eq_open: true,
        pl_open: false,
        shuffle: true,
        active: true,
        ..Default::default()
    };
    let mut w = MainWindow::new();
    // Signal fictif : quelques sinus pour animer l'analyseur.
    let n = 576;
    let sig: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / 44_100.0;
            0.4 * (t * 110.0 * std::f32::consts::TAU).sin() + 0.2 * (t * 880.0 * std::f32::consts::TAU).sin() + 0.08 * (t * 5000.0 * std::f32::consts::TAU).sin()
        })
        .collect();
    w.vis.analyze(&sig, &sig);
    w.vis.advance(0.1, &view.vis);
    let mut fb = Image::new(1, 1, 0);
    w.render(&mut fb, &skin, &view);
    write_png(&out, &fb, 3)?;
    w.shade = true;
    w.render(&mut fb, &skin, &view);
    let shade_path = out.with_file_name(format!("{}-shade.png", out.file_stem().unwrap().to_string_lossy()));
    write_png(&shade_path, &fb, 3)?;
    println!("{} et {}", out.display(), shade_path.display());
    Ok(())
}

fn export_default(path: Option<PathBuf>) -> Res {
    let path = path.ok_or("fichier de sortie manquant")?;
    std::fs::write(&path, Skin::default_skin().to_wsz())?;
    println!("skin par défaut exporté : {}", path.display());
    Ok(())
}

/// Écrit une image en PNG, agrandie `scale` fois au plus proche voisin.
pub fn write_png(path: &Path, img: &Image, scale: u32) -> Res {
    let (w, h) = (img.width * scale, img.height * scale);
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let p = img.pixels[((y / scale) * img.width + x / scale) as usize];
            data.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, (p >> 24) as u8]);
        }
    }
    let file = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&data)?;
    Ok(())
}
