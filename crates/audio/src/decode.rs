//! Décodage avec Symphonia : fichiers locaux (et toute source lisible).

use std::path::Path;

use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::{MetadataOptions, MetadataRevision, StandardTag};
use symphonia::core::units::Time;

use crate::AudioError;
use crate::readahead::ReadAhead;

/// Informations sur le morceau en cours de lecture.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackInfo {
    pub duration_ms: Option<u64>,
    pub sample_rate: u32,
    pub channels: u16,
    /// Débit moyen en kbit/s, si on peut l'estimer.
    pub bitrate_kbps: Option<u32>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<String>,
    pub comment: Option<String>,
    /// Gain ReplayGain du morceau et de l'album, en dB.
    pub rg_track_gain: Option<f32>,
    pub rg_album_gain: Option<f32>,
    pub rg_track_peak: Option<f32>,
    pub rg_album_peak: Option<f32>,
    pub seekable: bool,
    /// Nom du codec (« MP3 », « FLAC »…).
    pub codec: String,
    /// Nom de la radio (en-tête `icy-name`), pour un flux.
    pub stream_name: Option<String>,
}

impl TrackInfo {
    /// « Artiste - Titre », ou le titre seul.
    pub fn display_title(&self) -> Option<String> {
        match (&self.artist, &self.title) {
            (Some(a), Some(t)) if !a.is_empty() && !t.is_empty() => Some(format!("{a} - {t}")),
            (_, Some(t)) if !t.is_empty() => Some(t.clone()),
            _ => None,
        }
    }
}

/// Un décodeur ouvert sur une source.
pub struct Decoder {
    format: Box<dyn FormatReader + 'static>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    pub info: TrackInfo,
    scratch: Vec<f32>,
    eof: bool,
}

impl Decoder {
    /// Ouvre un fichier local **pour le jouer** : un fil le lit d'avance ([`ReadAhead`]), pour qu'un disque lent
    /// ne coupe pas la musique.
    pub fn open_file(path: &Path) -> Result<Decoder, AudioError> {
        let src = ReadAhead::open(path).map_err(|e| AudioError::Open(format!("{}: {e}", path.display())))?;
        let size = src.byte_len();
        Self::open(Box::new(src), hint_for(path), size)
    }

    /// Ouvre un fichier local pour en lire les informations (titre, durée…) sans lecture d'avance.
    pub fn probe_file(path: &Path) -> Result<Decoder, AudioError> {
        let file = std::fs::File::open(path).map_err(|e| AudioError::Open(format!("{}: {e}", path.display())))?;
        let size = file.metadata().map(|m| m.len()).ok();
        Self::open(Box::new(file), hint_for(path), size)
    }

    /// Ouvre une source quelconque (fichier, flux réseau…).
    pub fn open(source: Box<dyn MediaSource>, hint: Hint, byte_len: Option<u64>) -> Result<Decoder, AudioError> {
        let mss = MediaSourceStream::new(source, Default::default());
        let fmt_opts = FormatOptions::default();
        let meta_opts = MetadataOptions::default();
        let mut format =
            symphonia::default::get_probe().probe(&hint, mss, fmt_opts, meta_opts).map_err(|e| AudioError::Unsupported(e.to_string()))?;
        let track = format.default_track(TrackType::Audio).ok_or_else(|| AudioError::Unsupported("aucune piste audio".into()))?;
        let track_id = track.id;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .ok_or_else(|| AudioError::Unsupported("paramètres du codec inconnus".into()))?
            .clone();
        let dec_opts = AudioDecoderOptions::default().gapless(true);
        let decoder =
            symphonia::default::get_codecs().make_audio_decoder(&params, &dec_opts).map_err(|e| AudioError::Unsupported(e.to_string()))?;

        let duration_ms = match (track.time_base, track.duration) {
            (Some(tb), Some(d)) => tb.calc_duration(d).map(|t| t.as_millis().max(0) as u64),
            _ => None,
        }
        .or_else(|| match (track.num_frames, params.sample_rate) {
            (Some(n), Some(r)) if r > 0 => Some(n * 1000 / r as u64),
            _ => None,
        });
        let sample_rate = params.sample_rate.unwrap_or(44_100);
        let channels = params.channels.as_ref().map(|c| c.count() as u16).unwrap_or(2);
        let bitrate_kbps = match (byte_len, duration_ms) {
            (Some(b), Some(d)) if d > 0 => Some(((b * 8) / d) as u32),
            _ => None,
        };
        let codec = symphonia::default::get_codecs()
            .get_audio_decoder(params.codec)
            .map(|c| c.codec.info.short_name.to_ascii_uppercase())
            .unwrap_or_default();

        let mut info = TrackInfo {
            duration_ms,
            sample_rate,
            channels,
            bitrate_kbps,
            seekable: byte_len.is_some(),
            codec,
            ..Default::default()
        };
        // Métadonnées : la révision la plus récente.
        {
            let mut meta = format.metadata();
            if let Some(rev) = meta.skip_to_latest() {
                apply_tags(&mut info, rev);
            }
        }
        Ok(Decoder { format, decoder, track_id, info, scratch: Vec::new(), eof: false })
    }

    pub fn is_eof(&self) -> bool {
        self.eof
    }

    /// Décode le paquet suivant et ajoute les échantillons **stéréo entrelacés** à `out`.
    /// Renvoie le nombre de trames ajoutées (0 en fin de fichier).
    pub fn decode_next(&mut self, out: &mut Vec<f32>) -> Result<usize, AudioError> {
        let mut errors = 0;
        loop {
            if self.eof {
                return Ok(0);
            }
            let packet = match self.format.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) => {
                    self.eof = true;
                    return Ok(0);
                }
                Err(SymError::ResetRequired) => {
                    self.eof = true;
                    return Ok(0);
                }
                Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    self.eof = true;
                    return Ok(0);
                }
                Err(e) => return Err(AudioError::Decode(e.to_string())),
            };
            // Nouvelles métadonnées en cours de route (flux, OGG chaînés).
            {
                let mut meta = self.format.metadata();
                if !meta.is_latest() {
                    if let Some(rev) = meta.skip_to_latest() {
                        apply_tags(&mut self.info, rev);
                    }
                }
            }
            if packet.track_id != self.track_id {
                continue;
            }
            match self.decoder.decode(&packet) {
                Ok(buf) => {
                    let frames = buf.frames();
                    if frames == 0 {
                        continue;
                    }
                    let ch = buf.spec().channels().count().max(1);
                    if buf.spec().rate() > 0 {
                        self.info.sample_rate = buf.spec().rate();
                    }
                    self.scratch.resize(buf.samples_interleaved(), 0.0);
                    buf.copy_to_slice_interleaved(&mut self.scratch[..]);
                    to_stereo(&self.scratch, ch, out);
                    return Ok(frames);
                }
                Err(SymError::DecodeError(e)) => {
                    errors += 1;
                    log::debug!("paquet illisible ignoré : {e}");
                    if errors > 50 {
                        return Err(AudioError::Decode(e.to_string()));
                    }
                }
                Err(SymError::IoError(_)) => {
                    errors += 1;
                    if errors > 50 {
                        self.eof = true;
                        return Ok(0);
                    }
                }
                Err(e) => return Err(AudioError::Decode(e.to_string())),
            }
        }
    }

    /// Se place à `ms` ; renvoie la position atteinte en millisecondes.
    pub fn seek(&mut self, ms: u64) -> Result<u64, AudioError> {
        let to = SeekTo::Time { time: Time::from_millis_u64(ms), track_id: Some(self.track_id) };
        let seeked = self.format.seek(SeekMode::Accurate, to).map_err(|e| AudioError::Seek(e.to_string()))?;
        self.decoder.reset();
        self.eof = false;
        let tb = self.format.tracks().iter().find(|t| t.id == self.track_id).and_then(|t| t.time_base);
        let actual = tb.and_then(|tb| tb.calc_time(seeked.actual_ts)).map(|t| t.as_millis().max(0) as u64).unwrap_or(ms);
        Ok(actual)
    }
}

fn hint_for(path: &Path) -> Hint {
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    hint
}

/// Convertit `ch` canaux entrelacés en stéréo entrelacée (mono dupliqué, multicanal réduit).
fn to_stereo(src: &[f32], ch: usize, out: &mut Vec<f32>) {
    match ch {
        1 => {
            out.reserve(src.len() * 2);
            for &s in src {
                out.push(s);
                out.push(s);
            }
        }
        2 => out.extend_from_slice(src),
        n => {
            // Ordre usuel : FL, FR, C, LFE, SL, SR… ; centre et arrières mélangés à −3 dB.
            out.reserve(src.len() / n * 2);
            for f in src.chunks_exact(n) {
                let (mut l, mut r) = (f[0], f[1]);
                if n >= 3 {
                    l += 0.707 * f[2];
                    r += 0.707 * f[2];
                }
                if n >= 6 {
                    l += 0.707 * f[4];
                    r += 0.707 * f[5];
                }
                out.push(l * 0.6);
                out.push(r * 0.6);
            }
        }
    }
}

fn apply_tags(info: &mut TrackInfo, rev: &MetadataRevision) {
    let tags = rev.media.tags.iter().chain(rev.per_track.iter().flat_map(|t| t.metadata.tags.iter()));
    for tag in tags {
        let Some(std) = &tag.std else { continue };
        let s = |v: &std::sync::Arc<String>| Some(v.trim().to_string()).filter(|s| !s.is_empty());
        match std {
            StandardTag::TrackTitle(v) => info.title = s(v).or(info.title.take()),
            StandardTag::Artist(v) => info.artist = s(v).or(info.artist.take()),
            StandardTag::Album(v) => info.album = s(v).or(info.album.take()),
            StandardTag::Genre(v) => info.genre = s(v).or(info.genre.take()),
            StandardTag::Comment(v) => info.comment = s(v).or(info.comment.take()),
            StandardTag::ReplayGainTrackGain(v) => info.rg_track_gain = parse_db(v),
            StandardTag::ReplayGainAlbumGain(v) => info.rg_album_gain = parse_db(v),
            StandardTag::ReplayGainTrackPeak(v) => info.rg_track_peak = v.trim().parse().ok(),
            StandardTag::ReplayGainAlbumPeak(v) => info.rg_album_peak = v.trim().parse().ok(),
            _ => {}
        }
    }
    // Année et numéro de piste : clés brutes courantes (les variantes standard diffèrent selon les formats).
    for tag in rev.media.tags.iter() {
        let key = tag.raw.key.to_ascii_lowercase();
        let val = tag.raw.value.to_string();
        if info.year.is_none() && ["date", "year", "tyer", "tdrc", "©day"].contains(&key.as_str()) {
            info.year = Some(val.chars().take(4).collect());
        } else if info.track_number.is_none() && ["tracknumber", "trck", "trkn"].contains(&key.as_str()) {
            info.track_number = Some(val);
        }
    }
}

/// « -6.54 dB » → -6.54
fn parse_db(v: &str) -> Option<f32> {
    v.trim().trim_end_matches(|c: char| c.is_alphabetic() || c.is_whitespace()).trim().parse().ok()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Écrit un WAV PCM 16 bits stéréo (sinus) pour les tests.
    pub fn sine_wav(path: &Path, rate: u32, secs: f32, freq: f32) {
        let frames = (rate as f32 * secs) as u32;
        let mut data = Vec::new();
        for i in 0..frames {
            let s = ((i as f32 / rate as f32) * freq * std::f32::consts::TAU).sin() * 0.5;
            let v = (s * 32767.0) as i16;
            data.extend_from_slice(&v.to_le_bytes());
            data.extend_from_slice(&v.to_le_bytes());
        }
        let mut f = Vec::new();
        f.extend_from_slice(b"RIFF");
        f.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        f.extend_from_slice(b"WAVEfmt ");
        f.extend_from_slice(&16u32.to_le_bytes());
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&2u16.to_le_bytes());
        f.extend_from_slice(&rate.to_le_bytes());
        f.extend_from_slice(&(rate * 4).to_le_bytes());
        f.extend_from_slice(&4u16.to_le_bytes());
        f.extend_from_slice(&16u16.to_le_bytes());
        f.extend_from_slice(b"data");
        f.extend_from_slice(&(data.len() as u32).to_le_bytes());
        f.extend_from_slice(&data);
        std::fs::write(path, f).unwrap();
    }

    #[test]
    fn decode_wav_and_seek() {
        let dir = std::env::temp_dir().join(format!("alpagamp-dec-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("sine.wav");
        sine_wav(&p, 44_100, 2.0, 440.0);
        let mut d = Decoder::open_file(&p).unwrap();
        assert_eq!(d.info.sample_rate, 44_100);
        assert_eq!(d.info.channels, 2);
        assert_eq!(d.info.duration_ms, Some(2000));
        assert_eq!(d.info.bitrate_kbps, Some(1411));
        let mut out = Vec::new();
        let mut total = 0;
        loop {
            let n = d.decode_next(&mut out).unwrap();
            if n == 0 {
                break;
            }
            total += n;
        }
        assert_eq!(total, 88_200);
        assert_eq!(out.len(), 176_400);
        let peak = out.iter().fold(0f32, |m, s| m.max(s.abs()));
        assert!((peak - 0.5).abs() < 0.01, "crête {peak}");
        let pos = d.seek(1000).unwrap();
        assert!((pos as i64 - 1000).abs() < 30, "seek → {pos}");
        out.clear();
        let mut after = 0;
        while d.decode_next(&mut out).unwrap() > 0 {
            after = out.len() / 2;
        }
        assert!((after as i64 - 44_100).abs() < 2000, "après seek : {after}");
        std::fs::remove_dir_all(dir).ok();
    }

    /// Décoder depuis un disque qui bloque régulièrement 1,5 s : une fois le fichier lu d'avance, le décodage ne
    /// l'attend plus jamais (avant la 0.6.2, chaque bloc de 32 Ko pouvait bloquer la musique).
    #[test]
    fn decoding_from_a_slow_disk_never_waits() {
        use crate::readahead::tests::SlowDisk;
        use std::time::{Duration, Instant};
        let dir = std::env::temp_dir().join(format!("alpagamp-slow-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("sine.wav");
        sine_wav(&p, 24_000, 8.0, 440.0);
        let bytes = std::fs::read(&p).unwrap();
        let len = bytes.len() as u64;
        let disk = SlowDisk { inner: std::io::Cursor::new(bytes), delay: Duration::from_millis(5), stall: Duration::from_millis(1500), stall_every: 3, reads: 0 };
        let src = ReadAhead::new(disk, len).unwrap();
        let t = Instant::now();
        while src.buffered_ahead() < len {
            assert!(t.elapsed() < Duration::from_secs(20));
            std::thread::sleep(Duration::from_millis(20));
        }
        let mut d = Decoder::open(Box::new(src), Hint::new(), Some(len)).unwrap();
        let mut out = Vec::new();
        let mut worst = Duration::ZERO;
        loop {
            let t = Instant::now();
            let n = d.decode_next(&mut out).unwrap();
            worst = worst.max(t.elapsed());
            if n == 0 {
                break;
            }
        }
        assert_eq!(out.len(), 24_000 * 8 * 2);
        assert!(worst < Duration::from_millis(50), "un paquet a attendu {worst:?}");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn stereo_conversion() {
        let mut out = Vec::new();
        to_stereo(&[0.1, 0.2], 1, &mut out);
        assert_eq!(out, vec![0.1, 0.1, 0.2, 0.2]);
    }
}
