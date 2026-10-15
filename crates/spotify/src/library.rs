//! La Bibliothèque Spotify (ADR 0010, 0022) : quel canal pour quelle liste, listes gardées sur disque, attente
//! demandée par Spotify (429), et les lignes que la fenêtre affiche (avis, compte à rebours, « Actualiser »).
//!
//! - Playlists, titres likés, albums enregistrés, contenu d'une playlist ou d'un album : canaux internes
//!   (`internal`), par la session librespot.
//! - Recherche : titres par le canal interne ; albums et playlists par l'API web (`webapi`), si elle ne demande
//!   pas de patienter.
//! - Une liste reçue est gardée sur disque : elle s'affiche tout de suite et n'est redemandée que si elle a plus
//!   de 15 min, ou sur « Actualiser ».
//! - Quand Spotify demande de patienter, on ne redemande rien à ce canal avant l'heure dite (même si on insiste).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use alpagamp_core::i18n::t;
use alpagamp_core::library::{LibraryItem, LibraryKind};
use serde::{Deserialize, Serialize};

/// Âge au-delà duquel une liste gardée est redemandée.
pub const FRESH: Duration = Duration::from_secs(15 * 60);
/// Attente par défaut quand Spotify dit « 429 » sans préciser combien de temps.
pub const DEFAULT_WAIT: Duration = Duration::from_secs(60);

/// Pourquoi une requête a échoué.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    NotConnected,
    /// 429 : Spotify demande de patienter (durée de `Retry-After`, si elle est donnée).
    RateLimited(Option<Duration>),
    Http(u16),
    Network(String),
    Decode(String),
}

impl Fault {
    /// Erreur de librespot : sa propre limite de débit (ou un 429 qu'il n'a pas voulu attendre) devient
    /// [`Fault::RateLimited`].
    pub fn from_librespot(e: &librespot_core::Error) -> Fault {
        if e.kind == librespot_core::error::ErrorKind::ResourceExhausted {
            Fault::RateLimited(None)
        } else {
            Fault::Network(e.to_string())
        }
    }
}

/// Avis affiché en tête de liste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// Spotify demande de patienter jusqu'à `until` (secondes depuis 1970). `partial` : seule une partie de la
    /// réponse manque (albums et playlists d'une recherche).
    Wait { until: u64, partial: bool },
    NotConnected,
    Failed(Fault),
}

/// Une réponse de la Bibliothèque. Plusieurs peuvent arriver pour une même requête : la liste gardée d'abord,
/// puis la liste fraîche ; ou une liste en cours de chargement, puis complète.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryReply {
    pub items: Vec<LibraryItem>,
    /// `items` est une liste entière (gardée ou fraîche), pas un début de chargement.
    pub complete: bool,
    /// La liste vient du disque : son âge, en secondes.
    pub cached_age: Option<u64>,
    pub notice: Option<Notice>,
    /// Chargement en cours : éléments prêts, sur combien.
    pub progress: Option<(usize, usize)>,
    /// Plus rien n'arrivera pour cette requête.
    pub last: bool,
    /// Liste qu'on peut actualiser (ligne « Actualiser »).
    pub refreshable: bool,
}

impl LibraryReply {
    pub fn done(items: Vec<LibraryItem>) -> Self {
        LibraryReply { items, complete: true, cached_age: None, notice: None, progress: None, last: true, refreshable: false }
    }

    pub fn failed(notice: Notice) -> Self {
        LibraryReply { items: Vec::new(), complete: false, cached_age: None, notice: Some(notice), progress: None, last: true, refreshable: false }
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// --- Listes gardées sur disque ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct StoredItem {
    k: String,
    t: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    s: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    d: Option<u64>,
    u: String,
}

#[derive(Serialize, Deserialize)]
struct StoredList {
    saved: u64,
    items: Vec<StoredItem>,
}

fn kind_name(k: LibraryKind) -> &'static str {
    match k {
        LibraryKind::Track => "track",
        LibraryKind::Playlist => "playlist",
        LibraryKind::Album => "album",
        LibraryKind::Radio => "radio",
        LibraryKind::Action => "action",
        LibraryKind::Info => "info",
    }
}

fn kind_from(s: &str) -> Option<LibraryKind> {
    Some(match s {
        "track" => LibraryKind::Track,
        "playlist" => LibraryKind::Playlist,
        "album" => LibraryKind::Album,
        "radio" => LibraryKind::Radio,
        _ => return None,
    })
}

/// Listes gardées dans `<dossier Spotify>/bibliotheque/`, une par fichier JSON.
pub struct ListCache {
    dir: PathBuf,
}

impl ListCache {
    pub fn new(spotify_dir: &Path) -> ListCache {
        ListCache { dir: spotify_dir.join("bibliotheque") }
    }

    fn path(&self, key: &str) -> PathBuf {
        let safe: String = key.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
        self.dir.join(format!("{safe}.json"))
    }

    /// La liste gardée et son âge (secondes).
    pub fn load(&self, key: &str) -> Option<(Vec<LibraryItem>, u64)> {
        let b = std::fs::read(self.path(key)).ok()?;
        let s: StoredList = serde_json::from_slice(&b).ok()?;
        let items = s
            .items
            .into_iter()
            .filter_map(|i| {
                let mut it = LibraryItem::new(kind_from(&i.k)?, i.t, i.s, i.u);
                it.duration_ms = i.d;
                Some(it)
            })
            .collect();
        Some((items, now_secs().saturating_sub(s.saved)))
    }

    pub fn store(&self, key: &str, items: &[LibraryItem]) {
        let list = StoredList {
            saved: now_secs(),
            items: items
                .iter()
                .filter(|i| kind_from(kind_name(i.kind)).is_some())
                .map(|i| StoredItem { k: kind_name(i.kind).into(), t: i.title.clone(), s: i.subtitle.clone(), d: i.duration_ms, u: i.uri.clone() })
                .collect(),
        };
        if std::fs::create_dir_all(&self.dir).is_err() {
            return;
        }
        if let Ok(s) = serde_json::to_vec(&list) {
            let p = self.path(key);
            let tmp = p.with_extension("tmp");
            if std::fs::write(&tmp, s).is_ok() {
                let _ = std::fs::rename(&tmp, &p);
            }
        }
    }

    /// Oublie tout (déconnexion).
    pub fn clear(&self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }

    /// Attentes demandées par Spotify, par canal (gardées pour qu'un redémarrage n'insiste pas).
    pub fn load_waits(&self) -> HashMap<String, u64> {
        std::fs::read(self.dir.join("attente.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn store_waits(&self, w: &HashMap<String, u64>) {
        let _ = std::fs::create_dir_all(&self.dir);
        if let Ok(s) = serde_json::to_vec(w) {
            let _ = std::fs::write(self.dir.join("attente.json"), s);
        }
    }
}

/// Clé de la liste gardée pour une adresse de contenant (`None` : pas gardée).
pub fn cache_key_for_uri(uri: &str) -> Option<String> {
    if uri == "spotify:collection" {
        Some("likes".into())
    } else if let Some(id) = uri.strip_prefix("spotify:playlist:") {
        Some(format!("playlist-{id}"))
    } else {
        uri.strip_prefix("spotify:album:").map(|id| format!("album-{id}"))
    }
}

// --- Lignes affichées ------------------------------------------------------------------------------------

/// Préfixe de l'adresse d'une ligne de compte à rebours : `notice:wait:<échéance>:<partiel>`.
pub const WAIT_ROW: &str = "notice:wait:";
/// Adresse de la ligne « Actualiser ».
pub const REFRESH_ROW: &str = "action:refresh";

/// « 11:42 », « 1:05:09 ».
pub fn countdown(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs / 60) % 60, secs % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

/// « il y a 12 min ».
pub fn age_text(secs: u64) -> String {
    let (key, n) = match secs {
        0..60 => return t("lib.age_now").to_string(),
        60..3600 => ("lib.age_min", secs / 60),
        3600..86_400 => ("lib.age_hours", secs / 3600),
        _ => ("lib.age_days", secs / 86_400),
    };
    t(key).replace("{n}", &n.to_string())
}

/// Texte d'une erreur, dans la langue de l'interface.
pub fn fault_text(f: &Fault) -> String {
    match f {
        Fault::NotConnected => t("lib.not_connected").to_string(),
        Fault::RateLimited(_) => t("lib.wait").replace("{t}", "…"),
        Fault::Http(code) => t("lib.http").replace("{code}", &code.to_string()),
        Fault::Network(e) | Fault::Decode(e) => t("lib.network").replace("{e}", e),
    }
}

/// La ligne d'un avis (compte à rebours : texte à l'instant `now`).
fn notice_row(n: &Notice, now: u64) -> LibraryItem {
    match n {
        Notice::Wait { until, partial } => wait_row(*until, *partial, now),
        Notice::NotConnected => LibraryItem::info(t("lib.not_connected")),
        Notice::Failed(f) => LibraryItem::info(fault_text(f)),
    }
}

fn wait_row(until: u64, partial: bool, now: u64) -> LibraryItem {
    let uri = format!("{WAIT_ROW}{until}:{}", partial as u8);
    if now >= until {
        // L'heure est passée : la ligne devient « Réessayer maintenant ».
        return LibraryItem::new(LibraryKind::Action, t("lib.retry"), "", format!("{REFRESH_ROW}#{uri}"));
    }
    let key = if partial { "lib.wait_partial" } else { "lib.wait" };
    LibraryItem::new(LibraryKind::Info, t(key).replace("{t}", &countdown(until - now)), "", uri)
}

/// Les lignes à afficher pour une réponse : avis, « Actualiser » (avec l'âge de la liste), puis la liste.
pub fn rows(reply: &LibraryReply, now: u64) -> Vec<LibraryItem> {
    let mut out = Vec::new();
    if let Some(n) = &reply.notice {
        out.push(notice_row(n, now));
    }
    let waiting = matches!(&reply.notice, Some(Notice::Wait { until, partial: false }) if *until > now);
    if reply.refreshable && reply.last && !waiting && (reply.complete || reply.notice.is_none()) {
        let age = reply.cached_age.map(age_text).unwrap_or_default();
        out.push(LibraryItem::new(LibraryKind::Action, t("lib.refresh"), age, REFRESH_ROW));
    }
    if reply.items.is_empty() && reply.notice.is_none() && reply.complete {
        out.push(LibraryItem::info(t("lib.empty")));
    }
    out.extend(reply.items.iter().cloned());
    out
}

/// Met à jour les comptes à rebours des lignes affichées ; renvoie `true` si quelque chose a changé.
pub fn tick_rows(rows: &mut [LibraryItem], now: u64) -> bool {
    let mut changed = false;
    for r in rows.iter_mut() {
        let Some(rest) = r.uri.strip_prefix(WAIT_ROW) else { continue };
        let Some((until, partial)) = rest.split_once(':') else { continue };
        let Ok(until) = until.parse::<u64>() else { continue };
        let fresh = wait_row(until, partial == "1", now);
        if fresh != *r {
            *r = fresh;
            changed = true;
        }
    }
    changed
}

/// Une ligne cliquée est-elle « Actualiser » / « Réessayer » ?
pub fn is_refresh_row(uri: &str) -> bool {
    uri == REFRESH_ROW || uri.starts_with(&format!("{REFRESH_ROW}#"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(n: &str) -> LibraryItem {
        let mut it = LibraryItem::new(LibraryKind::Track, n, "A", format!("spotify:track:{n}"));
        it.duration_ms = Some(1000);
        it
    }

    #[test]
    fn lists_are_kept_on_disk() {
        let dir = std::env::temp_dir().join(format!("alpagamp-cache-{}", std::process::id()));
        let c = ListCache::new(&dir);
        assert!(c.load("likes").is_none());
        c.store("likes", &[track("a"), LibraryItem::info("x"), track("b")]);
        let (items, age) = c.load("likes").unwrap();
        assert_eq!(items, vec![track("a"), track("b")]);
        assert!(age < 5);
        let mut w = HashMap::new();
        w.insert("web".to_string(), 42u64);
        c.store_waits(&w);
        assert_eq!(c.load_waits().get("web"), Some(&42));
        c.clear();
        assert!(c.load("likes").is_none());
        assert_eq!(cache_key_for_uri("spotify:playlist:abc").as_deref(), Some("playlist-abc"));
        assert_eq!(cache_key_for_uri("spotify:collection").as_deref(), Some("likes"));
        assert!(cache_key_for_uri("spotify:artist:x").is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn countdown_rows_tick_and_turn_into_retry() {
        alpagamp_core::i18n::set_lang(alpagamp_core::i18n::Lang::Fr);
        assert_eq!(countdown(702), "11:42");
        assert_eq!(countdown(3909), "1:05:09");
        let reply = LibraryReply { items: vec![track("a")], complete: true, cached_age: Some(720), notice: Some(Notice::Wait { until: 1000, partial: false }), progress: None, last: true, refreshable: true };
        let mut r = rows(&reply, 298);
        assert_eq!(r[0].title, "Spotify demande de patienter : réessaie dans 11:42");
        // Pendant l'attente, pas de ligne « Actualiser » : seulement l'avis et la liste gardée.
        assert_eq!(r.len(), 2);
        assert!(!tick_rows(&mut r, 298));
        assert!(tick_rows(&mut r, 299));
        assert_eq!(r[0].title, "Spotify demande de patienter : réessaie dans 11:41");
        assert!(tick_rows(&mut r, 1000));
        assert_eq!((r[0].kind, r[0].title.as_str()), (LibraryKind::Action, "Réessayer maintenant"));
        assert!(is_refresh_row(&r[0].uri));
        // Liste gardée, sans avis : « Actualiser » avec son âge.
        let reply = LibraryReply { notice: None, ..reply };
        let r = rows(&reply, 0);
        assert_eq!((r[0].title.as_str(), r[0].subtitle.as_str()), ("Actualiser", "il y a 12 min"));
        assert!(is_refresh_row(&r[0].uri));
        alpagamp_core::i18n::set_lang(alpagamp_core::i18n::Lang::En);
    }
}
