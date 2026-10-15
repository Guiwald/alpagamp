//! Tâches de développement d'Alpagamp : `cargo xtask <commande>`.

mod apk;
mod changelog_page;

use std::path::{Path, PathBuf};

use alpagamp_skin::{Image, Sheet, Skin};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let res = match cmd {
        "skin-sheets" => skin_sheets(args.get(1).map(PathBuf::from), args.get(2).map(PathBuf::from)),
        "export-default-skin" => export_default(args.get(1).map(PathBuf::from)),
        "icons" => icons(args.get(1).map(PathBuf::from)),
        "changelog-page" => changelog(args.get(1).map(PathBuf::from)),
        "bump" => bump(args.get(1).map(String::as_str)),
        "dist-windows" => dist_windows(args.get(1).map(PathBuf::from)),
        "bundle-mac" => bundle_mac(args.get(1).map(PathBuf::from), args.get(2).map(PathBuf::from)),
        "render-main" => render_main(args.get(1).map(PathBuf::from), args.get(2).map(PathBuf::from)),
        "render-windows" => render_windows(args.get(1).map(PathBuf::from), args.get(2).map(PathBuf::from)),
        "apk" => apk::build_apk(),
        "plugins" => plugins(),
        _ => {
            eprintln!(
                "Commandes :\n  skin-sheets <dossier> [skin.wsz]       écrit chaque planche en PNG (×3)\n  export-default-skin <fichier.wsz>      exporte le skin par défaut\n  render-main <sortie.png> [skin.wsz]    aperçu de la fenêtre principale\n  render-windows <sortie.png> [skin.wsz] aperçu des trois fenêtres et de la fenêtre générique\n  icons <dossier>                        icônes PNG, .ico, .icns\n  dist-windows [alpagamp.exe]            paquet zip Windows dans dist/\n  bundle-mac <binaire> [dossier]         Alpagamp.app (+ .dmg sur macOS)\n  apk                                    APK Android signé dans dist/ (SDK, NDK)\n  plugins                                plugins d'exemple en WebAssembly → assets/plugins/\n  changelog-page                         page HTML des versions dans dist/\n  bump patch|minor|major                 nouvelle version"
            );
            Ok(())
        }
    };
    if let Err(e) = res {
        eprintln!("erreur : {e}");
        std::process::exit(1);
    }
}

pub type Res = Result<(), Box<dyn std::error::Error>>;

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

/// Rend les trois fenêtres empilées (principale, égaliseur, playlist) en PNG ×3, texte vectoriel compris.
fn render_windows(out: Option<PathBuf>, wsz: Option<PathBuf>) -> Res {
    use alpagamp_core::{Entry, Source, TotalTime};
    use alpagamp_ui::{EqView, EqWindow, MainView, MainWindow, PlaylistView, PlaylistWindow, Transport};
    let out = out.ok_or("fichier de sortie manquant")?;
    let def = Skin::default_skin();
    let skin = match wsz {
        Some(p) => Skin::load_wsz("x", &std::fs::read(&p)?, &def)?,
        None => def,
    };
    const Z: u32 = 3;
    let mut main = MainWindow::new();
    let mv = MainView {
        transport: Transport::Playing,
        position_ms: 83_000,
        duration_ms: Some(296_000),
        seekable: true,
        title: "3. Alpagamp - Chanson de test (4:56)".into(),
        kbps: Some(320),
        khz: Some(44),
        stereo: Some(true),
        volume: 200,
        eq_open: true,
        pl_open: true,
        active: true,
        ..Default::default()
    };
    let mut eq = EqWindow::new();
    let mut settings = alpagamp_core::eq::EqSettings::default();
    settings.enabled = true;
    settings.preamp = 36;
    settings.bands = [44, 40, 34, 28, 26, 29, 34, 38, 42, 45];
    let ev = EqView { eq: settings, active: false, volume: 200, balance: 0 };
    let titles = [
        ("Ruisseau de laine", 241_000u64),
        ("Les Andes à vélo", 198_000),
        ("Alpagamp - Chanson de test", 296_000),
        ("Un titre vraiment très long qui ne tient pas dans la fenêtre de la playlist", 412_000),
        ("Cullera, la nuit", 187_000),
        ("Radio sans durée", 0),
    ];
    let mut entries: Vec<Entry> = titles
        .iter()
        .map(|(t, d)| Entry::with_meta(Source::File(PathBuf::from(format!("/m/{t}.mp3"))), Some(t.to_string()), (*d > 0).then_some(*d)))
        .collect();
    entries[3].selected = true;
    let mut pl = PlaylistWindow::new(275, 116 + 29 * 2);
    let pv = PlaylistView {
        entries: &entries,
        current: Some(2),
        active: false,
        total: TotalTime { ms: titles.iter().map(|t| t.1).sum(), partial: true, unknown: false },
        selected: TotalTime { ms: 412_000, ..Default::default() },
        time_text: "1:23".into(),
    };
    let mut fm = Image::new(1, 1, 0);
    let mut fe = Image::new(1, 1, 0);
    let mut fp = Image::new(1, 1, 0);
    main.render(&mut fm, &skin, &mv);
    eq.render(&mut fe, &skin, &ev);
    pl.render(&mut fp, &skin, &pv);
    let (w, h) = (275 * Z, (fm.height + fe.height + fp.height) * Z);
    let mut buf = vec![0xFF00_0000u32; (w * h) as usize];
    let mut y0 = 0;
    for img in [&fm, &fe, &fp] {
        for y in 0..img.height * Z {
            for x in 0..img.width * Z {
                buf[((y0 + y) * w + x) as usize] = img.pixels[((y / Z) * img.width + x / Z) as usize];
            }
        }
        y0 += img.height * Z;
    }
    let pl_top = ((fm.height + fe.height) * Z) as usize;
    let sub = &mut buf[pl_top * w as usize..];
    alpagamp_ui::text::draw_runs(sub, w, fp.height * Z, Z as f32, &pl.text);
    let mut img = Image::new(w, h, 0);
    for (d, s) in img.pixels.iter_mut().zip(&buf) {
        *d = 0xFF00_0000 | (s & 0x00FF_FFFF);
    }
    write_png(&out, &img, 1)?;
    // Fenêtre générique « Aller au morceau ».
    let mut g = alpagamp_ui::GenWindow::new(alpagamp_ui::GenKind::Jump);
    g.key(alpagamp_ui::GenKey::Text("an".into()));
    g.set_results(vec![1, 4]);
    let gv = alpagamp_ui::GenView { list: alpagamp_ui::GenList::Entries(&entries), active: true, title: "Aller au morceau", prompt: "Rechercher :", ok_label: "Lire", cancel_label: "Fermer" };
    let mut fg = Image::new(1, 1, 0);
    g.render(&mut fg, &skin, &gv);
    let (gw, gh) = (fg.width * Z, fg.height * Z);
    let mut gbuf: Vec<u32> = (0..gw * gh).map(|i| fg.pixels[(((i / gw) / Z) * fg.width + (i % gw) / Z) as usize]).collect();
    alpagamp_ui::text::draw_runs(&mut gbuf, gw, gh, Z as f32, &g.text);
    let mut gimg = Image::new(gw, gh, 0);
    for (d, s) in gimg.pixels.iter_mut().zip(&gbuf) {
        *d = 0xFF00_0000 | (s & 0x00FF_FFFF);
    }
    let gen_path = out.with_file_name(format!("{}-gen.png", out.file_stem().unwrap().to_string_lossy()));
    write_png(&gen_path, &gimg, 1)?;
    println!("{} et {}", out.display(), gen_path.display());
    Ok(())
}

/// Écrit les icônes : PNG (16 à 1024), `alpagamp.ico` (Windows) et `alpagamp.icns` (macOS).
fn icons(out: Option<PathBuf>) -> Res {
    let out = out.ok_or("dossier de sortie manquant")?;
    std::fs::create_dir_all(&out)?;
    let png_bytes = |size: u32| -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let img = alpagamp_skin::app_icon(size);
        let mut data = Vec::new();
        for p in &img.pixels {
            data.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8, (p >> 24) as u8]);
        }
        let mut buf = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut buf, size, size);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header()?.write_image_data(&data)?;
        }
        Ok(buf)
    };
    for size in [16, 32, 64, 128, 256, 512, 1024] {
        std::fs::write(out.join(format!("icon-{size}.png")), png_bytes(size)?)?;
    }
    // ICO : entrées PNG 16, 32, 48 (→ 64 réduit), 256.
    let sizes = [16u32, 32, 64, 256];
    let pngs: Vec<Vec<u8>> = sizes.iter().map(|&s| png_bytes(s)).collect::<Result<_, _>>()?;
    let mut ico = Vec::new();
    ico.extend_from_slice(&[0, 0, 1, 0]);
    ico.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (s, p) in sizes.iter().zip(&pngs) {
        let b = if *s >= 256 { 0 } else { *s as u8 };
        ico.extend_from_slice(&[b, b, 0, 0]);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(p.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += p.len() as u32;
    }
    for p in &pngs {
        ico.extend_from_slice(p);
    }
    std::fs::write(out.join("alpagamp.ico"), ico)?;
    // ICNS : ic07 (128), ic08 (256), ic09 (512), ic10 (1024).
    let mut body = Vec::new();
    for (tag, s) in [(b"ic07", 128u32), (b"ic08", 256), (b"ic09", 512), (b"ic10", 1024)] {
        let p = png_bytes(s)?;
        body.extend_from_slice(tag);
        body.extend_from_slice(&((p.len() + 8) as u32).to_be_bytes());
        body.extend_from_slice(&p);
    }
    let mut icns = b"icns".to_vec();
    icns.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
    icns.extend_from_slice(&body);
    std::fs::write(out.join("alpagamp.icns"), icns)?;
    println!("icônes écrites dans {}", out.display());
    Ok(())
}

/// Écrit la page des versions (HTML) tirée du CHANGELOG.
fn changelog(out: Option<PathBuf>) -> Res {
    let root = workspace_root();
    let out = out.unwrap_or_else(|| root.join("dist/versions.html"));
    let md = std::fs::read_to_string(root.join("CHANGELOG.md"))?;
    std::fs::write(&out, changelog_page::render(&md))?;
    println!("{}", out.display());
    Ok(())
}

/// Date du jour (UTC) en AAAA-MM-JJ.
fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

/// `bump patch|minor|major` : nouvelle version dans VERSION, les Cargo.toml et le CHANGELOG (daté).
/// Construit les plugins d'exemple (`plugin-examples/`) en WebAssembly et les copie dans `assets/plugins/`.
fn plugins() -> Res {
    let root = workspace_root();
    let dir = root.join("plugin-examples");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let ok = std::process::Command::new(&cargo)
        .current_dir(&dir)
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
        .status()?
        .success();
    if !ok {
        return Err("échec de la compilation (cible installée ? rustup target add wasm32-unknown-unknown)".into());
    }
    let out = root.join("assets/plugins");
    std::fs::create_dir_all(&out)?;
    for name in ["barres-plein-ecran", "enhancer", "minuterie-de-sommeil"] {
        let src = dir.join("target/wasm32-unknown-unknown/release").join(format!("{}.wasm", name.replace('-', "_")));
        let dst = out.join(format!("{name}.wasm"));
        std::fs::copy(&src, &dst)?;
        println!("{} ({} Ko)", dst.display(), std::fs::metadata(&dst)?.len() / 1024);
    }
    Ok(())
}

fn bump(kind: Option<&str>) -> Res {
    let root = workspace_root();
    let old = version()?;
    let mut p: Vec<u32> = old.split('.').map(|x| x.parse().unwrap_or(0)).collect();
    p.resize(3, 0);
    match kind {
        Some("major") => p = vec![p[0] + 1, 0, 0],
        Some("minor") => p = vec![p[0], p[1] + 1, 0],
        Some("patch") => p[2] += 1,
        _ => return Err("bump patch|minor|major".into()),
    }
    let new = format!("{}.{}.{}", p[0], p[1], p[2]);
    std::fs::write(root.join("VERSION"), format!("{new}\n"))?;
    // Toutes les crates du projet suivent la version du lecteur (première ligne `version = "…"` du paquet).
    let dirs = ["crates/skin", "crates/core", "crates/audio", "crates/ui", "crates/net", "crates/spotify", "crates/player", "crates/milkdrop", "crates/museum", "crates/plugins", "app/desktop", "app/android"];
    for dir in dirs {
        let f = root.join(dir).join("Cargo.toml");
        if let Ok(t) = std::fs::read_to_string(&f) {
            let mut done = false;
            let out: Vec<String> = t
                .lines()
                .map(|l| {
                    if !done && l.starts_with("version = \"") {
                        done = true;
                        format!("version = \"{new}\"")
                    } else {
                        l.to_string()
                    }
                })
                .collect();
            std::fs::write(&f, out.join("\n") + "\n")?;
        }
    }
    // Les deux journaux (français, anglais) reçoivent le même titre de version : les notes de version les citent.
    for (nom, non_publie) in [("CHANGELOG.md", "## [Non publié]"), ("CHANGELOG.en.md", "## [Unreleased]")] {
        let cl = root.join(nom);
        if !cl.exists() {
            continue;
        }
        let t = std::fs::read_to_string(&cl)?;
        let t = t.replacen(non_publie, &format!("{non_publie}\n\n## [{new}] - {}", today()), 1);
        std::fs::write(&cl, t)?;
    }
    println!("{old} → {new} (compléter « Mise en service » dans CHANGELOG.md et « Rollout » dans CHANGELOG.en.md)");
    Ok(())
}

/// Version courante (fichier `VERSION`, source unique).
pub fn version() -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(workspace_root().join("VERSION"))?.trim().to_string())
}

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// Refuse d'écraser une version déjà produite (une version publiée ne change jamais).
pub fn refuse_overwrite(p: &Path) -> Res {
    if p.exists() {
        return Err(format!("{} existe déjà : changer de version (cargo xtask bump …)", p.display()).into());
    }
    Ok(())
}

const README_WINDOWS: &str = "Alpagamp {v} — Windows\r\n\r\n\
FR : Décompresser le dossier, puis lancer alpagamp.exe. Au premier lancement, Windows peut afficher\r\n\
« Windows a protégé votre ordinateur » : cliquer sur « Informations complémentaires », puis « Exécuter quand même ».\r\n\
Glisser des fichiers audio ou un dossier sur la fenêtre pour les jouer, et un skin .wsz pour l'appliquer.\r\n\
Clic droit sur la fenêtre : menu complet.\r\n\r\n\
EN: Unzip the folder and run alpagamp.exe. On first launch Windows may show \"Windows protected your PC\":\r\n\
click \"More info\", then \"Run anyway\". Drop audio files or a folder on the window to play them, and a .wsz skin\r\n\
to apply it. Right-click the window for the full menu.\r\n\r\n\
ES: Descomprimir la carpeta y abrir alpagamp.exe. Si Windows muestra \"Windows protegió su PC\", pulsar\r\n\
\"Más información\" y luego \"Ejecutar de todas formas\".\r\n\r\n\
https://github.com/Guiwald/alpagamp\r\n";

/// Construit le zip Windows à partir de l'exécutable déjà compilé.
fn dist_windows(exe: Option<PathBuf>) -> Res {
    use std::io::Write;
    let root = workspace_root();
    let v = version()?;
    let exe = exe.unwrap_or_else(|| root.join("target/x86_64-pc-windows-gnu/release/alpagamp.exe"));
    let out = root.join("dist").join(format!("Alpagamp-{v}-windows-x64.zip"));
    refuse_overwrite(&out)?;
    let file = std::fs::File::create(&out)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let dir = format!("Alpagamp-{v}");
    zip.start_file(format!("{dir}/alpagamp.exe"), opts)?;
    zip.write_all(&std::fs::read(&exe)?)?;
    zip.start_file(format!("{dir}/LISEZMOI-README.txt"), opts)?;
    zip.write_all(README_WINDOWS.replace("{v}", &v).as_bytes())?;
    zip.start_file(format!("{dir}/LICENSE.txt"), opts)?;
    zip.write_all(&std::fs::read(root.join("LICENSE"))?)?;
    zip.finish()?;
    println!("{}", out.display());
    Ok(())
}

/// Construit `Alpagamp.app` (et, sur macOS, `Alpagamp-X.Y.Z.dmg`) à partir d'un binaire (universel ou non).
fn bundle_mac(binary: Option<PathBuf>, out_dir: Option<PathBuf>) -> Res {
    let root = workspace_root();
    let v = version()?;
    let binary = binary.ok_or("binaire manquant")?;
    let out_dir = out_dir.unwrap_or_else(|| root.join("dist"));
    let app = out_dir.join("Alpagamp.app");
    let _ = std::fs::remove_dir_all(&app);
    let contents = app.join("Contents");
    std::fs::create_dir_all(contents.join("MacOS"))?;
    std::fs::create_dir_all(contents.join("Resources"))?;
    std::fs::copy(&binary, contents.join("MacOS/alpagamp"))?;
    std::fs::copy(root.join("assets/icons/alpagamp.icns"), contents.join("Resources/alpagamp.icns"))?;
    let build: u32 = {
        let p: Vec<u32> = v.split('.').map(|x| x.parse().unwrap_or(0)).collect();
        p.first().unwrap_or(&0) * 10000 + p.get(1).unwrap_or(&0) * 100 + p.get(2).unwrap_or(&0)
    };
    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Alpagamp</string>
    <key>CFBundleDisplayName</key><string>Alpagamp</string>
    <key>CFBundleIdentifier</key><string>fr.guiwald.alpagamp</string>
    <key>CFBundleVersion</key><string>{build}</string>
    <key>CFBundleShortVersionString</key><string>{v}</string>
    <key>CFBundleExecutable</key><string>alpagamp</string>
    <key>CFBundleIconFile</key><string>alpagamp</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>LSApplicationCategoryType</key><string>public.app-category.music</string>
    <key>NSHumanReadableCopyright</key><string>MIT — github.com/Guiwald/alpagamp</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key><string>Audio</string>
            <key>CFBundleTypeRole</key><string>Viewer</string>
            <key>LSHandlerRank</key><string>Alternate</string>
            <key>LSItemContentTypes</key>
            <array><string>public.audio</string><string>public.mp3</string><string>org.xiph.flac</string></array>
        </dict>
    </array>
</dict>
</plist>
"#);
    std::fs::write(contents.join("Info.plist"), plist)?;
    println!("{}", app.display());
    // Le .dmg ne se fabrique que sur macOS (hdiutil) ; ailleurs, seul le .app est produit.
    #[cfg(target_os = "macos")]
    {
        let run = |cmd: &mut std::process::Command| -> Res {
            let st = cmd.status()?;
            if !st.success() {
                return Err(format!("échec : {cmd:?}").into());
            }
            Ok(())
        };
        // Signature ad hoc (nécessaire sur Apple Silicon).
        run(std::process::Command::new("codesign").args(["--force", "--deep", "--sign", "-"]).arg(&app))?;
        let dmg = out_dir.join(format!("Alpagamp-{v}.dmg"));
        refuse_overwrite(&dmg)?;
        let stage = out_dir.join("dmg-stage");
        let _ = std::fs::remove_dir_all(&stage);
        std::fs::create_dir_all(&stage)?;
        run(std::process::Command::new("cp").arg("-R").arg(&app).arg(&stage))?;
        std::os::unix::fs::symlink("/Applications", stage.join("Applications"))?;
        run(std::process::Command::new("hdiutil").args(["create", "-volname", "Alpagamp", "-fs", "HFS+", "-format", "UDZO", "-srcfolder"]).arg(&stage).arg(&dmg))?;
        let _ = std::fs::remove_dir_all(&stage);
        println!("{}", dmg.display());
    }
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
