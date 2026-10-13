//! Construction de l'APK Android sans Gradle : bibliothèque Rust (NDK), `aapt2` pour le manifeste et les
//! ressources, ajout de la bibliothèque non compressée, `zipalign`, signature `apksigner`.
//!
//! Emplacements lus dans l'environnement : `ANDROID_HOME` (SDK : `build-tools/*`, `platforms/android-*`),
//! `ANDROID_NDK_HOME` (sinon `ANDROID_HOME/ndk/*` ou `ANDROID_HOME/android-ndk-*`). Signature :
//! `ALPAGAMP_KEYSTORE` et `ALPAGAMP_KEYSTORE_PASS` ; sans eux, une clé de débogage jetable est créée.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Res, refuse_overwrite, version, workspace_root};

const TARGET: &str = "aarch64-linux-android";
const MIN_SDK: u32 = 26;
const TARGET_SDK: u32 = 34;

fn newest_dir(dir: &Path, pred: impl Fn(&str) -> bool) -> Option<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).filter(|p| p.is_dir() && p.file_name().is_some_and(|n| pred(&n.to_string_lossy()))).collect();
    v.sort();
    v.pop()
}

struct Sdk {
    build_tools: PathBuf,
    android_jar: PathBuf,
    ndk_bin: PathBuf,
}

fn find_sdk() -> Result<Sdk, Box<dyn std::error::Error>> {
    let home = std::env::var_os("ANDROID_HOME").or_else(|| std::env::var_os("ANDROID_SDK_ROOT")).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/opt/android-sdk"));
    let build_tools = newest_dir(&home.join("build-tools"), |_| true)
        .or_else(|| newest_dir(&home, |n| n.starts_with("build-tools")))
        .filter(|p| p.join("aapt2").exists())
        .ok_or("build-tools introuvables (ANDROID_HOME)")?;
    let platform = newest_dir(&home.join("platforms"), |n| n.starts_with("android-"))
        .or_else(|| newest_dir(&home, |n| n.starts_with("platform-")))
        .ok_or("plateforme Android introuvable (ANDROID_HOME/platforms)")?;
    let ndk = std::env::var_os("ANDROID_NDK_HOME")
        .map(PathBuf::from)
        .or_else(|| newest_dir(&home.join("ndk"), |_| true))
        .or_else(|| newest_dir(&home, |n| n.starts_with("android-ndk")))
        .ok_or("NDK introuvable (ANDROID_NDK_HOME)")?;
    let host = if cfg!(target_os = "macos") { "darwin-x86_64" } else if cfg!(target_os = "windows") { "windows-x86_64" } else { "linux-x86_64" };
    Ok(Sdk { build_tools, android_jar: platform.join("android.jar"), ndk_bin: ndk.join("toolchains/llvm/prebuilt").join(host).join("bin") })
}

fn run(cmd: &mut Command) -> Res {
    let st = cmd.status().map_err(|e| format!("{cmd:?} : {e}"))?;
    if !st.success() {
        return Err(format!("échec : {cmd:?}").into());
    }
    Ok(())
}

fn manifest(v: &str, code: u32) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="fr.guiwald.alpagamp"
    android:versionCode="{code}"
    android:versionName="{v}">
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.READ_MEDIA_AUDIO" />
    <uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" android:maxSdkVersion="32" />
    <application
        android:label="@string/app_name"
        android:icon="@mipmap/ic_launcher"
        android:hasCode="false"
        android:allowBackup="true"
        android:theme="@android:style/Theme.DeviceDefault.NoActionBar">
        <activity
            android:name="android.app.NativeActivity"
            android:exported="true"
            android:launchMode="singleTask"
            android:screenOrientation="portrait"
            android:windowSoftInputMode="adjustResize"
            android:configChanges="orientation|screenSize|screenLayout|smallestScreenSize|keyboard|keyboardHidden|navigation|uiMode|density|fontScale|locale|layoutDirection">
            <meta-data android:name="android.app.lib_name" android:value="alpagamp" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#
    )
}

fn write_icon(path: &Path, size: u32) -> Res {
    let img = alpagamp_skin::app_icon(size);
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for p in &img.pixels {
        data.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8, (p >> 24) as u8]);
    }
    let file = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), size, size);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&data)?;
    Ok(())
}

/// `cargo xtask apk` : construit `dist/Alpagamp-X.Y.Z.apk`.
pub fn build_apk() -> Res {
    let root = workspace_root();
    let v = version()?;
    let parts: Vec<u32> = v.split('.').map(|x| x.parse().unwrap_or(0)).collect();
    let code = parts.first().unwrap_or(&0) * 10000 + parts.get(1).unwrap_or(&0) * 100 + parts.get(2).unwrap_or(&0);
    let out = root.join("dist").join(format!("Alpagamp-{v}.apk"));
    refuse_overwrite(&out)?;
    let sdk = find_sdk()?;
    let clang = sdk.ndk_bin.join(format!("{TARGET}{MIN_SDK}-clang"));
    // 1. Bibliothèque Rust.
    let env_target = TARGET.replace('-', "_");
    run(Command::new(env!("CARGO"))
        .args(["build", "--release", "-p", "alpagamp-android", "--target", TARGET])
        .current_dir(&root)
        .env(format!("CARGO_TARGET_{}_LINKER", env_target.to_uppercase()), &clang)
        .env(format!("CC_{env_target}"), &clang)
        .env(format!("AR_{env_target}"), sdk.ndk_bin.join("llvm-ar")))?;
    let so = root.join("target").join(TARGET).join("release/libalpagamp.so");
    // 2. Manifeste et ressources.
    let work = root.join("target/apk");
    let _ = std::fs::remove_dir_all(&work);
    let res = work.join("res");
    std::fs::create_dir_all(res.join("values"))?;
    std::fs::write(work.join("AndroidManifest.xml"), manifest(&v, code))?;
    std::fs::write(res.join("values/strings.xml"), "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources><string name=\"app_name\">Alpagamp</string></resources>\n")?;
    for (dpi, size) in [("mdpi", 48), ("hdpi", 72), ("xhdpi", 96), ("xxhdpi", 144), ("xxxhdpi", 192)] {
        let d = res.join(format!("mipmap-{dpi}"));
        std::fs::create_dir_all(&d)?;
        write_icon(&d.join("ic_launcher.png"), size)?;
    }
    let aapt2 = sdk.build_tools.join("aapt2");
    run(Command::new(&aapt2).arg("compile").arg("--dir").arg(&res).arg("-o").arg(work.join("res.zip")))?;
    let base = work.join("base.apk");
    run(Command::new(&aapt2)
        .arg("link")
        .arg("-o")
        .arg(&base)
        .arg("-I")
        .arg(&sdk.android_jar)
        .arg("--manifest")
        .arg(work.join("AndroidManifest.xml"))
        .args(["--min-sdk-version", &MIN_SDK.to_string(), "--target-sdk-version", &TARGET_SDK.to_string()])
        .args(["--version-code", &code.to_string(), "--version-name", &v])
        .arg(work.join("res.zip")))?;
    // 3. Ajout de la bibliothèque, non compressée (chargée directement depuis l'APK).
    let unaligned = work.join("unaligned.apk");
    {
        let mut src = zip::ZipArchive::new(std::fs::File::open(&base)?)?;
        let mut dst = zip::ZipWriter::new(std::fs::File::create(&unaligned)?);
        for i in 0..src.len() {
            let f = src.by_index(i)?;
            dst.raw_copy_file(f)?;
        }
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        dst.start_file(format!("lib/arm64-v8a/libalpagamp.so"), stored)?;
        let mut bytes = Vec::new();
        std::fs::File::open(&so)?.read_to_end(&mut bytes)?;
        dst.write_all(&bytes)?;
        dst.finish()?;
    }
    // 4. Alignement (pages de 16 Ko) et signature.
    let aligned = work.join("aligned.apk");
    run(Command::new(sdk.build_tools.join("zipalign")).args(["-f", "-P", "16", "4"]).arg(&unaligned).arg(&aligned))?;
    let (ks, pass) = match (std::env::var_os("ALPAGAMP_KEYSTORE"), std::env::var("ALPAGAMP_KEYSTORE_PASS")) {
        (Some(k), Ok(p)) => (PathBuf::from(k), p),
        _ => {
            eprintln!("ALPAGAMP_KEYSTORE absent : signature avec une clé de débogage jetable (l'appli ne pourra pas être mise à jour par une version signée autrement)");
            let ks = work.join("debug.p12");
            run(Command::new("keytool")
                .args(["-genkeypair", "-keystore"])
                .arg(&ks)
                .args(["-storetype", "PKCS12", "-storepass", "android", "-keypass", "android", "-alias", "debug", "-keyalg", "RSA", "-keysize", "2048", "-validity", "3650", "-dname", "CN=Alpagamp debug"]))?;
            (ks, "android".to_string())
        }
    };
    std::fs::create_dir_all(root.join("dist"))?;
    run(Command::new(sdk.build_tools.join("apksigner"))
        .args(["sign", "--ks"])
        .arg(&ks)
        .args(["--ks-pass", "env:ALPAGAMP_APKSIGNER_PASS", "--key-pass", "env:ALPAGAMP_APKSIGNER_PASS", "--out"])
        .arg(&out)
        .arg(&aligned)
        .env("ALPAGAMP_APKSIGNER_PASS", &pass))?;
    run(Command::new(sdk.build_tools.join("apksigner")).args(["verify", "--print-certs"]).arg(&out))?;
    let _ = std::fs::remove_file(out.with_extension("apk.idsig"));
    println!("{}", out.display());
    Ok(())
}
