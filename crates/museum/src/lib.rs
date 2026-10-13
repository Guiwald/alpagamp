//! Le Winamp Skin Museum (<https://skins.webamp.org>, API publique `api.webamp.org/graphql`) : parcourir et
//! chercher des skins classiques, en afficher un aperçu, les télécharger (ADR 0012). Rien n'est livré avec
//! Alpagamp : chaque skin vient du musée, à la demande de l'utilisateur.
//!
//! [`Museum`] fait le travail réseau sur des fils d'exécution et rend des [`Event`] ; [`Browser`] pilote la
//! fenêtre du navigateur (bureau et téléphone).

pub mod browser;

pub use browser::{Apply, Browser, list_skins, local_thumbnail};

use std::path::PathBuf;

use alpagamp_net::{Request as HttpRequest, fetch};
use alpagamp_skin::Image;
use crossbeam_channel::{Receiver, Sender, unbounded};
use serde_json::Value;

const API: &str = "https://api.webamp.org/graphql";
/// Skins par page.
pub const PAGE: usize = 30;

/// Un skin du musée.
#[derive(Clone, Debug, PartialEq)]
pub struct SkinInfo {
    pub md5: String,
    /// Nom du fichier sans extension.
    pub name: String,
    pub download_url: String,
    pub screenshot_url: String,
}

fn graphql(query: &str, variables: Value) -> Result<Value, String> {
    let body = serde_json::json!({ "query": query, "variables": variables }).to_string().into_bytes();
    let (resp, bytes) = fetch(API, &HttpRequest::post("application/json", body)).map_err(|e| e.0)?;
    if resp.status != 200 {
        return Err(format!("le musée répond {}", resp.status));
    }
    let v: Value = serde_json::from_slice(&bytes).map_err(|e| format!("réponse illisible : {e}"))?;
    if let Some(err) = v.pointer("/errors/0/message").and_then(Value::as_str) {
        return Err(err.to_string());
    }
    Ok(v)
}

/// Lit une liste de skins (sans ceux marqués « pour adultes »).
pub fn parse_list(list: &Value) -> Vec<SkinInfo> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter(|s| !s.get("nsfw").and_then(Value::as_bool).unwrap_or(false))
        .filter_map(|s| {
            let md5 = s.get("md5")?.as_str()?.to_string();
            let file = s.get("filename").and_then(Value::as_str).unwrap_or(&md5);
            let name = file.rsplit_once('.').map(|(n, _)| n).unwrap_or(file).replace('_', " ");
            Some(SkinInfo {
                name,
                download_url: s.get("download_url")?.as_str()?.to_string(),
                screenshot_url: s.get("screenshot_url")?.as_str()?.to_string(),
                md5,
            })
        })
        .collect()
}

const FIELDS: &str = "md5 filename download_url screenshot_url nsfw";

/// Les skins mis en avant par le musée, page par page.
pub fn browse(offset: usize) -> Result<Vec<SkinInfo>, String> {
    let q = format!("query($first: Int, $offset: Int) {{ skins(first: $first, offset: $offset, sort: MUSEUM) {{ nodes {{ {FIELDS} }} }} }}");
    let v = graphql(&q, serde_json::json!({ "first": PAGE, "offset": offset }))?;
    Ok(parse_list(v.pointer("/data/skins/nodes").unwrap_or(&Value::Null)))
}

/// Recherche par nom.
pub fn search(query: &str, offset: usize) -> Result<Vec<SkinInfo>, String> {
    let q = format!("query($q: String!, $first: Int, $offset: Int) {{ search_classic_skins(query: $q, first: $first, offset: $offset) {{ {FIELDS} }} }}");
    let v = graphql(&q, serde_json::json!({ "q": query, "first": PAGE, "offset": offset }))?;
    Ok(parse_list(v.pointer("/data/search_classic_skins").unwrap_or(&Value::Null)))
}

/// Décode une image PNG en pixels `0xFFRRGGBB`.
pub fn decode_png(bytes: &[u8]) -> Option<Image> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let ch = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return None,
    };
    let mut img = Image::new(w, h, 0xFF00_0000);
    for y in 0..h as usize {
        let row = &buf[y * info.line_size..];
        for x in 0..w as usize {
            let p = &row[x * ch..x * ch + ch];
            let (r, g, b) = if ch >= 3 { (p[0], p[1], p[2]) } else { (p[0], p[0], p[0]) };
            img.pixels[y * w as usize + x] = 0xFF00_0000 | (r as u32) << 16 | (g as u32) << 8 | b as u32;
        }
    }
    Some(img)
}

/// Aperçu : la fenêtre principale (275×116 en haut à gauche de la capture du musée) réduite de moitié.
pub fn thumbnail(shot: &Image) -> Image {
    let (sw, sh) = (shot.width.min(275), shot.height.min(116));
    let (w, h) = (sw / 2, sh / 2);
    let mut out = Image::new(w.max(1), h.max(1), 0xFF00_0000);
    for y in 0..h {
        for x in 0..w {
            // Moyenne de 2×2 pixels.
            let mut acc = [0u32; 3];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let p = shot.pixels[((y * 2 + dy) * shot.width + x * 2 + dx) as usize];
                acc[0] += (p >> 16) & 0xFF;
                acc[1] += (p >> 8) & 0xFF;
                acc[2] += p & 0xFF;
            }
            out.pixels[(y * w + x) as usize] = 0xFF00_0000 | (acc[0] / 4) << 16 | (acc[1] / 4) << 8 | acc[2] / 4;
        }
    }
    out
}

/// Ce que l'interface demande.
#[derive(Clone, Debug)]
pub enum Request {
    /// Page `offset` des skins mis en avant, ou d'une recherche si `query` n'est pas vide.
    List { query: String, offset: usize },
    Thumbnail { md5: String, url: String },
    /// Télécharge le `.wsz` dans `dest`.
    Download { info: SkinInfo, dest: PathBuf },
}

/// Ce qui revient.
#[derive(Debug)]
pub enum Event {
    List { id: u64, query: String, offset: usize, result: Result<Vec<SkinInfo>, String> },
    Thumbnail { md5: String, image: Option<Image> },
    Downloaded { id: u64, info: SkinInfo, result: Result<PathBuf, String> },
}

/// Fils d'exécution du musée.
pub struct Museum {
    tx: Sender<(u64, Request)>,
    rx: Receiver<Event>,
    next: u64,
}

impl Default for Museum {
    fn default() -> Self {
        Self::new()
    }
}

impl Museum {
    pub fn new() -> Museum {
        let (tx, jobs) = unbounded::<(u64, Request)>();
        let (out, rx) = unbounded();
        for i in 0..3 {
            let jobs = jobs.clone();
            let out = out.clone();
            std::thread::Builder::new()
                .name(format!("musée-{i}"))
                .spawn(move || {
                    for (id, req) in jobs.iter() {
                        let ev = work(id, req);
                        if out.send(ev).is_err() {
                            break;
                        }
                    }
                })
                .ok();
        }
        Museum { tx, rx, next: 1 }
    }

    /// Envoie une demande ; renvoie son numéro.
    pub fn request(&mut self, r: Request) -> u64 {
        let id = self.next;
        self.next += 1;
        let _ = self.tx.send((id, r));
        id
    }

    pub fn poll(&self) -> Vec<Event> {
        self.rx.try_iter().collect()
    }
}

fn work(id: u64, req: Request) -> Event {
    match req {
        Request::List { query, offset } => {
            let result = if query.trim().is_empty() { browse(offset) } else { search(query.trim(), offset) };
            Event::List { id, query, offset, result }
        }
        Request::Thumbnail { md5, url } => {
            let image = fetch(&url, &HttpRequest::get()).ok().filter(|(r, _)| r.status == 200).and_then(|(_, b)| decode_png(&b)).map(|i| thumbnail(&i));
            Event::Thumbnail { md5, image }
        }
        Request::Download { info, dest } => {
            let result = (|| {
                let (r, bytes) = fetch(&info.download_url, &HttpRequest::get()).map_err(|e| e.0)?;
                if r.status != 200 {
                    return Err(format!("le musée répond {}", r.status));
                }
                if let Some(dir) = dest.parent() {
                    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                }
                std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
                Ok(dest)
            })();
            Event::Downloaded { id, info, result }
        }
    }
}

/// Nom de fichier sûr pour ranger un skin téléchargé.
pub fn file_name(info: &SkinInfo) -> String {
    let clean: String = info.name.chars().map(|c| if c.is_alphanumeric() || " -_.()".contains(c) { c } else { '_' }).collect();
    let clean = clean.trim().trim_matches('.');
    let base = if clean.is_empty() { &info.md5[..8.min(info.md5.len())] } else { clean };
    format!("{base}.wsz")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_filters() {
        let v: Value = serde_json::from_str(
            r#"[{"md5":"aa","filename":"Cool_Skin.wsz","download_url":"https://x/aa.wsz","screenshot_url":"https://x/aa.png","nsfw":false},
                {"md5":"bb","filename":"bad.wsz","download_url":"u","screenshot_url":"s","nsfw":true},
                {"md5":"cc","filename":"noext","download_url":"u","screenshot_url":"s"}]"#,
        )
        .unwrap();
        let l = parse_list(&v);
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].name, "Cool Skin");
        assert_eq!(l[1].name, "noext");
        assert_eq!(file_name(&l[0]), "Cool Skin.wsz");
        let weird = SkinInfo { md5: "0123456789".into(), name: "a/b:c".into(), download_url: String::new(), screenshot_url: String::new() };
        assert_eq!(file_name(&weird), "a_b_c.wsz");
    }

    #[test]
    fn decodes_and_thumbnails() {
        let mut bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut bytes, 4, 2);
            enc.set_color(png::ColorType::Rgb);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            let data: Vec<u8> = (0..8).flat_map(|i| if i % 2 == 0 { [255, 0, 0] } else { [0, 0, 255] }).collect();
            w.write_image_data(&data).unwrap();
        }
        let img = decode_png(&bytes).unwrap();
        assert_eq!((img.width, img.height), (4, 2));
        assert_eq!(img.pixels[0], 0xFFFF_0000);
        let t = thumbnail(&img);
        assert_eq!((t.width, t.height), (2, 1));
        assert_eq!(t.pixels[0], 0xFF7F_007F);
    }

    /// Musée réel : `cargo test -p alpagamp-museum real_museum -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_museum() {
        let top = browse(0).unwrap();
        eprintln!("{} skins, ex. {:?}", top.len(), top.first());
        assert!(top.len() > 10);
        let s = search("zelda", 0).unwrap();
        assert!(!s.is_empty());
        let (_, b) = fetch(&s[0].screenshot_url, &HttpRequest::get()).unwrap();
        let t = thumbnail(&decode_png(&b).unwrap());
        assert_eq!((t.width, t.height), (137, 58));
    }
}
