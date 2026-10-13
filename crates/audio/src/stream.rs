//! Flux radio (HTTP, HTTPS, protocole ICY de SHOUTcast/Icecast).
//!
//! Un fil réseau lit le flux dans un tampon (en retirant les métadonnées ICY intercalées) ; le décodeur lit ce
//! tampon. Le fil de décodage ne bloque jamais sur le réseau : la source se déclare « pas prête » tant que le
//! tampon est trop maigre.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSource;

use crate::decode::{Decoder, TrackInfo};
use crate::{AudioError, PcmSource};

/// Taille du tampon réseau avant de commencer à décoder.
const PREBUFFER: usize = 48 * 1024;
/// En dessous, la source attend (sauf en fin de flux).
const LOW_WATER: usize = 12 * 1024;
const MAX_BUFFER: usize = 4 * 1024 * 1024;
const USER_AGENT: &str = concat!("Alpagamp/", env!("CARGO_PKG_VERSION"));

/// Adresse décomposée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    pub tls: bool,
    pub host: String,
    pub port: u16,
    /// Chemin et requête (« /stream?x=1 »).
    pub path: String,
}

impl Url {
    pub fn parse(s: &str) -> Result<Url, AudioError> {
        let s = s.trim();
        let (tls, rest) = if let Some(r) = s.strip_prefix("https://") {
            (true, r)
        } else if let Some(r) = s.strip_prefix("http://") {
            (false, r)
        } else if let Some(r) = s.strip_prefix("icy://") {
            (false, r)
        } else {
            return Err(AudioError::Open(format!("adresse non prise en charge : {s}")));
        };
        let (authority, path) = match rest.find(['/', '?']) {
            Some(i) => (&rest[..i], rest[i..].to_string()),
            None => (rest, "/".to_string()),
        };
        let path = if path.starts_with('?') { format!("/{path}") } else { path };
        let authority = authority.rsplit('@').next().unwrap_or(authority);
        let default_port = if tls { 443 } else { 80 };
        let (host, port) = if let Some(h) = authority.strip_prefix('[') {
            // IPv6 : [::1]:8000
            let end = h.find(']').ok_or_else(|| AudioError::Open(format!("adresse invalide : {s}")))?;
            let port = h[end + 1..].strip_prefix(':').and_then(|p| p.parse().ok()).unwrap_or(default_port);
            (h[..end].to_string(), port)
        } else {
            match authority.rsplit_once(':') {
                Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => {
                    (h.to_string(), p.parse().map_err(|_| AudioError::Open(format!("port invalide : {s}")))?)
                }
                _ => (authority.to_string(), default_port),
            }
        };
        if host.is_empty() {
            return Err(AudioError::Open(format!("adresse invalide : {s}")));
        }
        Ok(Url { tls, host, port, path })
    }

    /// Résout une adresse relative (redirection).
    fn join(&self, location: &str) -> Result<Url, AudioError> {
        if location.contains("://") {
            return Url::parse(location);
        }
        let scheme = if self.tls { "https" } else { "http" };
        if location.starts_with('/') {
            return Url::parse(&format!("{scheme}://{}:{}{location}", self.host, self.port));
        }
        let base = self.path.rsplit_once('/').map(|(b, _)| b).unwrap_or("");
        Url::parse(&format!("{scheme}://{}:{}{base}/{location}", self.host, self.port))
    }
}

/// En-têtes d'une réponse (noms en minuscules).
#[derive(Debug, Clone, Default)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
}

impl Response {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
}

type Conn = Box<dyn Read + Send + Sync>;

/// Racines de confiance : celles de Mozilla, plus celles du système (dont les autorités ajoutées par une
/// entreprise ou par l'utilisateur, et `SSL_CERT_FILE`).
fn root_store() -> rustls::RootCertStore {
    let mut roots = rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
    #[cfg(not(target_os = "android"))]
    {
        let native = rustls_native_certs::load_native_certs();
        let (added, _) = roots.add_parsable_certificates(native.certs);
        log::debug!("{added} certificats racines du système");
    }
    #[cfg(target_os = "android")]
    {
        use rustls::pki_types::CertificateDer;
        use rustls::pki_types::pem::PemObject;
        for dir in ["/system/etc/security/cacerts", "/apex/com.android.conscrypt/cacerts", "/data/misc/user/0/cacerts-added"] {
            let Ok(rd) = std::fs::read_dir(dir) else { continue };
            for e in rd.flatten() {
                if let Ok(bytes) = std::fs::read(e.path()) {
                    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(&bytes).flatten().collect();
                    roots.add_parsable_certificates(certs);
                }
            }
        }
    }
    roots
}

fn tls_config() -> Arc<rustls::ClientConfig> {
    static CFG: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    CFG.get_or_init(|| {
        let roots = root_store();
        let cfg = rustls::ClientConfig::builder_with_provider(Arc::new(rustls_graviola::default_provider()))
            .with_safe_default_protocol_versions()
            .expect("versions TLS")
            .with_root_certificates(roots)
            .with_no_client_auth();
        Arc::new(cfg)
    })
    .clone()
}

/// Lit l'en-tête de réponse octet par octet (jusqu'à la ligne vide).
fn read_head(r: &mut dyn Read) -> Result<Response, AudioError> {
    let mut head = Vec::with_capacity(1024);
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") && !head.ends_with(b"\n\n") {
        if head.len() > 32 * 1024 {
            return Err(AudioError::Open("en-tête de réponse trop long".into()));
        }
        match r.read(&mut byte) {
            Ok(0) => return Err(AudioError::Open("connexion fermée par le serveur".into())),
            Ok(_) => head.push(byte[0]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(AudioError::Open(format!("lecture : {e}"))),
        }
    }
    let text = String::from_utf8_lossy(&head);
    let mut lines = text.lines();
    let status_line = lines.next().unwrap_or_default();
    let mut parts = status_line.split_whitespace();
    let proto = parts.next().unwrap_or_default();
    if !(proto.starts_with("HTTP/") || proto == "ICY") {
        return Err(AudioError::Open(format!("réponse inattendue : {status_line}")));
    }
    let status = parts.next().and_then(|c| c.parse().ok()).unwrap_or(0);
    let headers = lines
        .filter_map(|l| l.split_once(':').map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())))
        .collect();
    Ok(Response { status, headers })
}

/// Mandataire HTTP pris dans l'environnement (`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY`), s'il y en a un.
fn proxy_for(url: &Url) -> Option<Url> {
    let names: &[&str] = if url.tls { &["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"] } else { &["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] };
    let p = names.iter().find_map(|n| std::env::var(n).ok().filter(|v| !v.trim().is_empty()))?;
    let no_proxy = std::env::var("NO_PROXY").or_else(|_| std::env::var("no_proxy")).unwrap_or_default();
    if no_proxy.split(',').map(str::trim).any(|d| !d.is_empty() && (d == "*" || url.host == d || url.host.ends_with(&format!(".{}", d.trim_start_matches('.'))))) {
        return None;
    }
    let p = if p.contains("://") { p } else { format!("http://{p}") };
    Url::parse(&p).ok().filter(|u| !u.tls)
}

/// Connexion TCP vers `url`, directe ou par un tunnel CONNECT du mandataire.
fn connect(url: &Url) -> Result<TcpStream, AudioError> {
    let via = proxy_for(url);
    let (host, port) = match &via {
        Some(p) => (p.host.as_str(), p.port),
        None => (url.host.as_str(), url.port),
    };
    let addr = (host, port)
        .to_socket_addrs()
        .map_err(|e| AudioError::Open(format!("{host} : {e}")))?
        .next()
        .ok_or_else(|| AudioError::Open(format!("{host} : adresse introuvable")))?;
    let mut tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(10)).map_err(|e| AudioError::Open(format!("{host} : {e}")))?;
    let _ = tcp.set_read_timeout(Some(Duration::from_secs(20)));
    let _ = tcp.set_nodelay(true);
    if via.is_some() {
        let target = format!("{}:{}", url.host, url.port);
        tcp.write_all(format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\nUser-Agent: {USER_AGENT}\r\n\r\n").as_bytes())
            .map_err(|e| AudioError::Open(format!("mandataire : {e}")))?;
        let resp = read_head(&mut tcp)?;
        if resp.status != 200 {
            return Err(AudioError::Open(format!("le mandataire répond {}", resp.status)));
        }
    }
    Ok(tcp)
}

/// Ouvre une connexion et envoie la requête ; suit les redirections.
pub fn request(url: &Url) -> Result<(Url, Response, Conn), AudioError> {
    let mut url = url.clone();
    for _ in 0..6 {
        let tcp = connect(&url)?;
        let req = format!(
            "GET {} HTTP/1.0\r\nHost: {}\r\nUser-Agent: {USER_AGENT}\r\nAccept: */*\r\nIcy-MetaData: 1\r\nConnection: close\r\n\r\n",
            url.path, url.host
        );
        let mut conn: Conn = if url.tls {
            let name = rustls::pki_types::ServerName::try_from(url.host.clone()).map_err(|e| AudioError::Open(format!("{} : {e}", url.host)))?;
            let client = rustls::ClientConnection::new(tls_config(), name).map_err(|e| AudioError::Open(format!("TLS : {e}")))?;
            let mut s = rustls::StreamOwned::new(client, tcp);
            s.write_all(req.as_bytes()).map_err(|e| AudioError::Open(format!("TLS : {e}")))?;
            Box::new(s)
        } else {
            let mut t = tcp;
            t.write_all(req.as_bytes()).map_err(|e| AudioError::Open(format!("envoi : {e}")))?;
            Box::new(t)
        };
        let resp = read_head(&mut *conn)?;
        match resp.status {
            200 => return Ok((url, resp, conn)),
            301 | 302 | 303 | 307 | 308 => {
                let loc = resp.header("location").ok_or_else(|| AudioError::Open("redirection sans adresse".into()))?;
                url = url.join(loc)?;
            }
            s => return Err(AudioError::Open(format!("le serveur répond {s}"))),
        }
    }
    Err(AudioError::Open("trop de redirections".into()))
}

/// Extrait `StreamTitle` d'un bloc de métadonnées ICY.
pub fn parse_stream_title(meta: &[u8]) -> Option<String> {
    let text = match std::str::from_utf8(meta) {
        Ok(t) => t.to_string(),
        Err(_) => meta.iter().map(|&b| b as char).collect(), // Latin-1
    };
    let start = text.find("StreamTitle='")? + "StreamTitle='".len();
    let rest = &text[start..];
    let end = rest.find("';").or_else(|| rest.rfind('\'')).unwrap_or(rest.len());
    let t = rest[..end].trim().to_string();
    (!t.is_empty()).then_some(t)
}

/// Retire les blocs de métadonnées ICY d'un flux (un bloc toutes les `metaint` octets audio).
struct IcyDemux {
    metaint: usize,
    until_meta: usize,
}

impl IcyDemux {
    /// Sépare `data` en audio (ajouté à `audio`) et métadonnées complètes (renvoyées) ; `pending` garde un bloc
    /// de métadonnées coupé entre deux lectures.
    fn feed(&mut self, mut data: &[u8], audio: &mut Vec<u8>, pending: &mut Option<(usize, Vec<u8>)>) -> Vec<Vec<u8>> {
        let mut metas = Vec::new();
        while !data.is_empty() {
            if let Some((len, buf)) = pending.as_mut() {
                let take = (*len - buf.len()).min(data.len());
                buf.extend_from_slice(&data[..take]);
                data = &data[take..];
                if buf.len() == *len {
                    metas.push(std::mem::take(buf));
                    *pending = None;
                    self.until_meta = self.metaint;
                }
                continue;
            }
            if self.metaint == 0 || self.until_meta > 0 {
                let take = if self.metaint == 0 { data.len() } else { self.until_meta.min(data.len()) };
                audio.extend_from_slice(&data[..take]);
                data = &data[take..];
                if self.metaint > 0 {
                    self.until_meta -= take;
                }
                continue;
            }
            // Octet de longueur.
            let len = data[0] as usize * 16;
            data = &data[1..];
            if len == 0 {
                self.until_meta = self.metaint;
            } else {
                *pending = Some((len, Vec::with_capacity(len)));
            }
        }
        metas
    }
}

/// État partagé entre le fil réseau et le décodeur.
struct Shared {
    buf: Mutex<VecDeque<u8>>,
    cond: Condvar,
    done: AtomicBool,
    stop: AtomicBool,
    title: Mutex<Option<String>>,
}

/// Lecteur du tampon pour Symphonia (non déplaçable).
struct BufferReader(Arc<Shared>);

impl Read for BufferReader {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut buf = self.0.buf.lock().unwrap();
        while buf.is_empty() {
            if self.0.done.load(Ordering::Acquire) {
                return Ok(0);
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "le flux ne répond plus"));
            }
            buf = self.0.cond.wait_timeout(buf, deadline - now).unwrap().0;
        }
        let n = out.len().min(buf.len());
        for (o, b) in out.iter_mut().zip(buf.drain(..n)) {
            *o = b;
        }
        self.0.cond.notify_all();
        Ok(n)
    }
}

impl std::io::Seek for BufferReader {
    fn seek(&mut self, _: std::io::SeekFrom) -> std::io::Result<u64> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "flux non déplaçable"))
    }
}

impl MediaSource for BufferReader {
    fn is_seekable(&self) -> bool {
        false
    }
    fn byte_len(&self) -> Option<u64> {
        None
    }
}

/// Une radio en cours de lecture.
pub struct StreamSource {
    decoder: Decoder,
    shared: Arc<Shared>,
    info: TrackInfo,
    title: Option<String>,
}

impl Drop for StreamSource {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        self.shared.cond.notify_all();
    }
}

fn hint_for(content_type: Option<&str>, path: &str) -> Hint {
    let mut h = Hint::new();
    let ct = content_type.unwrap_or_default().to_ascii_lowercase();
    let ext = if ct.contains("mpeg") || ct.contains("mp3") {
        Some("mp3")
    } else if ct.contains("aac") || ct.contains("mp4a") {
        Some("aac")
    } else if ct.contains("ogg") || ct.contains("opus") || ct.contains("vorbis") {
        Some("ogg")
    } else if ct.contains("flac") {
        Some("flac")
    } else {
        path.rsplit('.').next().filter(|e| e.len() <= 4)
    };
    if let Some(e) = ext {
        h.with_extension(e);
    }
    if !ct.is_empty() {
        h.mime_type(&ct);
    }
    h
}

fn is_playlist(content_type: Option<&str>, path: &str) -> bool {
    let ct = content_type.unwrap_or_default().to_ascii_lowercase();
    let p = path.split('?').next().unwrap_or_default().to_ascii_lowercase();
    ct.contains("scpls") || ct.contains("mpegurl") || ct.contains("pls+xml") || p.ends_with(".pls") || p.ends_with(".m3u") || p.ends_with(".xspf")
}

/// Ouvre une radio (ou une playlist de radios : on prend la première adresse qui répond).
pub fn open_stream(address: &str) -> Result<StreamSource, AudioError> {
    open_inner(address, 0)
}

fn open_inner(address: &str, depth: u32) -> Result<StreamSource, AudioError> {
    let url = Url::parse(address)?;
    let (url, resp, mut conn) = request(&url)?;
    let ct = resp.header("content-type").map(|s| s.to_string());
    if url.path.to_ascii_lowercase().contains(".m3u8") || ct.as_deref().is_some_and(|c| c.contains("vnd.apple.mpegurl")) {
        return Err(AudioError::Unsupported("flux HLS (.m3u8) pas encore pris en charge".into()));
    }
    if is_playlist(ct.as_deref(), &url.path) {
        if depth > 2 {
            return Err(AudioError::Open("playlists de radios imbriquées".into()));
        }
        let mut body = Vec::new();
        let _ = conn.by_ref().take(256 * 1024).read_to_end(&mut body);
        let entries = alpagamp_core::formats::read_playlist_bytes(&body, None);
        let mut last = AudioError::Open("playlist de radio vide".into());
        for e in entries {
            if let alpagamp_core::Source::Url(u) = &e.source {
                match open_inner(u, depth + 1) {
                    Ok(s) => return Ok(s),
                    Err(err) => last = err,
                }
            }
        }
        return Err(last);
    }
    let metaint: usize = resp.header("icy-metaint").and_then(|v| v.parse().ok()).unwrap_or(0);
    let station = resp.header("icy-name").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let bitrate = resp.header("icy-br").and_then(|v| v.split(',').next()?.trim().parse::<u32>().ok());
    let shared = Arc::new(Shared {
        buf: Mutex::new(VecDeque::with_capacity(PREBUFFER * 2)),
        cond: Condvar::new(),
        done: AtomicBool::new(false),
        stop: AtomicBool::new(false),
        title: Mutex::new(None),
    });
    // Fil réseau.
    {
        let shared = shared.clone();
        std::thread::Builder::new()
            .name("alpagamp-radio".into())
            .spawn(move || {
                let mut demux = IcyDemux { metaint, until_meta: metaint };
                let mut pending = None;
                let mut chunk = vec![0u8; 16 * 1024];
                let mut audio = Vec::with_capacity(16 * 1024);
                loop {
                    if shared.stop.load(Ordering::Acquire) {
                        break;
                    }
                    let n = match conn.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(n) => n,
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(e) => {
                            log::warn!("radio : {e}");
                            break;
                        }
                    };
                    audio.clear();
                    for m in demux.feed(&chunk[..n], &mut audio, &mut pending) {
                        if let Some(t) = parse_stream_title(&m) {
                            *shared.title.lock().unwrap() = Some(t);
                        }
                    }
                    let mut buf = shared.buf.lock().unwrap();
                    while buf.len() > MAX_BUFFER && !shared.stop.load(Ordering::Acquire) {
                        buf = shared.cond.wait_timeout(buf, Duration::from_millis(200)).unwrap().0;
                    }
                    buf.extend(audio.iter().copied());
                    shared.cond.notify_all();
                }
                shared.done.store(true, Ordering::Release);
                shared.cond.notify_all();
            })
            .map_err(|e| AudioError::Open(e.to_string()))?;
    }
    // Pré-remplissage.
    {
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut buf = shared.buf.lock().unwrap();
        while buf.len() < PREBUFFER && !shared.done.load(Ordering::Acquire) {
            let now = Instant::now();
            if now >= deadline {
                shared.stop.store(true, Ordering::Release);
                return Err(AudioError::Open("la radio ne répond pas".into()));
            }
            buf = shared.cond.wait_timeout(buf, deadline - now).unwrap().0;
        }
    }
    let reader = BufferReader(shared.clone());
    let decoder = match Decoder::open(Box::new(reader), hint_for(ct.as_deref(), &url.path), None) {
        Ok(d) => d,
        Err(e) => {
            shared.stop.store(true, Ordering::Release);
            shared.cond.notify_all();
            return Err(e);
        }
    };
    let mut info = decoder.info.clone();
    info.seekable = false;
    info.duration_ms = None;
    info.bitrate_kbps = bitrate.or(info.bitrate_kbps);
    info.stream_name = station.clone();
    if info.title.is_none() {
        info.title = station;
    }
    Ok(StreamSource { decoder, shared, info, title: None })
}

impl PcmSource for StreamSource {
    fn info(&self) -> &TrackInfo {
        &self.info
    }

    fn read(&mut self, out: &mut Vec<f32>) -> Result<usize, AudioError> {
        self.decoder.decode_next(out)
    }

    fn seek(&mut self, _ms: u64) -> Result<u64, AudioError> {
        Err(AudioError::Seek("une radio ne se déplace pas".into()))
    }

    fn take_info_update(&mut self) -> Option<TrackInfo> {
        let t = self.shared.title.lock().unwrap().clone();
        if t.is_some() && t != self.title {
            self.title = t.clone();
            self.info.title = t;
            self.info.artist = None;
            return Some(self.info.clone());
        }
        None
    }

    fn ready(&self) -> bool {
        self.shared.done.load(Ordering::Acquire) || self.shared.buf.lock().unwrap().len() >= LOW_WATER
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        let u = Url::parse("http://radio.example.org:8000/stream?x=1").unwrap();
        assert_eq!((u.tls, u.host.as_str(), u.port, u.path.as_str()), (false, "radio.example.org", 8000, "/stream?x=1"));
        let u = Url::parse("https://example.org").unwrap();
        assert_eq!((u.tls, u.port, u.path.as_str()), (true, 443, "/"));
        let u = Url::parse("http://[::1]:9000/a").unwrap();
        assert_eq!((u.host.as_str(), u.port), ("::1", 9000));
        assert_eq!(u.join("/b").unwrap().path, "/b");
        assert_eq!(Url::parse("http://h/dir/x.pls").unwrap().join("y.mp3").unwrap().path, "/dir/y.mp3");
        assert!(Url::parse("ftp://x").is_err());
    }

    #[test]
    fn icy_demux_and_title() {
        // 4 octets audio, bloc de 16 octets, 4 octets audio, bloc vide, 2 octets.
        let mut meta = b"StreamTitle='A - B';".to_vec();
        meta.resize(32, 0);
        let mut data = vec![1, 2, 3, 4, 2];
        data.extend_from_slice(&meta);
        data.extend_from_slice(&[5, 6, 7, 8, 0, 9, 10]);
        let mut d = IcyDemux { metaint: 4, until_meta: 4 };
        let mut pending = None;
        let mut audio = Vec::new();
        // Données livrées en morceaux de 3 octets pour couper les blocs.
        let mut metas = Vec::new();
        for c in data.chunks(3) {
            metas.extend(d.feed(c, &mut audio, &mut pending));
        }
        assert_eq!(audio, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(metas.len(), 1);
        assert_eq!(parse_stream_title(&metas[0]).as_deref(), Some("A - B"));
        assert_eq!(parse_stream_title(b"StreamTitle='';"), None);
    }

    /// Serveur ICY local : « ICY 200 OK », métadonnées toutes les 8192 octets, MP3 (WAV ici) servi en boucle.
    #[test]
    fn plays_a_local_icy_server() {
        let dir = std::env::temp_dir().join(format!("alpagamp-icy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav = dir.join("s.wav");
        crate::decode::tests::sine_wav(&wav, 44_100, 2.0, 440.0);
        let body = std::fs::read(&wav).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut req = [0u8; 1024];
            let _ = s.read(&mut req);
            let metaint = 8192;
            let _ = s.write_all(format!("ICY 200 OK\r\nicy-name: Radio Test\r\nicy-br: 128\r\ncontent-type: audio/wav\r\nicy-metaint: {metaint}\r\n\r\n").as_bytes());
            let mut meta = b"StreamTitle='Artiste - Morceau';".to_vec();
            meta.resize(48, 0);
            for (i, chunk) in body.chunks(metaint).enumerate() {
                let _ = s.write_all(chunk);
                if chunk.len() == metaint {
                    if i == 0 {
                        let _ = s.write_all(&[3]);
                        let _ = s.write_all(&meta);
                    } else {
                        let _ = s.write_all(&[0]);
                    }
                }
            }
        });
        let mut src = open_stream(&format!("http://127.0.0.1:{port}/live")).expect("ouverture");
        assert_eq!(src.info().stream_name.as_deref(), Some("Radio Test"));
        assert_eq!(src.info().bitrate_kbps, Some(128));
        let mut out = Vec::new();
        let mut frames = 0;
        let mut title = None;
        loop {
            out.clear();
            let n = src.read(&mut out).unwrap();
            if n == 0 {
                break;
            }
            frames += n;
            if let Some(i) = src.take_info_update() {
                title = i.title;
            }
        }
        assert!(frames > 80_000, "{frames}");
        assert_eq!(title.as_deref(), Some("Artiste - Morceau"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Radio réelle : `ALPAGAMP_RADIO=https://… cargo test -p alpagamp-audio real_radio -- --ignored`.
    #[test]
    #[ignore]
    fn real_radio() {
        let url = std::env::var("ALPAGAMP_RADIO").expect("ALPAGAMP_RADIO");
        let mut src = open_stream(&url).expect("ouverture");
        let mut out = Vec::new();
        let mut frames = 0;
        let start = Instant::now();
        while frames < 44_100 * 5 && start.elapsed() < Duration::from_secs(30) {
            out.clear();
            if !src.ready() {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
            frames += src.read(&mut out).unwrap();
            if let Some(i) = src.take_info_update() {
                eprintln!("titre : {:?}", i.title);
            }
        }
        eprintln!("{:?} {} Hz {:?} kbit/s, {frames} trames", src.info().stream_name, src.info().sample_rate, src.info().bitrate_kbps);
        assert!(frames > 0);
    }
}
