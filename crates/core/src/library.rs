//! Éléments de la Bibliothèque (Spotify, radios) : ce que la fenêtre affiche et ce qu'on peut en faire.

/// Nature d'un élément.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryKind {
    /// Morceau jouable (`uri` : `spotify:track:…`).
    Track,
    /// Contenant qu'on ouvre (playlist, album, titres likés).
    Playlist,
    Album,
    /// Radio Internet (`uri` : adresse du flux).
    Radio,
    /// Ligne d'action (« Se connecter à Spotify »).
    Action,
    /// Ligne d'information (chargement, erreur).
    Info,
}

/// Une ligne de la Bibliothèque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryItem {
    pub kind: LibraryKind,
    pub title: String,
    /// Artiste, propriétaire, pays…
    pub subtitle: String,
    pub duration_ms: Option<u64>,
    pub uri: String,
}

impl LibraryItem {
    pub fn new(kind: LibraryKind, title: impl Into<String>, subtitle: impl Into<String>, uri: impl Into<String>) -> Self {
        LibraryItem { kind, title: title.into(), subtitle: subtitle.into(), duration_ms: None, uri: uri.into() }
    }

    pub fn info(text: impl Into<String>) -> Self {
        LibraryItem::new(LibraryKind::Info, text, "", "")
    }

    /// Titre affiché dans la playlist : « Artiste - Titre ».
    pub fn playlist_title(&self) -> String {
        if self.subtitle.is_empty() { self.title.clone() } else { format!("{} - {}", self.subtitle, self.title) }
    }

    pub fn is_playable(&self) -> bool {
        matches!(self.kind, LibraryKind::Track | LibraryKind::Radio)
    }

    pub fn is_container(&self) -> bool {
        matches!(self.kind, LibraryKind::Playlist | LibraryKind::Album)
    }
}
