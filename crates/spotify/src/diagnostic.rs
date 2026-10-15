//! Diagnostic de Spotify (`alpagamp --diagnostic-spotify`) : avec la connexion déjà enregistrée, une poignée de
//! requêtes en lecture seule sur chaque canal ; le rapport ne contient que des codes de réponse, des attentes,
//! des nombres et des durées — jamais de jeton ni d'identifiant.

use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use librespot_core::cache::Cache;
use librespot_core::{Session, SessionConfig};

use crate::internal;
use crate::oauth::{self, OAuthToken};
use crate::webapi;
use crate::Fault;

fn fault(f: &Fault) -> String {
    match f {
        Fault::RateLimited(Some(d)) => format!("429, Retry-After {} s ({})", d.as_secs(), crate::library::countdown(d.as_secs())),
        Fault::RateLimited(None) => "429 (sans Retry-After) ou limite interne de librespot".into(),
        Fault::Http(s) => format!("HTTP {s}"),
        Fault::Network(e) => format!("réseau : {e}"),
        Fault::Decode(e) => format!("réponse illisible : {e}"),
        Fault::NotConnected => "pas connecté".into(),
    }
}

fn secs(t: Instant) -> String {
    format!("{:.1} s", t.elapsed().as_secs_f32())
}

/// Lance le diagnostic sur le dossier Spotify de l'appli ; renvoie le rapport.
pub fn run(dir: &Path) -> String {
    let mut r = String::new();
    let _ = writeln!(r, "Alpagamp {} — diagnostic Spotify", env!("CARGO_PKG_VERSION"));
    let rt = match tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            let _ = writeln!(r, "tokio : {e}");
            return r;
        }
    };
    let cache = Cache::new(Some(dir.join("credentials")), None, None, None).ok();
    let Some(creds) = cache.as_ref().and_then(|c| c.credentials()) else {
        let _ = writeln!(r, "Pas d'identifiants gardés : connecte-toi d'abord dans Alpagamp (Bibliothèque › Se connecter).");
        return r;
    };
    let device_id = std::fs::read_to_string(dir.join("device-id")).map(|s| s.trim().to_string()).unwrap_or_else(|_| "0000000000000000diagnostic".into());
    let config = SessionConfig { client_id: oauth::CLIENT_ID.to_string(), device_id, ..SessionConfig::default() };
    // Session::new démarre des tâches : il faut être dans le contexte du runtime.
    let _ctx = rt.enter();
    let session = Session::new(config, cache);
    let t = Instant::now();
    match rt.block_on(session.connect(creds, false)) {
        Ok(()) => {
            let _ = writeln!(r, "Session librespot : ouverte ({})", secs(t));
        }
        Err(e) => {
            let _ = writeln!(r, "Session librespot : échec ({e})");
            return r;
        }
    }

    // --- API web (api.spotify.com) ---
    let _ = writeln!(r, "\n[API web]");
    let mut oauth_token: Option<OAuthToken> = std::fs::read(dir.join("oauth.json")).ok().and_then(|b| serde_json::from_slice(&b).ok());
    if let Some(tok) = &oauth_token {
        if tok.is_expired() {
            match oauth::refresh(tok) {
                Ok(n) => {
                    // Le jeton de renouvellement peut changer : on garde le nouveau, comme l'appli.
                    if let Ok(s) = serde_json::to_string(&n) {
                        let _ = std::fs::write(dir.join("oauth.json"), s);
                    }
                    let _ = writeln!(r, "jeton de connexion : renouvelé");
                    oauth_token = Some(n);
                }
                Err(e) => {
                    let _ = writeln!(r, "jeton de connexion : renouvellement refusé ({e})");
                    oauth_token = None;
                }
            }
        } else {
            let _ = writeln!(r, "jeton de connexion : valide");
        }
    } else {
        let _ = writeln!(r, "jeton de connexion : absent");
    }
    let probe = |r: &mut String, label: &str, token: &str, path: &str| {
        let t = Instant::now();
        let (status, retry, reason) = webapi::probe(token, path);
        let status = status.map(|s| s.to_string()).unwrap_or_else(|| "pas de réponse".into());
        let retry = retry.map(|v| format!(", Retry-After {v} s")).unwrap_or_default();
        let reason = if reason.is_empty() { String::new() } else { format!(", « {reason} »") };
        let _ = writeln!(r, "{label} {path} : {status}{retry}{reason} ({})", secs(t));
    };
    if let Some(tok) = &oauth_token {
        probe(&mut r, "jeton de connexion", &tok.access_token, "/me");
        probe(&mut r, "jeton de connexion", &tok.access_token, "/me/playlists?limit=1");
    }
    match rt.block_on(session.login5().auth_token()) {
        Ok(t5) => probe(&mut r, "jeton de session (login5)", &t5.access_token, "/me"),
        Err(e) => {
            let _ = writeln!(r, "jeton de session (login5) : échec ({e})");
        }
    }

    // --- Canaux internes ---
    let _ = writeln!(r, "\n[Canaux internes]");
    let t = Instant::now();
    let playlists = rt.block_on(internal::rootlist(&session));
    match &playlists {
        Ok(p) => {
            let named = p.iter().filter(|i| !i.title.starts_with("spotify:")).count();
            let _ = writeln!(r, "playlists (rootlist) : {} trouvées, {} avec leur nom ({})", p.len(), named, secs(t));
        }
        Err(f) => {
            let _ = writeln!(r, "playlists (rootlist) : {} ({})", fault(f), secs(t));
        }
    }
    for (i, pl) in playlists.as_ref().map(|p| p.iter().take(3).collect::<Vec<_>>()).unwrap_or_default().into_iter().enumerate() {
        let id = pl.uri.trim_start_matches("spotify:playlist:").to_string();
        let t = Instant::now();
        match rt.block_on(internal::playlist_uris(&session, &id)) {
            Ok((u, announced)) => {
                let _ = writeln!(r, "playlist n° {} : {} morceaux ou épisodes lus, {} annoncés ({})", i + 1, u.len(), announced, secs(t));
            }
            Err(f) => {
                let _ = writeln!(r, "playlist n° {} : {} ({})", i + 1, fault(&f), secs(t));
            }
        }
    }
    let t = Instant::now();
    let coll = rt.block_on(internal::collection(&session));
    let mut liked = Vec::new();
    let mut albums = Vec::new();
    match &coll {
        Ok(c) => {
            liked = c.iter().filter(|i| i.uri.starts_with("spotify:track:")).map(|i| i.uri.clone()).collect();
            albums = c.iter().filter(|i| i.uri.starts_with("spotify:album:")).map(|i| i.uri.clone()).collect();
            let other = c.len() - liked.len() - albums.len();
            let _ = writeln!(r, "collection : {} titres likés, {} albums, {} autres ({})", liked.len(), albums.len(), other, secs(t));
        }
        Err(f) => {
            let _ = writeln!(r, "collection : {} ({})", fault(f), secs(t));
        }
    }
    if !liked.is_empty() {
        let first: Vec<String> = liked.iter().take(internal::BATCH).cloned().collect();
        let t = Instant::now();
        match rt.block_on(internal::metadata(&session, &first, |_| {})) {
            Ok(m) => {
                let named = m.values().filter(|x| !x.name.is_empty()).count();
                let _ = writeln!(r, "métadonnées (paquet de {}) : {} noms lus ({})", first.len(), named, secs(t));
            }
            Err(f) => {
                let _ = writeln!(r, "métadonnées : {} ({})", fault(&f), secs(t));
            }
        }
    }
    if let Some(a) = albums.first() {
        let t = Instant::now();
        match rt.block_on(internal::album_uris(&session, a)) {
            Ok((u, _)) => {
                let _ = writeln!(r, "1er album enregistré : {} morceaux ({})", u.len(), secs(t));
            }
            Err(f) => {
                let _ = writeln!(r, "1er album enregistré : {} ({})", fault(&f), secs(t));
            }
        }
    }
    let t = Instant::now();
    match rt.block_on(internal::search_tracks(&session, "camera silens")) {
        Ok(u) => {
            let named = rt.block_on(internal::metadata(&session, &u, |_| {})).map(|m| m.values().filter(|x| !x.name.is_empty()).count()).unwrap_or(0);
            let _ = writeln!(r, "recherche de titres (« camera silens ») : {} titres, {} nommés ({})", u.len(), named, secs(t));
        }
        Err(f) => {
            let _ = writeln!(r, "recherche de titres : {} ({})", fault(&f), secs(t));
        }
    }
    session.shutdown();
    let _ = writeln!(r, "\nFin du diagnostic.");
    r
}
