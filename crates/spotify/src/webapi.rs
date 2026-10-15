//! L'API web publique de Spotify (`api.spotify.com`), réduite depuis la 0.6.2 à ce que les canaux internes ne
//! savent pas faire : chercher des **albums et des playlists** (ADR 0022). Elle est fortement bridée pour les
//! jetons du lecteur de bureau : on lit `Retry-After` et on n'insiste pas.

use alpagamp_core::library::{LibraryItem, LibraryKind};
use alpagamp_net::{Request, fetch, form_encode};
use serde_json::Value;

use crate::Fault;
use crate::internal::parse_retry_after;

const API: &str = "https://api.spotify.com/v1";

/// Une requête GET ; une réponse 429 donne [`Fault::RateLimited`] avec l'attente demandée.
pub fn get(token: &str, url: &str) -> Result<Value, Fault> {
    let (resp, body) = fetch(url, &Request::get().header("Authorization", format!("Bearer {token}"))).map_err(|e| Fault::Network(e.0))?;
    classify(resp.status, resp.header("retry-after"), &body, url)
}

/// Interprète une réponse de l'API web.
pub fn classify(status: u16, retry_after: Option<&str>, body: &[u8], url: &str) -> Result<Value, Fault> {
    match status {
        200 => serde_json::from_slice(body).map_err(|e| Fault::Decode(e.to_string())),
        429 => {
            let wait = retry_after.and_then(parse_retry_after);
            // Depuis juillet 2026, le corps précise parfois « QUOTA_EXCEEDED ».
            let reason = serde_json::from_slice::<Value>(body).ok().and_then(|v| v.pointer("/error/reason").or(v.get("reason")).and_then(Value::as_str).map(str::to_string));
            log::warn!("Spotify (API web) : 429 sur {}, Retry-After {wait:?}, raison {reason:?}", url.split('?').next().unwrap_or(url));
            Err(Fault::RateLimited(wait))
        }
        s => {
            log::warn!("Spotify (API web) : {s} sur {}", url.split('?').next().unwrap_or(url));
            Err(Fault::Http(s))
        }
    }
}

fn artists(v: &Value) -> String {
    v.get("artists").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.get("name").and_then(Value::as_str)).collect::<Vec<_>>().join(", ")).unwrap_or_default()
}

/// Albums et playlists d'une réponse de recherche.
pub fn parse_search(v: &Value) -> Vec<LibraryItem> {
    let mut out = Vec::new();
    for a in v.pointer("/albums/items").and_then(Value::as_array).into_iter().flatten() {
        if let (Some(name), Some(uri)) = (a.get("name").and_then(Value::as_str), a.get("uri").and_then(Value::as_str)) {
            out.push(LibraryItem::new(LibraryKind::Album, name, artists(a), uri));
        }
    }
    for p in v.pointer("/playlists/items").and_then(Value::as_array).into_iter().flatten() {
        if let (Some(name), Some(uri)) = (p.get("name").and_then(Value::as_str), p.get("uri").and_then(Value::as_str)) {
            let owner = p.pointer("/owner/display_name").and_then(Value::as_str).unwrap_or("");
            out.push(LibraryItem::new(LibraryKind::Playlist, name, owner, uri));
        }
    }
    out
}

/// Recherche d'albums et de playlists (10 résultats de chaque au plus, la limite actuelle de Spotify).
pub fn search_albums_playlists(token: &str, query: &str) -> Result<Vec<LibraryItem>, Fault> {
    let v = get(token, &format!("{API}/search?q={}&type=album,playlist&limit=10", form_encode(query)))?;
    Ok(parse_search(&v))
}

/// Une requête de diagnostic : code de réponse et attente demandée, sans rien lire d'autre.
pub fn probe(token: &str, path: &str) -> (Option<u16>, Option<String>, String) {
    match fetch(&format!("{API}{path}"), &Request::get().header("Authorization", format!("Bearer {token}"))) {
        Ok((resp, body)) => {
            let reason = serde_json::from_slice::<Value>(&body).ok().and_then(|v| v.pointer("/error/reason").or(v.get("reason")).or(v.pointer("/error/message")).and_then(Value::as_str).map(str::to_string)).unwrap_or_default();
            (Some(resp.status), resp.header("retry-after").map(str::to_string), reason)
        }
        Err(e) => (None, None, e.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_is_read() {
        let r = classify(429, Some("3600"), br#"{"error":{"status":429,"message":"API rate limit exceeded"}}"#, "https://api.spotify.com/v1/me/playlists?limit=50");
        assert_eq!(r.unwrap_err(), Fault::RateLimited(Some(std::time::Duration::from_secs(3600))));
        assert_eq!(classify(429, None, b"", "x").unwrap_err(), Fault::RateLimited(None));
        assert_eq!(classify(403, None, b"", "x").unwrap_err(), Fault::Http(403));
    }

    #[test]
    fn search_is_parsed() {
        let v: Value = serde_json::from_str(
            r#"{"albums":{"items":[{"name":"Rien qu'en traînant","uri":"spotify:album:a","artists":[{"name":"Camera Silens"}]}]},
               "playlists":{"items":[null,{"name":"Punk FR","uri":"spotify:playlist:p","owner":{"display_name":"Bob"}}]}}"#,
        )
        .unwrap();
        let items = parse_search(&v);
        assert_eq!(items.len(), 2);
        assert_eq!((items[0].kind, items[0].subtitle.as_str()), (LibraryKind::Album, "Camera Silens"));
        assert_eq!((items[1].kind, items[1].subtitle.as_str()), (LibraryKind::Playlist, "Bob"));
    }
}
