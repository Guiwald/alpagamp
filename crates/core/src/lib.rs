//! Modèle du lecteur Alpagamp, indépendant de l'interface et de l'audio.

pub mod eq;
pub mod formats;
pub mod playlist;
pub mod settings;
pub mod source;
pub mod time;

pub use playlist::{Entry, Playlist, SortKey, TotalTime};
pub use settings::Settings;
pub use source::Source;
