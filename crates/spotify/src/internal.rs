//! Canaux internes de Spotify (« spclient »), ceux de ses propres applis, par la session librespot (ADR 0022) :
//! liste des playlists (rootlist), contenu d'une playlist, collection (titres likés, albums enregistrés),
//! métadonnées par paquets et recherche de titres.
//!
//! L'API web publique (`api.spotify.com`) bride fortement les jetons obtenus avec l'identifiant du lecteur de
//! bureau, que partagent tous les lecteurs basés sur librespot : tout ce qui peut passer par ici y passe.
//!
//! Les requêtes sont faites à la main (adresse, jeton et « client-token » fournis par librespot) pour lire
//! l'en-tête `Retry-After` d'une réponse 429 : librespot l'attend lui-même jusqu'à 10 s, puis le jette.

use std::collections::HashMap;
use std::time::Duration;

use alpagamp_core::library::{LibraryItem, LibraryKind};
use bytes::Bytes;
use http_body_util::BodyExt;
use librespot_core::{Session, SpotifyId};
use librespot_protocol::extended_metadata::{BatchedEntityRequest, BatchedExtensionResponse, EntityRequest, ExtensionQuery};
use librespot_protocol::extension_kind::ExtensionKind;
use librespot_protocol::metadata::{Album, Episode, Track};
use librespot_protocol::playlist4_external::SelectedListContent;
use protobuf::{EnumOrUnknown, Message};

use crate::Fault;

const PROTOBUF: &str = "application/x-protobuf";
/// Le service de la collection ne répond qu'à son propre type (sinon : 400).
const COLLECTION: &str = "application/vnd.collection-v2.spotify.proto";
/// Décorations demandées pour une liste (sans elles, la longueur annoncée manque).
const DECORATE: &str = "decorate=revision,attributes,length,owner,capabilities";
/// Plafond de lignes par liste (une playlist Spotify en compte au plus 10 000).
pub const MAX_ITEMS: usize = 10_000;
/// Taille d'un paquet de métadonnées.
pub const BATCH: usize = 100;

/// Une requête au spclient. `body` : `None` pour un GET.
pub async fn request(session: &Session, endpoint: &str, accept: &str, body: Option<(&str, Vec<u8>)>) -> Result<Bytes, Fault> {
    let base = session.spclient().base_url().await.map_err(|e| Fault::Network(e.to_string()))?;
    let token = session.login5().auth_token().await.map_err(|e| Fault::Network(format!("jeton : {e}")))?;
    let client_token = session.spclient().client_token().await.ok();
    let method = if body.is_some() { http::Method::POST } else { http::Method::GET };
    let mut b = http::Request::builder()
        .method(method)
        .uri(format!("{base}{endpoint}"))
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token.access_token))
        .header(http::header::ACCEPT, accept);
    if let Some(ct) = client_token {
        b = b.header("client-token", ct);
    }
    let payload = match body {
        Some((ct, bytes)) => {
            b = b.header(http::header::CONTENT_TYPE, ct);
            Bytes::from(bytes)
        }
        None => Bytes::new(),
    };
    let req = b.header(http::header::CONTENT_LENGTH, payload.len()).body(payload).map_err(|e| Fault::Network(e.to_string()))?;
    let fut = session.http_client().request_fut(req).map_err(|e| Fault::from_librespot(&e))?;
    let resp = tokio::time::timeout(Duration::from_secs(30), fut).await.map_err(|_| Fault::Network("délai dépassé".into()))?.map_err(|e| Fault::Network(e.to_string()))?;
    let status = resp.status().as_u16();
    let retry_after = resp.headers().get(http::header::RETRY_AFTER).and_then(|v| v.to_str().ok()).and_then(parse_retry_after);
    let body = resp.into_body().collect().await.map_err(|e| Fault::Network(e.to_string()))?.to_bytes();
    match status {
        200..=299 => Ok(body),
        429 => {
            log::warn!("Spotify (canal interne) : 429 sur {endpoint}, Retry-After {retry_after:?}");
            Err(Fault::RateLimited(retry_after))
        }
        s => {
            log::warn!("Spotify (canal interne) : {s} sur {endpoint}");
            Err(Fault::Http(s))
        }
    }
}

/// Décode un message sans exiger ses champs « required » (proto2) : Spotify fait évoluer ses messages, et un
/// champ absent ne doit pas faire échouer toute la liste.
pub fn decode<M: Message>(body: &[u8]) -> Result<M, Fault> {
    let mut m = M::new();
    m.merge_from_bytes(body).map_err(|e| Fault::Decode(e.to_string()))?;
    Ok(m)
}

/// `Retry-After` en secondes (la forme « date HTTP » n'est pas utilisée par Spotify).
pub fn parse_retry_after(v: &str) -> Option<Duration> {
    v.trim().parse::<u64>().ok().map(Duration::from_secs)
}

/// Décode le nom d'un dossier de playlists (« Mes+favoris%21 »).
fn folder_name(s: &str) -> String {
    crate::oauth::percent_decode(s)
}

/// Lit une page de la liste des playlists. `folders` : dossiers ouverts (identifiant, nom), d'une page à
/// l'autre. Renvoie le nombre d'éléments de la page.
pub fn rootlist_page(list: &SelectedListContent, me: &str, folders: &mut Vec<(String, String)>, out: &mut Vec<LibraryItem>) -> usize {
    let contents = list.contents.get_or_default();
    let n = contents.items.len();
    let aligned = contents.meta_items.len() == n;
    for (i, item) in contents.items.iter().enumerate() {
        let uri = item.uri();
        if let Some(rest) = uri.strip_prefix("spotify:start-group:") {
            let (id, name) = rest.split_once(':').unwrap_or((rest, ""));
            folders.push((id.to_string(), folder_name(name)));
            continue;
        }
        if let Some(id) = uri.strip_prefix("spotify:end-group:") {
            if let Some(pos) = folders.iter().rposition(|(f, _)| f == id) {
                folders.truncate(pos);
            }
            continue;
        }
        if !uri.starts_with("spotify:playlist:") {
            continue;
        }
        let meta = if aligned { Some(&contents.meta_items[i]) } else { None };
        let name = meta.map(|m| m.attributes.get_or_default().name().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| uri.to_string());
        let owner = meta.map(|m| m.owner_username().to_string()).unwrap_or_default();
        let mut subtitle = folders.iter().map(|(_, n)| n.as_str()).filter(|n| !n.is_empty()).collect::<Vec<_>>().join(" › ");
        if !owner.is_empty() && owner != me {
            if !subtitle.is_empty() {
                subtitle.push_str(" · ");
            }
            subtitle.push_str(&owner);
        }
        out.push(LibraryItem::new(LibraryKind::Playlist, name, subtitle, uri));
    }
    n
}

/// Mes playlists, dans l'ordre de ma bibliothèque. Les dossiers sont aplatis : leur nom va en sous-titre.
pub async fn rootlist(session: &Session) -> Result<Vec<LibraryItem>, Fault> {
    let me = session.username();
    let mut out = Vec::new();
    let mut folders: Vec<(String, String)> = Vec::new();
    let mut from = 0usize;
    loop {
        let endpoint = format!("/playlist/v2/user/{me}/rootlist?{DECORATE},status_code&from={from}&length=500");
        let body = request(session, &endpoint, PROTOBUF, None).await?;
        let list = decode::<SelectedListContent>(&body)?;
        let n = rootlist_page(&list, &me, &mut folders, &mut out);
        from += n;
        let total = list.length().max(0) as usize;
        if n == 0 || from >= total || from >= MAX_ITEMS {
            break;
        }
    }
    Ok(out)
}

/// Adresses des éléments d'une playlist (morceaux, épisodes ; les fichiers locaux sont écartés), et le nombre
/// d'éléments annoncé par Spotify.
pub async fn playlist_uris(session: &Session, id: &str) -> Result<(Vec<String>, usize), Fault> {
    let mut uris = Vec::new();
    let mut from = 0usize;
    let mut announced = 0usize;
    loop {
        let endpoint = if from == 0 { format!("/playlist/v2/playlist/{id}?{DECORATE}") } else { format!("/playlist/v2/playlist/{id}?{DECORATE}&from={from}&length=1000") };
        let body = request(session, &endpoint, PROTOBUF, None).await?;
        let list = decode::<SelectedListContent>(&body)?;
        let contents = list.contents.get_or_default();
        let n = contents.items.len();
        uris.extend(contents.items.iter().map(|i| i.uri().to_string()).filter(|u| is_playable_uri(u)));
        from += n;
        announced = announced.max(list.length().max(0) as usize);
        let more = contents.truncated() || from < announced;
        if n == 0 || !more || from >= MAX_ITEMS {
            break;
        }
    }
    Ok((uris, announced))
}

pub fn is_playable_uri(u: &str) -> bool {
    u.starts_with("spotify:track:") || u.starts_with("spotify:episode:")
}

/// Un élément de la collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionItem {
    pub uri: String,
    pub added_at: i64,
}

/// Encode un champ « chaîne » (type 2) d'un message protobuf.
fn pb_string(out: &mut Vec<u8>, field: u32, s: &str) {
    pb_varint(out, ((field << 3) | 2) as u64);
    pb_varint(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

fn pb_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

/// `PageRequest { username = 1, set = 2, pagination_token = 3, limit = 4 }` (collection2v2.proto, que
/// librespot ne compile pas).
pub fn page_request(user: &str, set: &str, token: &str, limit: u32) -> Vec<u8> {
    let mut b = Vec::new();
    pb_string(&mut b, 1, user);
    pb_string(&mut b, 2, set);
    if !token.is_empty() {
        pb_string(&mut b, 3, token);
    }
    pb_varint(&mut b, 4 << 3);
    pb_varint(&mut b, limit as u64);
    b
}

/// Lit `PageResponse { repeated CollectionItem items = 1; string next_page_token = 2 }`, avec
/// `CollectionItem { uri = 1, added_at = 2, is_removed = 3 }`.
pub fn parse_page(body: &[u8]) -> Result<(Vec<CollectionItem>, String), Fault> {
    let mut items = Vec::new();
    let mut next = String::new();
    let mut r = protobuf::CodedInputStream::from_bytes(body);
    let bad = |e: protobuf::Error| Fault::Decode(e.to_string());
    while !r.eof().map_err(bad)? {
        let tag = r.read_raw_varint32().map_err(bad)?;
        match (tag >> 3, tag & 7) {
            (1, 2) => {
                let bytes = r.read_bytes().map_err(bad)?;
                let mut ir = protobuf::CodedInputStream::from_bytes(&bytes);
                let (mut uri, mut added_at, mut removed) = (String::new(), 0i64, false);
                while !ir.eof().map_err(bad)? {
                    let t = ir.read_raw_varint32().map_err(bad)?;
                    match (t >> 3, t & 7) {
                        (1, 2) => uri = ir.read_string().map_err(bad)?,
                        (2, 0) => added_at = ir.read_int32().map_err(bad)? as i64,
                        (3, 0) => removed = ir.read_bool().map_err(bad)?,
                        (_, w) => skip(&mut ir, w).map_err(bad)?,
                    }
                }
                if !removed && !uri.is_empty() {
                    items.push(CollectionItem { uri, added_at });
                }
            }
            (2, 2) => next = r.read_string().map_err(bad)?,
            (_, w) => skip(&mut r, w).map_err(bad)?,
        }
    }
    Ok((items, next))
}

fn skip(r: &mut protobuf::CodedInputStream, wire: u32) -> protobuf::Result<()> {
    match wire {
        0 => r.read_raw_varint64().map(|_| ()),
        1 => r.read_raw_little_endian64().map(|_| ()),
        2 => r.read_bytes().map(|_| ()),
        5 => r.read_raw_little_endian32().map(|_| ()),
        _ => Err(protobuf::Error::from(std::io::Error::new(std::io::ErrorKind::InvalidData, "type de champ inconnu"))),
    }
}

/// La collection : titres likés (`spotify:track:`) et albums enregistrés (`spotify:album:`) partagent
/// l'ensemble « collection » ; on les distingue par l'adresse. Du plus récent au plus ancien.
pub async fn collection(session: &Session) -> Result<Vec<CollectionItem>, Fault> {
    let me = session.username();
    let mut all = Vec::new();
    let mut token = String::new();
    for _ in 0..200 {
        let body = request(session, "/collection/v2/paging", COLLECTION, Some((COLLECTION, page_request(&me, "collection", &token, 300)))).await?;
        let (items, next) = parse_page(&body)?;
        let n = items.len();
        all.extend(items);
        if next.is_empty() || n == 0 || next == token {
            break;
        }
        token = next;
    }
    all.sort_by(|a, b| b.added_at.cmp(&a.added_at));
    Ok(all)
}

/// Métadonnées d'un élément.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Meta {
    pub name: String,
    pub artists: String,
    pub duration_ms: Option<u64>,
    /// Morceaux d'un album (adresses).
    pub tracks: Vec<String>,
}

fn kind_of(uri: &str) -> Option<ExtensionKind> {
    if uri.starts_with("spotify:track:") {
        Some(ExtensionKind::TRACK_V4)
    } else if uri.starts_with("spotify:album:") {
        Some(ExtensionKind::ALBUM_V4)
    } else if uri.starts_with("spotify:episode:") {
        Some(ExtensionKind::EPISODE_V4)
    } else {
        None
    }
}

fn gid_uri(kind: &str, gid: &[u8]) -> Option<String> {
    SpotifyId::from_raw(gid).ok()?.to_base62().ok().map(|b| format!("spotify:{kind}:{b}"))
}

fn join_artists<'a>(names: impl Iterator<Item = &'a str>) -> String {
    names.filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", ")
}

/// Décode une réponse de métadonnées étendues.
pub fn parse_metadata(body: &[u8]) -> Result<HashMap<String, Meta>, Fault> {
    let resp = decode::<BatchedExtensionResponse>(body)?;
    let mut out = HashMap::new();
    for array in &resp.extended_metadata {
        for d in &array.extension_data {
            let Some(any) = d.extension_data.as_ref() else { continue };
            let uri = d.entity_uri.clone();
            let meta = match array.extension_kind.enum_value() {
                Ok(ExtensionKind::TRACK_V4) => decode::<Track>(&any.value).ok().map(|t| Meta {
                    name: t.name().to_string(),
                    artists: join_artists(t.artist.iter().map(|a| a.name())),
                    duration_ms: t.duration.map(|d| d.max(0) as u64),
                    tracks: Vec::new(),
                }),
                Ok(ExtensionKind::ALBUM_V4) => decode::<Album>(&any.value).ok().map(|a| Meta {
                    name: a.name().to_string(),
                    artists: join_artists(a.artist.iter().map(|x| x.name())),
                    duration_ms: None,
                    tracks: a.disc.iter().flat_map(|d| d.track.iter()).filter_map(|t| gid_uri("track", t.gid())).collect(),
                }),
                Ok(ExtensionKind::EPISODE_V4) => decode::<Episode>(&any.value).ok().map(|e| Meta {
                    name: e.name().to_string(),
                    artists: e.show.get_or_default().name().to_string(),
                    duration_ms: e.duration.map(|d| d.max(0) as u64),
                    tracks: Vec::new(),
                }),
                _ => None,
            };
            if let Some(m) = meta {
                out.insert(uri, m);
            }
        }
    }
    Ok(out)
}

/// Métadonnées d'une liste d'adresses, par paquets de [`BATCH`]. `progress` est appelé après chaque paquet.
pub async fn metadata(session: &Session, uris: &[String], mut progress: impl FnMut(&HashMap<String, Meta>)) -> Result<HashMap<String, Meta>, Fault> {
    let mut out = HashMap::new();
    let wanted: Vec<&String> = uris.iter().filter(|u| kind_of(u).is_some()).collect();
    for chunk in wanted.chunks(BATCH) {
        let req = BatchedEntityRequest {
            entity_request: chunk
                .iter()
                .map(|u| EntityRequest {
                    entity_uri: u.to_string(),
                    query: vec![ExtensionQuery { extension_kind: EnumOrUnknown::new(kind_of(u).unwrap_or(ExtensionKind::TRACK_V4)), ..Default::default() }],
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let body = req.write_to_bytes().map_err(|e| Fault::Decode(e.to_string()))?;
        let resp = request(session, "/extended-metadata/v0/extended-metadata", PROTOBUF, Some((PROTOBUF, body))).await?;
        out.extend(parse_metadata(&resp)?);
        progress(&out);
    }
    Ok(out)
}

/// Lignes de morceaux à partir des adresses et des métadonnées (dans l'ordre des adresses).
pub fn track_items(uris: &[String], meta: &HashMap<String, Meta>) -> Vec<LibraryItem> {
    uris.iter()
        .filter(|u| is_playable_uri(u))
        .map(|u| match meta.get(u) {
            Some(m) => {
                let mut it = LibraryItem::new(LibraryKind::Track, if m.name.is_empty() { u.as_str() } else { m.name.as_str() }, m.artists.clone(), u.clone());
                it.duration_ms = m.duration_ms;
                it
            }
            None => LibraryItem::new(LibraryKind::Track, u.clone(), "", u.clone()),
        })
        .collect()
}

/// Morceaux d'un album (adresses), par ses métadonnées.
pub async fn album_uris(session: &Session, uri: &str) -> Result<(Vec<String>, String), Fault> {
    let meta = metadata(session, &[uri.to_string()], |_| {}).await?;
    let m = meta.get(uri).ok_or(Fault::Http(404))?;
    Ok((m.tracks.clone(), m.artists.clone()))
}

/// Recherche de titres par la résolution de contexte (`spotify:search:…`), seul moteur de recherche interne
/// stable. Renvoie les adresses des morceaux trouvés.
pub fn search_query(query: &str) -> String {
    query.split_whitespace().map(alpagamp_net::form_encode).collect::<Vec<_>>().join("+")
}

pub async fn search_tracks(session: &Session, query: &str) -> Result<Vec<String>, Fault> {
    let q = search_query(query);
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let body = request(session, &format!("/context-resolve/v1/spotify:search:{q}"), "application/json", None).await?;
    let v: serde_json::Value = serde_json::from_slice(&body).map_err(|e| Fault::Decode(e.to_string()))?;
    let mut uris = Vec::new();
    for page in v.get("pages").and_then(|p| p.as_array()).into_iter().flatten() {
        for t in page.get("tracks").and_then(|t| t.as_array()).into_iter().flatten() {
            if let Some(u) = t.get("uri").and_then(|u| u.as_str()) {
                if is_playable_uri(u) && !uris.iter().any(|x: &String| x == u) {
                    uris.push(u.to_string());
                }
            }
        }
    }
    Ok(uris)
}

#[cfg(test)]
mod tests {
    use super::*;
    use librespot_protocol::extended_metadata::EntityExtensionDataArray;
    use librespot_protocol::entity_extension_data::EntityExtensionData;
    use librespot_protocol::metadata::Artist;
    use librespot_protocol::playlist4_external::{Item, ListAttributes, ListItems, MetaItem};

    #[test]
    fn page_request_and_response_roundtrip() {
        let b = page_request("guiwald", "collection", "", 300);
        // username, set, limit
        assert_eq!(&b[..2], &[0x0a, 7]);
        assert!(b.windows(10).any(|w| w == b"collection"));
        assert_eq!(&b[b.len() - 3..], &[0x20, 0xac, 0x02]);
        // Réponse : deux éléments (dont un retiré) et un jeton de page.
        let mut resp = Vec::new();
        for (uri, at, removed) in [("spotify:track:a", 5, false), ("spotify:album:b", 9, true)] {
            let mut it = Vec::new();
            pb_string(&mut it, 1, uri);
            pb_varint(&mut it, 2 << 3);
            pb_varint(&mut it, at);
            if removed {
                pb_varint(&mut it, 3 << 3);
                pb_varint(&mut it, 1);
            }
            pb_varint(&mut resp, (1 << 3) | 2);
            pb_varint(&mut resp, it.len() as u64);
            resp.extend(it);
        }
        pb_string(&mut resp, 2, "next");
        pb_string(&mut resp, 3, "sync");
        let (items, next) = parse_page(&resp).unwrap();
        assert_eq!(items, vec![CollectionItem { uri: "spotify:track:a".into(), added_at: 5 }]);
        assert_eq!(next, "next");
    }

    #[test]
    fn metadata_is_decoded() {
        let mut t = Track::new();
        t.set_name("Comme hier".into());
        t.set_duration(193_000);
        let mut a = Artist::new();
        a.set_name("Camera Silens".into());
        t.artist.push(a);
        let mut album = Album::new();
        album.set_name("Rien qu'en traînant".into());
        let mut disc = librespot_protocol::metadata::Disc::new();
        let mut tr = Track::new();
        tr.set_gid(vec![0u8; 15].into_iter().chain([1u8]).collect());
        disc.track.push(tr);
        album.disc.push(disc);
        let mut resp = BatchedExtensionResponse::new();
        for (kind, uri, bytes) in [(ExtensionKind::TRACK_V4, "spotify:track:x", t.write_to_bytes().unwrap()), (ExtensionKind::ALBUM_V4, "spotify:album:y", album.write_to_bytes().unwrap())] {
            let mut arr = EntityExtensionDataArray::new();
            arr.extension_kind = EnumOrUnknown::new(kind);
            let mut d = EntityExtensionData::new();
            d.entity_uri = uri.into();
            let mut any = protobuf::well_known_types::any::Any::new();
            any.value = bytes;
            d.extension_data = protobuf::MessageField::some(any);
            arr.extension_data.push(d);
            resp.extended_metadata.push(arr);
        }
        let m = parse_metadata(&resp.write_to_bytes().unwrap()).unwrap();
        let tm = &m["spotify:track:x"];
        assert_eq!((tm.name.as_str(), tm.artists.as_str(), tm.duration_ms), ("Comme hier", "Camera Silens", Some(193_000)));
        assert_eq!(m["spotify:album:y"].tracks, vec!["spotify:track:0000000000000000000001".to_string()]);
        let items = track_items(&["spotify:track:x".into(), "spotify:local:z".into(), "spotify:track:q".into()], &m);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].playlist_title(), "Camera Silens - Comme hier");
        assert_eq!(items[1].title, "spotify:track:q");
    }

    #[test]
    fn rootlist_message_shape() {
        // Vérifie qu'on lit bien noms et propriétaires dans les « meta_items » parallèles aux éléments.
        let mut list = SelectedListContent::new();
        let mut contents = ListItems::new();
        contents.set_pos(0);
        contents.set_truncated(false);
        for (uri, name, owner) in [("spotify:start-group:g1:Mes+favoris", "", ""), ("spotify:playlist:p1", "Punk", "guiwald"), ("spotify:end-group:g1", "", ""), ("spotify:playlist:p2", "Hits", "spotify")] {
            let mut it = Item::new();
            it.set_uri(uri.into());
            contents.items.push(it);
            let mut m = MetaItem::new();
            let mut at = ListAttributes::new();
            at.set_name(name.into());
            m.attributes = protobuf::MessageField::some(at);
            m.set_owner_username(owner.into());
            contents.meta_items.push(m);
        }
        list.contents = protobuf::MessageField::some(contents);
        let parsed: SelectedListContent = decode(&list.write_to_bytes().unwrap()).unwrap();
        let (mut folders, mut out) = (Vec::new(), Vec::new());
        assert_eq!(rootlist_page(&parsed, "guiwald", &mut folders, &mut out), 4);
        assert!(folders.is_empty());
        assert_eq!(out.len(), 2);
        assert_eq!((out[0].title.as_str(), out[0].subtitle.as_str(), out[0].uri.as_str()), ("Punk", "Mes favoris", "spotify:playlist:p1"));
        assert_eq!((out[1].title.as_str(), out[1].subtitle.as_str()), ("Hits", "spotify"));
        assert_eq!(folder_name("Mes+favoris%21"), "Mes favoris!");
        assert_eq!(search_query("  never   gonna é "), "never+gonna+%C3%A9");
    }
}
