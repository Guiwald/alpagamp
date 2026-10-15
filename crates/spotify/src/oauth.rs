//! Connexion à Spotify par le navigateur (OAuth 2 « authorization code » avec PKCE), sans configuration :
//! on utilise l'identifiant client du lecteur de bureau de Spotify, comme librespot (ADR 0010).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;

use alpagamp_net::{Request, fetch, form_encode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Identifiant client public du lecteur de bureau de Spotify (celui qu'utilise librespot).
pub const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const AUTHORIZE: &str = "https://accounts.spotify.com/authorize";
const TOKEN: &str = "https://accounts.spotify.com/api/token";
/// Port de retour préféré (sinon un port libre).
const PORT: u16 = 8898;

pub const SCOPES: &[&str] = &[
    "streaming",
    "user-read-private",
    "user-read-email",
    "playlist-read-private",
    "playlist-read-collaborative",
    "user-library-read",
    "user-read-playback-state",
    "user-modify-playback-state",
    "user-read-currently-playing",
    "user-read-recently-played",
    "user-top-read",
];

/// Jeton d'accès et de renouvellement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: String,
    /// Expiration (secondes depuis 1970).
    pub expires_at: u64,
}

impl OAuthToken {
    pub fn is_expired(&self) -> bool {
        now() + 60 >= self.expires_at
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn random_b64(n: usize) -> Result<String, String> {
    let mut buf = vec![0u8; n];
    getrandom::fill(&mut buf).map_err(|e| e.to_string())?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
}

fn parse_token(body: &[u8], previous_refresh: Option<&str>) -> Result<OAuthToken, String> {
    let r: TokenResponse = serde_json::from_slice(body).map_err(|e| format!("réponse de Spotify illisible : {e}"))?;
    Ok(OAuthToken {
        access_token: r.access_token,
        refresh_token: r.refresh_token.or(previous_refresh.map(str::to_string)).unwrap_or_default(),
        expires_at: now() + r.expires_in.unwrap_or(3600),
    })
}

/// Page affichée dans le navigateur après l'autorisation.
const DONE_PAGE: &str = "<!doctype html><meta charset=utf-8><title>Alpagamp</title><body style=\"font:16px sans-serif;background:#222;color:#cfe;display:flex;align-items:center;justify-content:center;height:90vh\"><p>Alpagamp est connecté à Spotify. Vous pouvez fermer cet onglet.<br>Alpagamp is connected to Spotify. You can close this tab.</p>";

/// Attente maximale du retour du navigateur.
pub const AUTHORIZE_TIMEOUT: Duration = Duration::from_secs(300);

/// Une tentative de connexion en cours : la page à rouvrir si l'onglet a été fermé, et l'annulation.
#[derive(Default)]
pub struct Attempt {
    url: Mutex<Option<String>>,
    cancelled: AtomicBool,
}

impl Attempt {
    /// Adresse de la page d'autorisation (une fois ouverte).
    pub fn url(&self) -> Option<String> {
        self.url.lock().clone()
    }

    /// Annule l'attente : [`authorize`] rend la main en moins d'un dixième de seconde.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// Pourquoi la connexion n'a pas abouti.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// Annulée par l'utilisateur.
    Cancelled,
    /// Pas de retour du navigateur dans le délai : on revient simplement à « Se connecter ».
    TimedOut,
    Failed(String),
}

impl From<String> for AuthError {
    fn from(s: String) -> Self {
        AuthError::Failed(s)
    }
}

/// Ouvre la page d'autorisation (par `open_url`) et attend le retour du navigateur (`timeout` au plus).
/// Pendant l'attente, la page peut être rouverte (`attempt.url()`) ; `attempt.cancel()` l'interrompt.
pub fn authorize(open_url: &dyn Fn(&str), attempt: &Attempt, timeout: Duration) -> Result<OAuthToken, AuthError> {
    let listener = TcpListener::bind(("127.0.0.1", PORT)).or_else(|_| TcpListener::bind(("127.0.0.1", 0))).map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}/login");
    let verifier = random_b64(64)?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = random_b64(16)?;
    let url = format!(
        "{AUTHORIZE}?response_type=code&client_id={CLIENT_ID}&scope={}&redirect_uri={}&code_challenge_method=S256&code_challenge={challenge}&state={state}",
        form_encode(&SCOPES.join(" ")),
        form_encode(&redirect)
    );
    *attempt.url.lock() = Some(url.clone());
    open_url(&url);
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + timeout;
    let code = loop {
        if attempt.is_cancelled() {
            return Err(AuthError::Cancelled);
        }
        if Instant::now() > deadline {
            return Err(AuthError::TimedOut);
        }
        let (mut s, _) = match listener.accept() {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(e) => return Err(AuthError::Failed(e.to_string())),
        };
        let _ = s.set_nonblocking(false);
        let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
        let mut buf = [0u8; 4096];
        let n = s.read(&mut buf).unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]).to_string();
        let path = req.split_whitespace().nth(1).unwrap_or("").to_string();
        if !path.starts_with("/login") {
            let _ = s.write_all(b"HTTP/1.0 404 Not Found\r\nContent-Length: 0\r\n\r\n");
            continue;
        }
        let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
        let param = |name: &str| query.split('&').find_map(|kv| kv.strip_prefix(&format!("{name}="))).map(percent_decode);
        let _ = s.write_all(format!("HTTP/1.0 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{DONE_PAGE}", DONE_PAGE.len()).as_bytes());
        if let Some(err) = param("error") {
            return Err(AuthError::Failed(format!("Spotify a refusé la connexion ({err})")));
        }
        if param("state").as_deref() != Some(state.as_str()) {
            continue;
        }
        match param("code") {
            Some(c) => break c,
            None => continue,
        }
    };
    let body = format!(
        "grant_type=authorization_code&code={}&redirect_uri={}&client_id={CLIENT_ID}&code_verifier={verifier}",
        form_encode(&code),
        form_encode(&redirect)
    );
    let (resp, body) = fetch(TOKEN, &Request::post("application/x-www-form-urlencoded", body.into_bytes())).map_err(|e| e.0)?;
    if resp.status != 200 {
        return Err(AuthError::Failed(format!("Spotify refuse le jeton ({}) : {}", resp.status, String::from_utf8_lossy(&body))));
    }
    Ok(parse_token(&body, None)?)
}

/// Renouvelle un jeton expiré.
pub fn refresh(token: &OAuthToken) -> Result<OAuthToken, String> {
    if token.refresh_token.is_empty() {
        return Err("pas de jeton de renouvellement".into());
    }
    let body = format!("grant_type=refresh_token&refresh_token={}&client_id={CLIENT_ID}", form_encode(&token.refresh_token));
    let (resp, body) = fetch(TOKEN, &Request::post("application/x-www-form-urlencoded", body.into_bytes())).map_err(|e| e.0)?;
    if resp.status != 200 {
        return Err(format!("renouvellement refusé ({})", resp.status));
    }
    parse_token(&body, Some(&token.refresh_token))
}

pub(crate) fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                let h = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                match h {
                    Some(v) => {
                        out.push(v);
                        i += 3;
                    }
                    None => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_parsing_and_decoding() {
        let t = parse_token(br#"{"access_token":"A","token_type":"Bearer","expires_in":3600,"refresh_token":"R"}"#, None).unwrap();
        assert_eq!((t.access_token.as_str(), t.refresh_token.as_str()), ("A", "R"));
        assert!(!t.is_expired());
        let t2 = parse_token(br#"{"access_token":"B","expires_in":10}"#, Some("R")).unwrap();
        assert_eq!(t2.refresh_token, "R");
        assert!(t2.is_expired());
        assert_eq!(percent_decode("a%20b+c%3D"), "a b c=");
        assert_eq!(random_b64(64).unwrap().len(), 86);
    }

    /// Simule le retour du navigateur sur le port local (l'échange du code échoue ensuite : pas de réseau).
    #[test]
    fn local_redirect_is_received() {
        let opened = std::sync::Arc::new(parking_lot::Mutex::new(String::new()));
        let o2 = opened.clone();
        let h = std::thread::spawn(move || {
            authorize(
                &move |u: &str| {
                    *o2.lock() = u.to_string();
                },
                &Attempt::default(),
                AUTHORIZE_TIMEOUT,
            )
        });
        let url = loop {
            let u = opened.lock().clone();
            if !u.is_empty() {
                break u;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        assert!(url.starts_with(AUTHORIZE) && url.contains("code_challenge_method=S256"));
        let redirect = url.split("redirect_uri=").nth(1).unwrap().split('&').next().unwrap();
        let redirect = percent_decode(redirect);
        let state = url.split("state=").nth(1).unwrap().to_string();
        let port: u16 = redirect.trim_start_matches("http://127.0.0.1:").split('/').next().unwrap().parse().unwrap();
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.write_all(format!("GET /login?error=access_denied&state={state} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes()).unwrap();
        let mut resp = String::new();
        let _ = s.read_to_string(&mut resp);
        assert!(resp.contains("Alpagamp"));
        let r = h.join().unwrap();
        assert!(matches!(r, Err(AuthError::Failed(e)) if e.contains("access_denied")));
    }

    /// Onglet fermé : on peut rouvrir la même page, puis annuler ; l'attente s'arrête aussitôt.
    #[test]
    fn waiting_can_be_reopened_and_cancelled() {
        let attempt = std::sync::Arc::new(Attempt::default());
        let a2 = attempt.clone();
        let opened = std::sync::Arc::new(parking_lot::Mutex::new(0));
        let o2 = opened.clone();
        let h = std::thread::spawn(move || authorize(&move |_| *o2.lock() += 1, &a2, AUTHORIZE_TIMEOUT));
        let url = loop {
            if let Some(u) = attempt.url() {
                break u;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(url.starts_with(AUTHORIZE));
        let t = Instant::now();
        attempt.cancel();
        assert_eq!(h.join().unwrap(), Err(AuthError::Cancelled));
        assert!(t.elapsed() < Duration::from_secs(1));
        assert_eq!(*opened.lock(), 1);
        // Délai dépassé : pas d'erreur à afficher, juste « TimedOut ».
        let r = authorize(&|_| {}, &Attempt::default(), Duration::from_millis(100));
        assert_eq!(r, Err(AuthError::TimedOut));
    }
}
