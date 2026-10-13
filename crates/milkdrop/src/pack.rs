//! Le pack de presets « Cream of the Crop » (9 795 presets triés par ISOSCELES, le pack de projectM) : téléchargé à
//! la demande, jamais livré avec Alpagamp (ADR 0004 et 0019).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use std::sync::Mutex;

/// Version figée du dépôt (une archive identique pour tout le monde).
pub const COMMIT: &str = "0180df21f5e0bd39b9060cc5de420ed2f1f9e509";
/// Dossier du pack, dans le dossier des presets.
pub const DIR: &str = "Cream of the Crop";

fn urls() -> Vec<String> {
    vec![
        format!("https://codeload.github.com/projectM-visualizer/presets-cream-of-the-crop/zip/{COMMIT}"),
        format!("https://github.com/projectM-visualizer/presets-cream-of-the-crop/archive/{COMMIT}.zip"),
    ]
}

/// Où en est l'installation.
#[derive(Clone, Debug, PartialEq)]
pub enum PackState {
    Idle,
    Downloading { done: u64, total: Option<u64> },
    Extracting { done: usize, total: usize },
    Ready { presets: usize },
    Failed(String),
}

/// Installation en arrière-plan.
pub struct PackInstaller {
    state: Arc<Mutex<PackState>>,
    cancel: Arc<AtomicBool>,
}

impl Default for PackInstaller {
    fn default() -> Self {
        PackInstaller { state: Arc::new(Mutex::new(PackState::Idle)), cancel: Arc::new(AtomicBool::new(false)) }
    }
}

/// Le pack est-il déjà là ?
pub fn installed(presets_dir: &Path) -> bool {
    presets_dir.join(DIR).join("LICENSE.md").exists()
}

impl PackInstaller {
    pub fn state(&self) -> PackState {
        self.state.lock().map(|s| s.clone()).unwrap_or(PackState::Idle)
    }

    pub fn busy(&self) -> bool {
        matches!(self.state(), PackState::Downloading { .. } | PackState::Extracting { .. })
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Lance le téléchargement puis l'extraction dans `presets_dir`/Cream of the Crop.
    pub fn start(&self, presets_dir: PathBuf) {
        if self.busy() {
            return;
        }
        self.cancel.store(false, Ordering::Relaxed);
        let state = self.state.clone();
        let cancel = self.cancel.clone();
        let set = move |s: PackState| {
            if let Ok(mut g) = state.lock() {
                *g = s;
            }
        };
        set(PackState::Downloading { done: 0, total: None });
        std::thread::Builder::new()
            .name("presets".into())
            .spawn(move || {
                let r = install(&presets_dir, &urls(), &cancel, &set);
                set(match r {
                    Ok(n) => PackState::Ready { presets: n },
                    Err(e) => {
                        log::warn!("pack de presets : {e}");
                        PackState::Failed(e)
                    }
                });
            })
            .ok();
    }
}

fn install(presets_dir: &Path, urls: &[String], cancel: &AtomicBool, set: &dyn Fn(PackState)) -> Result<usize, String> {
    std::fs::create_dir_all(presets_dir).map_err(|e| e.to_string())?;
    let zip_path = presets_dir.join("cream-of-the-crop.zip");
    let mut last = String::new();
    let mut ok = false;
    for url in urls {
        match alpagamp_net::download(url, &zip_path, |done, total| set(PackState::Downloading { done, total }), cancel) {
            Ok(()) => {
                ok = true;
                break;
            }
            Err(e) => last = e.0,
        }
        if cancel.load(Ordering::Relaxed) {
            break;
        }
    }
    if !ok {
        return Err(last);
    }
    let n = extract(&zip_path, &presets_dir.join(DIR), &|done, total| set(PackState::Extracting { done, total }));
    let _ = std::fs::remove_file(&zip_path);
    n
}

/// Extrait les `.milk` (et la licence) en retirant le dossier racine de l'archive.
pub fn extract(zip_path: &Path, dest: &Path, progress: &dyn Fn(usize, usize)) -> Result<usize, String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(|e| format!("archive illisible : {e}"))?;
    let total = zip.len();
    let tmp = dest.with_extension("tmp");
    let _ = std::fs::remove_dir_all(&tmp);
    let mut count = 0;
    for i in 0..total {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let Some(path) = entry.enclosed_name() else { continue };
        // Retire « presets-cream-of-the-crop-<commit>/ ».
        let rel: PathBuf = path.components().skip(1).collect();
        let keep = match rel.extension().and_then(|e| e.to_str()) {
            Some(e) if e.eq_ignore_ascii_case("milk") => {
                count += 1;
                true
            }
            _ => rel.parent().is_some_and(|p| p.as_os_str().is_empty()) && rel.extension().is_some_and(|e| e == "md"),
        };
        if !keep || rel.as_os_str().is_empty() {
            continue;
        }
        let out = tmp.join(&rel);
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut f = std::fs::File::create(&out).map_err(|e| format!("{} : {e}", out.display()))?;
        std::io::copy(&mut entry, &mut f).map_err(|e| e.to_string())?;
        if i % 50 == 0 {
            progress(i, total);
        }
    }
    if count == 0 {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err("l'archive ne contient aucun preset".into());
    }
    let _ = std::fs::remove_dir_all(dest);
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Téléchargement depuis un petit serveur local, puis extraction (le premier serveur répond 404).
    #[test]
    fn downloads_and_installs() {
        let dir = std::env::temp_dir().join(format!("alpagamp-pack-dl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut body = Vec::new();
        {
            let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut body));
            let o = zip::write::SimpleFileOptions::default();
            for (name, b) in [("p-x/LICENSE.md", "l"), ("p-x/Waveform/a.milk", "[preset00]"), ("p-x/Waveform/b.milk", "[preset00]")] {
                z.start_file(name, o).unwrap();
                z.write_all(b.as_bytes()).unwrap();
            }
            z.finish().unwrap();
        }
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for (i, conn) in listener.incoming().take(2).enumerate() {
                let mut c = conn.unwrap();
                let mut req = [0u8; 1024];
                let _ = std::io::Read::read(&mut c, &mut req);
                if i == 0 {
                    let _ = c.write_all(b"HTTP/1.0 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                } else {
                    let _ = c.write_all(format!("HTTP/1.0 200 OK\r\nContent-Length: {}\r\n\r\n", body.len()).as_bytes());
                    let _ = c.write_all(&body);
                }
            }
        });
        let urls = vec![format!("http://127.0.0.1:{port}/absent.zip"), format!("http://127.0.0.1:{port}/pack.zip")];
        let states = std::sync::Mutex::new(Vec::new());
        let n = install(&dir, &urls, &AtomicBool::new(false), &|s| states.lock().unwrap().push(s)).unwrap();
        assert_eq!(n, 2);
        assert!(installed(&dir));
        assert!(!dir.join("cream-of-the-crop.zip").exists());
        assert!(states.lock().unwrap().iter().any(|s| matches!(s, PackState::Downloading { done, .. } if *done > 0)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extracts_presets_only() {
        let dir = std::env::temp_dir().join(format!("alpagamp-pack-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("p.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
            let o = zip::write::SimpleFileOptions::default();
            for (name, body) in [
                ("root-abc/LICENSE.md", "licence"),
                ("root-abc/Dancer/Comet/a.milk", "[preset00]"),
                ("root-abc/Waveform/b.MILK", "[preset00]"),
                ("root-abc/Waveform/preview.jpg", "x"),
            ] {
                z.start_file(name, o).unwrap();
                z.write_all(body.as_bytes()).unwrap();
            }
            z.finish().unwrap();
        }
        let dest = dir.join(DIR);
        assert_eq!(extract(&zip_path, &dest, &|_, _| {}).unwrap(), 2);
        assert!(dest.join("Dancer/Comet/a.milk").exists());
        assert!(dest.join("LICENSE.md").exists());
        assert!(!dest.join("Waveform/preview.jpg").exists());
        assert!(installed(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
