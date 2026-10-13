//! Parcours de la bibliothèque Spotify par l'API web (playlists, titres likés, albums, recherche).

use alpagamp_core::library::{LibraryItem, LibraryKind};
use alpagamp_net::{Request, fetch, form_encode};
use serde_json::Value;

const API: &str = "https://api.spotify.com/v1";
/// Plafond de lignes chargées par liste (pagination).
const MAX_ITEMS: usize = 2000;

fn get(token: &str, url: &str) -> Result<Value, String> {
    let (resp, body) = fetch(url, &Request::get().header("Authorization", format!("Bearer {token}"))).map_err(|e| e.0)?;
    match resp.status {
        200 => serde_json::from_slice(&body).map_err(|e| format!("réponse illisible : {e}")),
        401 => Err("session Spotify expirée".into()),
        403 => Err("accès refusé par Spotify".into()),
        404 => Err("introuvable".into()),
        429 => Err("Spotify demande de patienter (trop de requêtes)".into()),
        s => Err(format!("Spotify répond {s}")),
    }
}

/// Suit la pagination (`next`) et applique `f` à chaque élément de `items`.
fn paged(token: &str, first: String, mut f: impl FnMut(&Value)) -> Result<(), String> {
    let mut url = Some(first);
    let mut count = 0;
    while let Some(u) = url.take() {
        let v = get(token, &u)?;
        // La recherche range ses résultats sous une clé (« tracks », « albums »…).
        let page = if v.get("items").is_some() { &v } else { v.as_object().and_then(|o| o.values().next()).unwrap_or(&v) };
        for it in page.get("items").and_then(Value::as_array).into_iter().flatten() {
            f(it);
            count += 1;
        }
        if count >= MAX_ITEMS {
            break;
        }
        url = page.get("next").and_then(Value::as_str).map(str::to_string);
    }
    Ok(())
}

fn artists(v: &Value) -> String {
    v.get("artists").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.get("name").and_then(Value::as_str)).collect::<Vec<_>>().join(", ")).unwrap_or_default()
}

fn track_item(t: &Value) -> Option<LibraryItem> {
    let uri = t.get("uri")?.as_str()?;
    if !uri.starts_with("spotify:track:") && !uri.starts_with("spotify:episode:") {
        return None;
    }
    let mut it = LibraryItem::new(LibraryKind::Track, t.get("name").and_then(Value::as_str).unwrap_or("?"), artists(t), uri);
    it.duration_ms = t.get("duration_ms").and_then(Value::as_u64);
    Some(it)
}

/// Mes playlists (plus « Titres likés » en tête).
pub fn playlists(token: &str, liked_label: &str) -> Result<Vec<LibraryItem>, String> {
    let mut out = vec![LibraryItem::new(LibraryKind::Playlist, liked_label, "", "spotify:collection")];
    paged(token, format!("{API}/me/playlists?limit=50"), |p| {
        if let (Some(name), Some(uri)) = (p.get("name").and_then(Value::as_str), p.get("uri").and_then(Value::as_str)) {
            let owner = p.pointer("/owner/display_name").and_then(Value::as_str).unwrap_or("");
            out.push(LibraryItem::new(LibraryKind::Playlist, name, owner, uri));
        }
    })?;
    Ok(out)
}

/// Morceaux d'une playlist (`spotify:playlist:…`), des titres likés (`spotify:collection`) ou d'un album.
pub fn tracks_of(token: &str, uri: &str) -> Result<Vec<LibraryItem>, String> {
    let mut out = Vec::new();
    if uri == "spotify:collection" {
        paged(token, format!("{API}/me/tracks?limit=50"), |i| out.extend(i.get("track").and_then(track_item)))?;
    } else if let Some(id) = uri.strip_prefix("spotify:playlist:") {
        // L'API a renommé « tracks » en « items » ; on essaie les deux.
        let r = paged(token, format!("{API}/playlists/{id}/items?limit=100"), |i| out.extend(i.get("item").or(i.get("track")).and_then(track_item)));
        if r.is_err() && out.is_empty() {
            paged(token, format!("{API}/playlists/{id}/tracks?limit=100"), |i| out.extend(i.get("track").and_then(track_item)))?;
        }
    } else if let Some(id) = uri.strip_prefix("spotify:album:") {
        let album = get(token, &format!("{API}/albums/{id}"))?;
        let album_artist = artists(&album);
        paged(token, format!("{API}/albums/{id}/tracks?limit=50"), |t| {
            if let Some(mut it) = track_item(t) {
                if it.subtitle.is_empty() {
                    it.subtitle = album_artist.clone();
                }
                out.push(it);
            }
        })?;
    }
    Ok(out)
}

/// Mes albums.
pub fn albums(token: &str) -> Result<Vec<LibraryItem>, String> {
    let mut out = Vec::new();
    paged(token, format!("{API}/me/albums?limit=50"), |i| {
        if let Some(a) = i.get("album") {
            if let (Some(name), Some(uri)) = (a.get("name").and_then(Value::as_str), a.get("uri").and_then(Value::as_str)) {
                out.push(LibraryItem::new(LibraryKind::Album, name, artists(a), uri));
            }
        }
    })?;
    Ok(out)
}

/// Recherche : morceaux, puis albums et playlists.
pub fn search(token: &str, query: &str) -> Result<Vec<LibraryItem>, String> {
    let v = get(token, &format!("{API}/search?q={}&type=track,album,playlist&limit=30", form_encode(query)))?;
    let mut out = Vec::new();
    for t in v.pointer("/tracks/items").and_then(Value::as_array).into_iter().flatten() {
        out.extend(track_item(t));
    }
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
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tracks() {
        let v: Value = serde_json::from_str(r#"{"uri":"spotify:track:abc","name":"Song","duration_ms":123000,"artists":[{"name":"A"},{"name":"B"}]}"#).unwrap();
        let it = track_item(&v).unwrap();
        assert_eq!((it.title.as_str(), it.subtitle.as_str(), it.duration_ms), ("Song", "A, B", Some(123_000)));
        assert_eq!(it.playlist_title(), "A, B - Song");
        let local: Value = serde_json::from_str(r#"{"uri":"spotify:local:x","name":"L"}"#).unwrap();
        assert!(track_item(&local).is_none());
    }
}
