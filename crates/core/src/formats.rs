//! Formats de playlist : M3U, M3U étendu, M3U8, PLS.

use std::path::Path;

use crate::playlist::Entry;
use crate::source::Source;

/// Lit une playlist ; le format est détecté par la première ligne (`[playlist]` → PLS, sinon M3U).
pub fn read_playlist(text: &str, base: Option<&Path>) -> Vec<Entry> {
    let text = text.trim_start_matches('\u{FEFF}');
    let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_ascii_lowercase();
    if first == "[playlist]" { read_pls(text, base) } else { read_m3u(text, base) }
}

/// Lit des octets de playlist (UTF-8, sinon Latin-1).
pub fn read_playlist_bytes(bytes: &[u8], base: Option<&Path>) -> Vec<Entry> {
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    };
    read_playlist(&text, base)
}

fn read_m3u(text: &str, base: Option<&Path>) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut pending: Option<(Option<u64>, Option<String>)> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(info) = line.strip_prefix("#EXTINF:") {
            let (dur, title) = match info.split_once(',') {
                Some((d, t)) => (d.trim(), Some(t.trim().to_string())),
                None => (info.trim(), None),
            };
            // La durée peut être suivie d'attributs (« 123 tvg-id=… ») : on garde le premier nombre.
            let secs = dur.split_whitespace().next().and_then(|d| d.parse::<f64>().ok());
            let ms = secs.filter(|s| *s >= 0.0).map(|s| (s * 1000.0).round() as u64);
            pending = Some((ms, title.filter(|t| !t.is_empty())));
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let source = Source::parse(line, base);
        let mut e = Entry::new(source);
        if let Some((ms, title)) = pending.take() {
            e.duration_ms = ms;
            e.title = title;
            e.meta_loaded = e.title.is_some();
        }
        out.push(e);
    }
    out
}

fn read_pls(text: &str, base: Option<&Path>) -> Vec<Entry> {
    use std::collections::BTreeMap;
    #[derive(Default)]
    struct P {
        file: Option<String>,
        title: Option<String>,
        length: Option<i64>,
    }
    let mut map: BTreeMap<u32, P> = BTreeMap::new();
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let k = k.trim().to_ascii_lowercase();
        let v = v.trim().to_string();
        let (field, num) = if let Some(n) = k.strip_prefix("file") {
            ("file", n)
        } else if let Some(n) = k.strip_prefix("title") {
            ("title", n)
        } else if let Some(n) = k.strip_prefix("length") {
            ("length", n)
        } else {
            continue;
        };
        let Ok(n) = num.parse::<u32>() else { continue };
        let p = map.entry(n).or_default();
        match field {
            "file" => p.file = Some(v),
            "title" => p.title = Some(v).filter(|t| !t.is_empty()),
            _ => p.length = v.parse().ok(),
        }
    }
    map.into_values()
        .filter_map(|p| {
            let file = p.file?;
            let mut e = Entry::new(Source::parse(&file, base));
            e.title = p.title;
            e.meta_loaded = e.title.is_some();
            e.duration_ms = p.length.filter(|l| *l >= 0).map(|l| l as u64 * 1000);
            Some(e)
        })
        .collect()
}

/// Écrit une M3U étendue. `utf8_bom` pour le `.m3u8` (comme Winamp).
pub fn write_m3u(entries: &[Entry], utf8_bom: bool) -> String {
    let mut s = String::new();
    if utf8_bom {
        s.push('\u{FEFF}');
    }
    s.push_str("#EXTM3U\n");
    for e in entries {
        if let Some(t) = &e.title {
            let secs = e.duration_ms.map(|d| (d / 1000) as i64).unwrap_or(-1);
            s.push_str(&format!("#EXTINF:{secs},{t}\n"));
        }
        s.push_str(&e.source.to_playlist_string());
        s.push('\n');
    }
    s
}

/// Écrit un PLS (version 2, fins de ligne CRLF).
pub fn write_pls(entries: &[Entry]) -> String {
    let mut s = String::from("[playlist]\r\n");
    for (i, e) in entries.iter().enumerate() {
        let n = i + 1;
        s.push_str(&format!("File{n}={}\r\n", e.source.to_playlist_string()));
        if let Some(t) = &e.title {
            s.push_str(&format!("Title{n}={t}\r\n"));
        }
        match (e.duration_ms, e.source.is_stream()) {
            (Some(d), _) => s.push_str(&format!("Length{n}={}\r\n", d / 1000)),
            (None, true) => s.push_str(&format!("Length{n}=-1\r\n")),
            _ => {}
        }
    }
    s.push_str(&format!("NumberOfEntries={}\r\nVersion=2\r\n", entries.len()));
    s
}

/// Extensions reconnues comme playlists.
pub fn is_playlist_path(p: &Path) -> bool {
    matches!(p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("m3u" | "m3u8" | "pls"))
}

/// Extensions audio reconnues (fichiers locaux).
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "mp2", "mp1", "flac", "ogg", "oga", "opus", "wav", "wave", "aif", "aiff", "m4a", "mp4", "aac", "alac", "caf", "mka", "webm"];

pub fn is_audio_path(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).map(|e| AUDIO_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str())).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn m3u_roundtrip() {
        let txt = "#EXTM3U\n#EXTINF:215,Artiste - Titre\n/music/a.mp3\nhttp://radio/stream\n";
        let e = read_playlist(txt, None);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].title.as_deref(), Some("Artiste - Titre"));
        assert_eq!(e[0].duration_ms, Some(215_000));
        assert_eq!(e[1].source, Source::Url("http://radio/stream".into()));
        let out = write_m3u(&e, false);
        assert_eq!(read_playlist(&out, None), e);
    }

    #[test]
    fn pls_parse_with_gaps_and_stream() {
        let txt = "[playlist]\nFile2=b.mp3\nFile1=http://x/s\nTitle1=Radio\nLength1=-1\nLength2=60\nNumberOfEntries=2\n";
        let e = read_playlist(txt, Some(Path::new("/base")));
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].title.as_deref(), Some("Radio"));
        assert_eq!(e[0].duration_ms, None);
        assert_eq!(e[1].source, Source::File(PathBuf::from("/base/b.mp3")));
        assert_eq!(e[1].duration_ms, Some(60_000));
        let again = read_playlist(&write_pls(&e), Some(Path::new("/base")));
        assert_eq!(again.len(), 2);
    }

    #[test]
    fn bom_and_crlf() {
        let e = read_playlist("\u{FEFF}#EXTM3U\r\n#EXTINF:-1,Live\r\nhttp://a/b\r\n", None);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].duration_ms, None);
        assert_eq!(e[0].title.as_deref(), Some("Live"));
    }
}
