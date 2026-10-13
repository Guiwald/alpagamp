//! La liste des presets : ceux d'Alpagamp (intégrés), puis les fichiers `.milk` du dossier des presets (le pack
//! téléchargé et ceux que l'on y dépose), triés par dossier puis par nom.

use std::path::{Path, PathBuf};

/// Presets originaux d'Alpagamp (licence MIT, comme le reste du projet).
pub const BUILTIN: &[(&str, &[u8])] = &[
    ("Alpagamp - Laine des Andes", include_bytes!("../presets/laine-des-andes.milk")),
    ("Alpagamp - Cullera de nuit", include_bytes!("../presets/cullera-de-nuit.milk")),
    ("Alpagamp - Tunnel du lama", include_bytes!("../presets/tunnel-du-lama.milk")),
    ("Alpagamp - Étincelles", include_bytes!("../presets/etincelles.milk")),
    ("Alpagamp - Marée", include_bytes!("../presets/maree.milk")),
    ("Alpagamp - Néon", include_bytes!("../presets/neon.milk")),
];

#[derive(Clone, Debug, PartialEq)]
pub enum PresetSource {
    Builtin(usize),
    File(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    /// Nom affiché (nom du fichier sans extension).
    pub name: String,
    /// Dossier relatif (« Dancer/Comet »), vide pour les presets intégrés.
    pub folder: String,
    pub source: PresetSource,
}

impl Preset {
    /// Clé stable (réglages : dernier preset vu).
    pub fn key(&self) -> String {
        if self.folder.is_empty() { self.name.clone() } else { format!("{}/{}", self.folder, self.name) }
    }

    pub fn bytes(&self) -> Option<Vec<u8>> {
        match &self.source {
            PresetSource::Builtin(i) => BUILTIN.get(*i).map(|(_, b)| b.to_vec()),
            PresetSource::File(p) => std::fs::read(p).ok(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PresetList {
    pub items: Vec<Preset>,
}

impl PresetList {
    /// Les presets intégrés seuls.
    pub fn builtin() -> PresetList {
        PresetList {
            items: BUILTIN
                .iter()
                .enumerate()
                .map(|(i, (name, _))| Preset { name: name.to_string(), folder: String::new(), source: PresetSource::Builtin(i) })
                .collect(),
        }
    }

    /// Intégrés + fichiers trouvés sous `dir` (récursif).
    pub fn scan(dir: &Path) -> PresetList {
        let mut list = PresetList::builtin();
        let mut files = Vec::new();
        walk(dir, dir, &mut files, 0);
        files.sort_by(|a: &Preset, b: &Preset| {
            (a.folder.to_lowercase(), a.name.to_lowercase()).cmp(&(b.folder.to_lowercase(), b.name.to_lowercase()))
        });
        list.items.extend(files);
        list
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, i: usize) -> Option<&Preset> {
        self.items.get(i)
    }

    pub fn find_key(&self, key: &str) -> Option<usize> {
        self.items.iter().position(|p| p.key() == key)
    }

    /// Indices des presets dont le nom ou le dossier contient tous les mots de `query`.
    pub fn search(&self, query: &str) -> Vec<usize> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        self.items
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                let hay = p.key().to_lowercase();
                words.iter().all(|w| hay.contains(w.as_str()))
            })
            .map(|(i, _)| i)
            .collect()
    }
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<Preset>, depth: usize) {
    if depth > 6 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if p.is_dir() {
            walk(root, &p, out, depth + 1);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("milk")) {
            let folder = p
                .parent()
                .and_then(|d| d.strip_prefix(root).ok())
                .map(|r| r.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>().join("/"))
                .unwrap_or_default();
            let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            out.push(Preset { name: stem, folder, source: PresetSource::File(p) });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_presets_parse_names() {
        let l = PresetList::builtin();
        assert_eq!(l.len(), BUILTIN.len());
        assert!(l.items.iter().all(|p| p.bytes().is_some_and(|b| b.starts_with(b"[preset00]") || b.starts_with(b"MILKDROP"))));
        assert_eq!(l.search("lama").len(), 1);
    }

    #[test]
    fn scans_folders() {
        let dir = std::env::temp_dir().join(format!("alpagamp-presets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Pack/Dancer")).unwrap();
        std::fs::write(dir.join("Pack/Dancer/b.milk"), "[preset00]").unwrap();
        std::fs::write(dir.join("Pack/Dancer/a.milk"), "[preset00]").unwrap();
        std::fs::write(dir.join("mine.MILK"), "[preset00]").unwrap();
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        let l = PresetList::scan(&dir);
        let names: Vec<_> = l.items[BUILTIN.len()..].iter().map(|p| p.key()).collect();
        assert_eq!(names, ["mine", "Pack/Dancer/a", "Pack/Dancer/b"]);
        assert_eq!(l.search("dancer/a"), vec![BUILTIN.len() + 1]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
