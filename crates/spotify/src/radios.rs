//! Annuaire libre de radios (radio-browser.info) pour l'onglet Radios de la Bibliothèque.

use alpagamp_core::library::{LibraryItem, LibraryKind};
use alpagamp_net::{Request, fetch, form_encode};
use serde_json::Value;

/// Serveurs miroirs, essayés dans l'ordre.
const MIRRORS: &[&str] = &["de1.api.radio-browser.info", "de2.api.radio-browser.info", "fi1.api.radio-browser.info", "nl1.api.radio-browser.info"];

fn query(path: &str) -> Result<Value, String> {
    let mut last = String::from("annuaire injoignable");
    for m in MIRRORS {
        match fetch(&format!("https://{m}{path}"), &Request::get()) {
            Ok((r, body)) if r.status == 200 => return serde_json::from_slice(&body).map_err(|e| e.to_string()),
            Ok((r, _)) => last = format!("l'annuaire répond {}", r.status),
            Err(e) => last = e.0,
        }
    }
    Err(last)
}

fn items(v: &Value) -> Vec<LibraryItem> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| {
            let name = s.get("name")?.as_str()?.trim();
            let url = s.get("url_resolved").and_then(Value::as_str).filter(|u| !u.is_empty()).or(s.get("url").and_then(Value::as_str))?;
            if name.is_empty() || url.contains(".m3u8") {
                return None;
            }
            let country = s.get("countrycode").and_then(Value::as_str).unwrap_or("");
            let codec = s.get("codec").and_then(Value::as_str).unwrap_or("");
            let br = s.get("bitrate").and_then(Value::as_u64).filter(|b| *b > 0).map(|b| format!(" {b}k")).unwrap_or_default();
            let sub = [country, &format!("{codec}{br}")].iter().filter(|x| !x.trim().is_empty()).cloned().collect::<Vec<_>>().join(" · ");
            Some(LibraryItem::new(LibraryKind::Radio, name, sub, url))
        })
        .collect()
}

/// Radios les plus écoutées.
pub fn top() -> Result<Vec<LibraryItem>, String> {
    query("/json/stations/topclick/150?hidebroken=true").map(|v| items(&v))
}

/// Recherche par nom (ou genre).
pub fn search(q: &str) -> Result<Vec<LibraryItem>, String> {
    let mut v = items(&query(&format!("/json/stations/search?name={}&limit=150&hidebroken=true&order=clickcount&reverse=true", form_encode(q)))?);
    if v.is_empty() {
        v = items(&query(&format!("/json/stations/search?tag={}&limit=150&hidebroken=true&order=clickcount&reverse=true", form_encode(q)))?);
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stations() {
        let v: Value = serde_json::from_str(
            r#"[{"name":" FIP ","url":"http://a","url_resolved":"https://icecast.radiofrance.fr/fip-midfi.mp3","countrycode":"FR","codec":"MP3","bitrate":128},
                {"name":"HLS","url_resolved":"https://x/live.m3u8"},{"name":"","url":"http://b"}]"#,
        )
        .unwrap();
        let it = items(&v);
        assert_eq!(it.len(), 1);
        assert_eq!((it[0].title.as_str(), it[0].subtitle.as_str()), ("FIP", "FR · MP3 128k"));
    }

    /// Annuaire réel : `cargo test -p alpagamp-spotify real_directory -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_directory() {
        let top = top().unwrap();
        eprintln!("{} radios, ex. {:?}", top.len(), top.first());
        assert!(top.len() > 50);
        let s = search("jazz").unwrap();
        eprintln!("jazz : {} radios, ex. {:?}", s.len(), s.first());
        assert!(!s.is_empty());
    }
}
