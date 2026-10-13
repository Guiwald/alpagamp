//! Client HTTP/HTTPS minimal d'Alpagamp (radios, connexion Spotify, API web) : HTTP/1.0, réponses « ICY »,
//! redirections, mandataire `HTTPS_PROXY`, TLS par rustls et graviola (sans C, ADR 0004), racines du système.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

const USER_AGENT: &str = concat!("Alpagamp/", env!("CARGO_PKG_VERSION"));

/// Erreur réseau (message lisible).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetError(pub String);

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NetError {}


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
    pub fn parse(s: &str) -> Result<Url, NetError> {
        let s = s.trim();
        let (tls, rest) = if let Some(r) = s.strip_prefix("https://") {
            (true, r)
        } else if let Some(r) = s.strip_prefix("http://") {
            (false, r)
        } else if let Some(r) = s.strip_prefix("icy://") {
            (false, r)
        } else {
            return Err(NetError(format!("adresse non prise en charge : {s}")));
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
            let end = h.find(']').ok_or_else(|| NetError(format!("adresse invalide : {s}")))?;
            let port = h[end + 1..].strip_prefix(':').and_then(|p| p.parse().ok()).unwrap_or(default_port);
            (h[..end].to_string(), port)
        } else {
            match authority.rsplit_once(':') {
                Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => {
                    (h.to_string(), p.parse().map_err(|_| NetError(format!("port invalide : {s}")))?)
                }
                _ => (authority.to_string(), default_port),
            }
        };
        if host.is_empty() {
            return Err(NetError(format!("adresse invalide : {s}")));
        }
        Ok(Url { tls, host, port, path })
    }

    /// Résout une adresse relative (redirection).
    fn join(&self, location: &str) -> Result<Url, NetError> {
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

/// Corps d'une réponse, à lire.
pub type Conn = Box<dyn Read + Send + Sync>;

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
fn read_head(r: &mut dyn Read) -> Result<Response, NetError> {
    let mut head = Vec::with_capacity(1024);
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") && !head.ends_with(b"\n\n") {
        if head.len() > 32 * 1024 {
            return Err(NetError("en-tête de réponse trop long".into()));
        }
        match r.read(&mut byte) {
            Ok(0) => return Err(NetError("connexion fermée par le serveur".into())),
            Ok(_) => head.push(byte[0]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(NetError(format!("lecture : {e}"))),
        }
    }
    let text = String::from_utf8_lossy(&head);
    let mut lines = text.lines();
    let status_line = lines.next().unwrap_or_default();
    let mut parts = status_line.split_whitespace();
    let proto = parts.next().unwrap_or_default();
    if !(proto.starts_with("HTTP/") || proto == "ICY") {
        return Err(NetError(format!("réponse inattendue : {status_line}")));
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
fn connect(url: &Url) -> Result<TcpStream, NetError> {
    let via = proxy_for(url);
    let (host, port) = match &via {
        Some(p) => (p.host.as_str(), p.port),
        None => (url.host.as_str(), url.port),
    };
    let addr = (host, port)
        .to_socket_addrs()
        .map_err(|e| NetError(format!("{host} : {e}")))?
        .next()
        .ok_or_else(|| NetError(format!("{host} : adresse introuvable")))?;
    let mut tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(10)).map_err(|e| NetError(format!("{host} : {e}")))?;
    let _ = tcp.set_read_timeout(Some(Duration::from_secs(20)));
    let _ = tcp.set_nodelay(true);
    if via.is_some() {
        let target = format!("{}:{}", url.host, url.port);
        tcp.write_all(format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\nUser-Agent: {USER_AGENT}\r\n\r\n").as_bytes())
            .map_err(|e| NetError(format!("mandataire : {e}")))?;
        let resp = read_head(&mut tcp)?;
        if resp.status != 200 {
            return Err(NetError(format!("le mandataire répond {}", resp.status)));
        }
    }
    Ok(tcp)
}

/// Une requête.
#[derive(Debug, Clone, Default)]
pub struct Request<'a> {
    pub method: &'a str,
    pub headers: Vec<(&'a str, String)>,
    pub body: Option<Vec<u8>>,
}

impl<'a> Request<'a> {
    pub fn get() -> Self {
        Request { method: "GET", ..Default::default() }
    }
    pub fn post(content_type: &str, body: Vec<u8>) -> Self {
        Request { method: "POST", headers: vec![("Content-Type", content_type.to_string())], body: Some(body) }
    }
    pub fn header(mut self, name: &'a str, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
    }
}

/// Envoie une requête ; suit les redirections (GET). Renvoie l'adresse finale, l'en-tête et le corps à lire.
/// Tous les codes de réponse sont renvoyés (à l'appelant de vérifier `status`), sauf les redirections suivies.
pub fn send(url: &Url, req: &Request) -> Result<(Url, Response, Conn), NetError> {
    let mut url = url.clone();
    for _ in 0..6 {
        let tcp = connect(&url)?;
        let mut head = format!("{} {} HTTP/1.0\r\nHost: {}\r\nUser-Agent: {USER_AGENT}\r\nAccept: */*\r\nConnection: close\r\n", req.method, url.path, url.host);
        for (k, v) in &req.headers {
            head.push_str(&format!("{k}: {v}\r\n"));
        }
        if let Some(b) = &req.body {
            head.push_str(&format!("Content-Length: {}\r\n", b.len()));
        }
        head.push_str("\r\n");
        let mut bytes = head.into_bytes();
        if let Some(b) = &req.body {
            bytes.extend_from_slice(b);
        }
        let mut conn: Conn = if url.tls {
            let name = rustls::pki_types::ServerName::try_from(url.host.clone()).map_err(|e| NetError(format!("{} : {e}", url.host)))?;
            let client = rustls::ClientConnection::new(tls_config(), name).map_err(|e| NetError(format!("TLS : {e}")))?;
            let mut s = rustls::StreamOwned::new(client, tcp);
            s.write_all(&bytes).map_err(|e| NetError(format!("TLS : {e}")))?;
            Box::new(s)
        } else {
            let mut t = tcp;
            t.write_all(&bytes).map_err(|e| NetError(format!("envoi : {e}")))?;
            Box::new(t)
        };
        let resp = read_head(&mut *conn)?;
        match resp.status {
            301 | 302 | 303 | 307 | 308 if req.method == "GET" => {
                let loc = resp.header("location").ok_or_else(|| NetError("redirection sans adresse".into()))?;
                url = url.join(loc)?;
            }
            _ => return Ok((url, resp, conn)),
        }
    }
    Err(NetError("trop de redirections".into()))
}

/// Requête GET d'un flux (radio) : demande aussi les métadonnées ICY ; échoue si la réponse n'est pas 200.
pub fn open_stream(url: &Url) -> Result<(Url, Response, Conn), NetError> {
    let (u, resp, conn) = send(url, &Request::get().header("Icy-MetaData", "1"))?;
    if resp.status != 200 {
        return Err(NetError(format!("le serveur répond {}", resp.status)));
    }
    Ok((u, resp, conn))
}

/// Lit tout le corps (taille limitée), en gérant `Content-Length` et le découpage « chunked ».
pub fn read_body(resp: &Response, conn: &mut Conn, limit: usize) -> Result<Vec<u8>, NetError> {
    let mut raw = Vec::new();
    let max = resp.header("content-length").and_then(|v| v.parse::<usize>().ok()).unwrap_or(limit).min(limit);
    conn.take(max as u64).read_to_end(&mut raw).map_err(|e| NetError(format!("lecture : {e}")))?;
    if resp.header("transfer-encoding").is_some_and(|v| v.eq_ignore_ascii_case("chunked")) {
        return dechunk(&raw);
    }
    Ok(raw)
}

fn dechunk(mut data: &[u8]) -> Result<Vec<u8>, NetError> {
    let mut out = Vec::new();
    loop {
        let line_end = data.windows(2).position(|w| w == b"\r\n").ok_or_else(|| NetError("découpage invalide".into()))?;
        let size_txt = std::str::from_utf8(&data[..line_end]).unwrap_or("").split(';').next().unwrap_or("").trim().to_string();
        let size = usize::from_str_radix(&size_txt, 16).map_err(|_| NetError("taille de bloc invalide".into()))?;
        data = &data[line_end + 2..];
        if size == 0 {
            return Ok(out);
        }
        if data.len() < size {
            return Err(NetError("bloc tronqué".into()));
        }
        out.extend_from_slice(&data[..size]);
        data = &data[(size + 2).min(data.len())..];
    }
}

/// Requête complète : code, en-têtes et corps (≤ 16 Mo).
pub fn fetch(url: &str, req: &Request) -> Result<(Response, Vec<u8>), NetError> {
    let u = Url::parse(url)?;
    let (_, resp, mut conn) = send(&u, req)?;
    let body = read_body(&resp, &mut conn, 16 * 1024 * 1024)?;
    Ok((resp, body))
}

/// Encode une valeur pour une requête (`application/x-www-form-urlencoded`, paramètres d'URL).
pub fn form_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
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
    fn chunked_and_form() {
        assert_eq!(dechunk(b"4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n").unwrap(), b"Wikipedia");
        assert_eq!(form_encode("a b&c=é"), "a%20b%26c%3D%C3%A9");
    }

    #[test]
    fn local_post() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = vec![0u8; 4096];
            let n = s.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let ok = req.starts_with("POST /token HTTP/1.0") && req.contains("Content-Length: 7") && req.ends_with("a=1&b=2");
            let body = if ok { "{\"ok\":true}" } else { "{}" };
            let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes());
        });
        let (resp, body) = fetch(&format!("http://127.0.0.1:{port}/token"), &Request::post("application/x-www-form-urlencoded", b"a=1&b=2".to_vec())).unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(body, b"{\"ok\":true}");
    }
}
