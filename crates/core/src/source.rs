//! D'où vient un morceau.

use std::path::{Path, PathBuf};

/// La source d'une entrée de playlist.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Source {
    /// Fichier local.
    File(PathBuf),
    /// Flux réseau (radio HTTP/ICY, fichier distant).
    Url(String),
    /// Morceau Spotify (`spotify:track:…`).
    Spotify(String),
}

impl Source {
    /// Interprète une ligne de playlist ou un argument : URL, URI Spotify ou chemin.
    /// Les chemins relatifs sont résolus par rapport à `base` (dossier de la playlist).
    pub fn parse(s: &str, base: Option<&Path>) -> Source {
        let t = s.trim();
        let lower = t.to_ascii_lowercase();
        if lower.starts_with("spotify:") {
            return Source::Spotify(t.to_string());
        }
        if let Some(uri) = spotify_from_url(t) {
            return Source::Spotify(uri);
        }
        if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("icy://") {
            return Source::Url(t.to_string());
        }
        if let Some(rest) = t.strip_prefix("file://") {
            return Source::File(PathBuf::from(percent_decode(rest)));
        }
        let p = PathBuf::from(t.replace('\\', std::path::MAIN_SEPARATOR_STR));
        match base {
            Some(b) if p.is_relative() => Source::File(b.join(p)),
            _ => Source::File(p),
        }
    }

    /// Texte à écrire dans une playlist.
    pub fn to_playlist_string(&self) -> String {
        match self {
            Source::File(p) => p.display().to_string(),
            Source::Url(u) => u.clone(),
            Source::Spotify(u) => u.clone(),
        }
    }

    /// Titre de repli : nom du fichier sans extension, ou l'URL.
    pub fn fallback_title(&self) -> String {
        match self {
            Source::File(p) => p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string()),
            Source::Url(u) => u.clone(),
            Source::Spotify(u) => u.clone(),
        }
    }

    /// Nom de fichier (pour le tri « par nom de fichier »).
    pub fn file_name(&self) -> String {
        match self {
            Source::File(p) => p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
            Source::Url(u) | Source::Spotify(u) => u.rsplit('/').next().unwrap_or(u).to_string(),
        }
    }

    pub fn is_stream(&self) -> bool {
        matches!(self, Source::Url(_))
    }
}

/// `https://open.spotify.com/track/ID?si=…` → `spotify:track:ID`.
fn spotify_from_url(s: &str) -> Option<String> {
    let rest = s.strip_prefix("https://open.spotify.com/").or_else(|| s.strip_prefix("http://open.spotify.com/"))?;
    let rest = rest.split(['?', '#']).next()?;
    let mut parts = rest.split('/').filter(|p| !p.is_empty() && !p.starts_with("intl-"));
    let kind = parts.next()?;
    let id = parts.next()?;
    if ["track", "album", "playlist", "artist", "episode", "show"].contains(&kind) && !id.is_empty() {
        Some(format!("spotify:{kind}:{id}"))
    } else {
        None
    }
}

/// Décodage `%XX` minimal (chemins `file://`).
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kinds() {
        assert_eq!(Source::parse("spotify:track:abc", None), Source::Spotify("spotify:track:abc".into()));
        assert_eq!(
            Source::parse("https://open.spotify.com/intl-fr/track/4uLU6hMCjMI75M1A2tKUQC?si=x", None),
            Source::Spotify("spotify:track:4uLU6hMCjMI75M1A2tKUQC".into())
        );
        assert_eq!(Source::parse("http://radio.example/stream", None), Source::Url("http://radio.example/stream".into()));
        assert_eq!(Source::parse("file:///tmp/a%20b.mp3", None), Source::File(PathBuf::from("/tmp/a b.mp3")));
        assert_eq!(Source::parse("sub/x.mp3", Some(Path::new("/music"))), Source::File(PathBuf::from("/music/sub/x.mp3")));
    }
}
