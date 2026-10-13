//! Le pilote du navigateur de skins, commun au bureau et au téléphone : listes du musée (page par page) et des
//! skins rangés, aperçus, téléchargement. L'application applique le skin choisi ([`Apply`]).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use alpagamp_core::i18n::t;
use alpagamp_skin::{Image, Skin};
use alpagamp_ui::{MainView, MainWindow, SkinRow, SkinsAction, SkinsTab, SkinsWindow};

use crate::{Event, Museum, Request, SkinInfo, file_name, thumbnail};

/// Ce que l'application doit faire.
#[derive(Clone, Debug, PartialEq)]
pub enum Apply {
    /// Revenir au skin par défaut.
    Default,
    /// Charger ce fichier `.wsz`.
    File(PathBuf),
    /// Ouvrir la boîte « Charger un skin ».
    OpenDialog,
}

/// Aperçu d'un skin rangé : sa fenêtre principale dessinée par Alpagamp, réduite de moitié.
pub fn local_thumbnail(skin: &Skin) -> Image {
    let mut fb = Image::new(1, 1, 0);
    let mut w = MainWindow::new();
    w.render(&mut fb, skin, &MainView { title: "Alpagamp".into(), active: true, ..Default::default() });
    thumbnail(&fb)
}

/// Skins `.wsz` / `.zip` d'un dossier, triés.
pub fn list_skins(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| matches!(p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("wsz" | "zip")))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

pub struct Browser {
    pub ui: SkinsWindow,
    skins_dir: PathBuf,
    museum: Option<Museum>,
    /// Demande de liste en cours (et si elle ajoute une page).
    pending: Option<(u64, bool)>,
    query: String,
    infos: Vec<SkinInfo>,
    /// Lignes de « Mes skins » : `None` = skin par défaut.
    locals: Vec<Option<PathBuf>>,
    thumbs_asked: HashSet<String>,
    error: Option<String>,
}

impl Browser {
    pub fn new(skins_dir: PathBuf, ui: SkinsWindow) -> Browser {
        Browser { ui, skins_dir, museum: None, pending: None, query: String::new(), infos: Vec::new(), locals: Vec::new(), thumbs_asked: HashSet::new(), error: None }
    }

    fn museum(&mut self) -> &mut Museum {
        self.museum.get_or_insert_with(Museum::new)
    }

    /// Charge la liste de l'onglet courant ; `current` : le skin appliqué (pour le marquer).
    pub fn load_tab(&mut self, current: Option<&Path>) {
        self.error = None;
        match self.ui.tab {
            SkinsTab::Museum => {
                let q = self.query.clone();
                self.infos.clear();
                self.ui.set_rows(Vec::new());
                let id = self.museum().request(Request::List { query: q, offset: 0 });
                self.pending = Some((id, false));
            }
            SkinsTab::Mine => {
                let mut locals: Vec<Option<PathBuf>> = vec![None];
                locals.extend(list_skins(&self.skins_dir).into_iter().map(Some));
                let rows = locals
                    .iter()
                    .enumerate()
                    .map(|(i, l)| {
                        let name = match l {
                            None => t("skins.default").to_string(),
                            Some(p) => p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                        };
                        let detail = if l.as_deref() == current { t("skins.applied").to_string() } else { String::new() };
                        SkinRow { key: format!("local:{i}"), name, detail, thumb: None }
                    })
                    .collect();
                self.locals = locals;
                self.ui.set_rows(rows);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.ui.rows().is_empty()
    }

    /// Une action de la fenêtre ; renvoie ce qu'il faut appliquer tout de suite.
    pub fn action(&mut self, a: SkinsAction, current: Option<&Path>) -> Option<Apply> {
        match a {
            SkinsAction::Tab(_) => self.load_tab(current),
            SkinsAction::Search(q) => {
                self.query = q;
                self.load_tab(current);
            }
            SkinsAction::OpenFile => return Some(Apply::OpenDialog),
            SkinsAction::Apply(i) => match self.ui.tab {
                SkinsTab::Mine => {
                    let l = self.locals.get(i).cloned()?;
                    self.mark_applied(&format!("local:{i}"));
                    return Some(match l {
                        None => Apply::Default,
                        Some(p) => Apply::File(p),
                    });
                }
                SkinsTab::Museum => {
                    let info = self.infos.get(i).cloned()?;
                    let dest = self.skins_dir.join(file_name(&info));
                    if dest.exists() {
                        self.mark_applied(&info.md5);
                        return Some(Apply::File(dest));
                    }
                    self.ui.set_detail(&info.md5, t("skins.downloading"));
                    self.museum().request(Request::Download { info, dest });
                }
            },
        }
        None
    }

    fn mark_applied(&mut self, key: &str) {
        let keys: Vec<String> = self.ui.rows().iter().map(|r| r.key.clone()).collect();
        for k in keys {
            self.ui.set_detail(&k, if k == key { t("skins.applied") } else { "" });
        }
    }

    /// Événements du musée, aperçus, page suivante. `visible` : la fenêtre est à l'écran. Renvoie un skin
    /// téléchargé à appliquer.
    pub fn tick(&mut self, visible: bool, default_skin: &Skin) -> Option<Apply> {
        let mut apply = None;
        let events = self.museum.as_ref().map(|m| m.poll()).unwrap_or_default();
        for ev in events {
            match ev {
                Event::List { id, result, .. } => {
                    let Some((pid, append)) = self.pending else { continue };
                    if pid != id {
                        continue;
                    }
                    self.pending = None;
                    match result {
                        Ok(list) => {
                            let rows: Vec<SkinRow> = list.iter().map(|s| SkinRow { key: s.md5.clone(), name: s.name.clone(), detail: String::new(), thumb: None }).collect();
                            if append {
                                self.ui.append_rows(rows);
                                self.infos.extend(list);
                            } else {
                                if list.is_empty() {
                                    self.error = Some(t("skins.none").to_string());
                                }
                                self.ui.set_rows(rows);
                                self.infos = list;
                            }
                        }
                        Err(e) => {
                            log::warn!("musée des skins : {e}");
                            self.error = Some(format!("{} ({e})", t("skins.error")));
                        }
                    }
                }
                Event::Thumbnail { md5, image } => {
                    if let Some(img) = image {
                        self.ui.set_thumb(&md5, img);
                    }
                }
                Event::Downloaded { info, result, .. } => match result {
                    Ok(path) => {
                        self.mark_applied(&info.md5);
                        apply = Some(Apply::File(path));
                    }
                    Err(e) => self.ui.set_detail(&info.md5, &e),
                },
            }
        }
        if !visible {
            return apply;
        }
        let missing = self.ui.visible_without_thumb();
        match self.ui.tab {
            SkinsTab::Museum => {
                for i in missing {
                    let Some(info) = self.infos.get(i).cloned() else { continue };
                    if self.thumbs_asked.insert(info.md5.clone()) {
                        self.museum().request(Request::Thumbnail { md5: info.md5, url: info.screenshot_url });
                    }
                }
                if self.pending.is_none() && self.error.is_none() && self.ui.wants_more() {
                    let q = self.query.clone();
                    let offset = self.infos.len();
                    let id = self.museum().request(Request::List { query: q, offset });
                    self.pending = Some((id, true));
                }
            }
            SkinsTab::Mine => {
                // Deux par image, pour rester fluide.
                for i in missing.into_iter().take(2) {
                    let skin = match self.locals.get(i) {
                        Some(None) => Some(default_skin.clone()),
                        Some(Some(p)) => std::fs::read(p).ok().and_then(|b| Skin::load_wsz("", &b, default_skin).ok()),
                        None => None,
                    };
                    let img = skin.as_ref().map(local_thumbnail).unwrap_or_else(|| Image::new(137, 58, 0xFF00_0000));
                    self.ui.set_thumb(&format!("local:{i}"), img);
                }
            }
        }
        apply
    }

    /// Texte d'état en bas de la fenêtre.
    pub fn status(&self) -> String {
        if self.pending.is_some() {
            return t("skins.loading").to_string();
        }
        if let Some(e) = &self.error {
            return e.clone();
        }
        match self.ui.tab {
            SkinsTab::Museum => t("skins.credit").to_string(),
            SkinsTab::Mine => String::new(),
        }
    }
}
