//! Page des versions (artefact claude.ai) tirée de `CHANGELOG.md`. Ne jamais l'écrire à la main.

use std::fmt::Write as _;

#[derive(Default)]
struct Version {
    name: String,
    date: Option<String>,
    service: Vec<String>,
    sections: Vec<(String, Vec<Entry>)>,
}

struct Entry {
    component: Option<String>,
    text: String,
}

fn parse(md: &str) -> Vec<Version> {
    let mut out: Vec<Version> = Vec::new();
    let mut cur: Option<Version> = None;
    for line in md.lines() {
        if let Some(rest) = line.strip_prefix("## [") {
            if let Some(v) = cur.take() {
                out.push(v);
            }
            let (name, tail) = rest.split_once(']').unwrap_or((rest, ""));
            let date = tail.trim().trim_start_matches('-').trim();
            cur = Some(Version { name: name.to_string(), date: (!date.is_empty()).then(|| date.to_string()), ..Default::default() });
            continue;
        }
        let Some(v) = cur.as_mut() else { continue };
        if let Some(q) = line.strip_prefix('>') {
            let q = q.trim();
            let q = q.strip_prefix("Mise en service :").map(str::trim).unwrap_or(q);
            if v.service.is_empty() {
                v.service.push(q.to_string());
            } else if let Some(last) = v.service.last_mut() {
                last.push(' ');
                last.push_str(q);
            }
        } else if let Some(h) = line.strip_prefix("### ") {
            v.sections.push((h.trim().to_string(), Vec::new()));
        } else if let Some(item) = line.strip_prefix("- ") {
            let (component, text) = match item.split_once(" : ") {
                Some((c, t)) if c.chars().count() <= 20 && !c.contains('`') => (Some(c.trim().to_string()), t.to_string()),
                _ => (None, item.to_string()),
            };
            if let Some((_, entries)) = v.sections.last_mut() {
                entries.push(Entry { component, text });
            }
        } else if line.starts_with("  ") && !line.trim().is_empty() {
            if let Some(e) = v.sections.last_mut().and_then(|(_, e)| e.last_mut()) {
                e.text.push(' ');
                e.text.push_str(line.trim());
            }
        }
    }
    if let Some(v) = cur {
        out.push(v);
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Markdown en ligne : `code`, **gras**, [lien](url).
fn inline(s: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut bold = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            if let Some(j) = chars[i + 1..].iter().position(|&x| x == '`') {
                let code: String = chars[i + 1..i + 1 + j].iter().collect();
                let _ = write!(out, "<code>{}</code>", esc(&code));
                i += j + 2;
                continue;
            }
        }
        if c == '*' && chars.get(i + 1) == Some(&'*') {
            out.push_str(if bold { "</strong>" } else { "<strong>" });
            bold = !bold;
            i += 2;
            continue;
        }
        if c == '[' {
            if let Some(j) = chars[i..].iter().position(|&x| x == ']') {
                if chars.get(i + j + 1) == Some(&'(') {
                    if let Some(k) = chars[i + j + 2..].iter().position(|&x| x == ')') {
                        let text: String = chars[i + 1..i + j].iter().collect();
                        let url: String = chars[i + j + 2..i + j + 2 + k].iter().collect();
                        let _ = write!(out, "<a href=\"{}\">{}</a>", esc(&url), esc(&text));
                        i += j + 3 + k;
                        continue;
                    }
                }
            }
        }
        out.push_str(&esc(&c.to_string()));
        i += 1;
    }
    if bold {
        out.push_str("</strong>");
    }
    out
}

fn semver(s: &str) -> Option<(u32, u32, u32)> {
    let p: Vec<u32> = s.split('.').map(|x| x.parse().ok()).collect::<Option<_>>()?;
    (p.len() == 3).then(|| (p[0], p[1], p[2]))
}

/// (classe, libellé, explication)
fn nature(v: &str, prev: Option<&str>) -> (&'static str, &'static str, &'static str) {
    match (semver(v), prev.and_then(semver)) {
        (Some(_), None) => ("n-premiere", "Première", "première version"),
        (Some(a), Some(b)) if a.0 != b.0 => ("n-majeur", "Majeur", "une action est nécessaire de ton côté"),
        (Some(a), Some(b)) if a.1 != b.1 => ("n-mineur", "Mineur", "nouvelles fonctions, rien à faire de ton côté"),
        (Some(_), Some(_)) => ("n-correctif", "Correctif", "corrections uniquement"),
        _ => ("n-preparation", "En préparation", "pas encore publié"),
    }
}

const MOIS: [&str; 12] = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];

fn date_fr(d: &str) -> String {
    let p: Vec<&str> = d.split('-').collect();
    if p.len() == 3 {
        if let (Ok(m), Ok(j)) = (p[1].parse::<usize>(), p[2].parse::<u32>()) {
            if (1..=12).contains(&m) {
                return format!("{j} {} {}", MOIS[m - 1], p[0]);
            }
        }
    }
    d.to_string()
}

fn rubrique_class(h: &str) -> &'static str {
    let l = h.to_lowercase();
    if l.starts_with("ajout") {
        "r-ajoute"
    } else if l.starts_with("modifi") {
        "r-modifie"
    } else if l.starts_with("corrig") {
        "r-corrige"
    } else if l.starts_with("sécurité") || l.starts_with("securite") {
        "r-securite"
    } else if l.starts_with("supprim") || l.starts_with("obsol") {
        "r-retire"
    } else {
        "r-note"
    }
}

pub fn render(md: &str) -> String {
    let all = parse(md);
    let versions: Vec<&Version> = all.iter().filter(|v| !(v.name.eq_ignore_ascii_case("non publié") && v.sections.iter().all(|(_, e)| e.is_empty()))).collect();
    let released: Vec<&Version> = versions.iter().copied().filter(|v| semver(&v.name).is_some()).collect();
    let current = released.first();
    let mut components: Vec<String> = Vec::new();
    let mut n_entries = 0;
    for v in &versions {
        for (_, es) in &v.sections {
            for e in es {
                n_entries += 1;
                if let Some(c) = &e.component {
                    if !components.contains(c) {
                        components.push(c.clone());
                    }
                }
            }
        }
    }
    let first_date = released.last().and_then(|v| v.date.clone()).map(|d| date_fr(&d)).unwrap_or_default();
    let total = format!(
        "{} version{} · {} changement{} · depuis le {}",
        released.len(),
        if released.len() > 1 { "s" } else { "" },
        n_entries,
        if n_entries > 1 { "s" } else { "" },
        first_date
    );

    let mut h = String::new();
    h.push_str(HEAD);
    let _ = write!(
        h,
        r#"<div class="page">
  <header class="tete">
    <div>
      <p class="surtitre">Journal des versions</p>
      <h1>Alpagamp</h1>
      <p class="chapo">Le lecteur à skins classiques, pour Mac, Windows et Android. Tout ce qui a changé, version par version, et ce qu'il faut faire pour l'installer.</p>
    </div>
    <div class="lcd" aria-label="Version en service">
      <div class="etiq">En service</div>
      <div class="vnum">{}</div>
      <div class="vdate">{}</div>
    </div>
    <p class="legende"><span><span class="pt n-majeur"></span><span><strong>Majeur</strong> : une action de ta part (désinstaller, refaire un réglage)</span></span><span><span class="pt n-mineur"></span><span><strong>Mineur</strong> : nouvelles fonctions, rien à faire</span></span><span><span class="pt n-correctif"></span><span><strong>Correctif</strong> : corrections uniquement</span></span></p>
  </header>
  <div class="outils">
    <input id="recherche" type="search" placeholder="Chercher : skin, égaliseur, Spotify, Windows…" aria-label="Chercher dans les versions">
    <div class="filtres" role="group" aria-label="Composant"><button type="button" data-c="tout" aria-pressed="true">Tout</button>"#,
        current.map(|v| v.name.as_str()).unwrap_or("—"),
        current.and_then(|v| v.date.as_deref()).map(|d| format!("depuis le {}", date_fr(d))).unwrap_or_default()
    );
    for c in &components {
        let _ = write!(h, r#"<button type="button" data-c="{}" aria-pressed="false">{}</button>"#, esc(&c.to_lowercase()), esc(c));
    }
    let _ = write!(h, r#"</div>
    <p class="compte" id="compte" data-total="{0}" aria-live="polite">{0}</p>
  </div>
  <div class="corps">
    <nav class="index" aria-label="Versions"><ol>"#, esc(&total));
    for (i, v) in versions.iter().enumerate() {
        let prev = versions.get(i + 1..).and_then(|rest| rest.iter().find(|p| semver(&p.name).is_some())).map(|p| p.name.as_str());
        let (cls, _, _) = nature(&v.name, prev);
        let id = anchor(&v.name);
        let day = v.date.as_deref().map(short_date).unwrap_or_default();
        let _ = write!(
            h,
            r##"<li><a href="#{id}" data-v="{id}"><span class="pt {cls}"></span><span class="inum">{}</span><span class="ijour">{day}</span></a></li>"##,
            esc(&v.name)
        );
    }
    h.push_str("</ol></nav>\n    <main class=\"fil\">");
    for (i, v) in versions.iter().enumerate() {
        let prev = versions.get(i + 1..).and_then(|rest| rest.iter().find(|p| semver(&p.name).is_some())).map(|p| p.name.as_str());
        let (cls, label, why) = nature(&v.name, prev);
        let id = anchor(&v.name);
        let mut comps: Vec<String> = Vec::new();
        for (_, es) in &v.sections {
            for e in es {
                if let Some(c) = &e.component {
                    if !comps.contains(c) {
                        comps.push(c.clone());
                    }
                }
            }
        }
        let prep = if semver(&v.name).is_none() { " preparation" } else { "" };
        let _ = write!(h, r#"<article class="version{prep}" id="{id}"><header class="vtete"><h2 class="num">{}</h2>"#, esc(&v.name));
        if let Some(d) = &v.date {
            let _ = write!(h, r#"<time datetime="{}" class="quand">{}</time>"#, esc(d), date_fr(d));
        }
        let _ = write!(h, r#"<span class="nature {cls}" title="{why}">{label}</span><span class="comps">"#);
        for c in &comps {
            let _ = write!(h, r#"<span class="comp">{}</span>"#, esc(c));
        }
        h.push_str("</span></header>");
        if !v.service.is_empty() {
            h.push_str(r#"<div class="service"><p class="etiquette">Mise en service</p>"#);
            for s in &v.service {
                let mut cs = s.chars();
                let cap: String = cs.next().map(|c| c.to_uppercase().chain(cs).collect()).unwrap_or_default();
                let _ = write!(h, "<p>{}</p>", inline(&cap));
            }
            h.push_str("</div>");
        }
        for (title, es) in &v.sections {
            if es.is_empty() {
                continue;
            }
            let _ = write!(h, r#"<section class="rubrique {}"><h3>{}</h3><ul class="entrees">"#, rubrique_class(title), esc(title));
            for e in es {
                let c = e.component.as_deref().map(|c| c.to_lowercase()).unwrap_or_default();
                let comp = e.component.as_deref().map(|c| format!("<span class=\"etag\">{}</span> ", esc(c))).unwrap_or_default();
                let _ = write!(h, r#"<li class="entree" data-c="{}">{comp}{}</li>"#, esc(&c), inline(&e.text));
            }
            h.push_str("</ul></section>");
        }
        h.push_str("</article>");
    }
    h.push_str(r#"<p class="vide" id="vide" hidden>Aucune version ne correspond.</p></main>
  </div>
  <p class="pied">Page générée depuis <code>CHANGELOG.md</code> par <code>cargo xtask changelog-page</code> · <a href="https://github.com/Guiwald/alpagamp">github.com/Guiwald/alpagamp</a></p>
</div>"#);
    h.push_str(SCRIPT);
    h
}

fn anchor(name: &str) -> String {
    if semver(name).is_some() { format!("v{name}") } else { "preparation".into() }
}

fn short_date(d: &str) -> String {
    let p: Vec<&str> = d.split('-').collect();
    if p.len() == 3 { format!("{}/{}", p[2].trim_start_matches('0'), p[1]) } else { d.to_string() }
}

const HEAD: &str = r#"<title>Versions d'Alpagamp</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Silkscreen&family=Atkinson+Hyperlegible:ital,wght@0,400;0,700;1,400&family=JetBrains+Mono:wght@500;700&display=swap">
<style>
/* Mise en page : en-tête avec un afficheur LCD (la version en service), puis index collant et fiches de version. */
/* Palette : celle du skin par défaut d'Alpagamp — graphite, LCD menthe, laine d'alpaga. */
:root {
  --bg: #eceef1; --card: #ffffff; --ink: #1b1e23; --muted: #5d6571; --line: #d6dbe1;
  --accent: #17804f; --accent-soft: #dcf1e5; --wool: #8a6a3c; --wool-soft: #f2e9da;
  --warn: #93650e; --warn-soft: #f6ebd2; --err: #b3261e; --err-soft: #f9dedc;
  --code-bg: #e7eaee; --wash: #f6f7f9;
  --lcd-bg: #07120c; --lcd-fg: #6cf2a2; --lcd-dim: #1d4a33;
  --pixel: "Silkscreen", "JetBrains Mono", ui-monospace, monospace;
  --sans: "Atkinson Hyperlegible", system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  --mono: "JetBrains Mono", ui-monospace, "SF Mono", Menlo, Consolas, monospace;
}
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) {
  --bg: #14161a; --card: #1d2025; --ink: #e7ebef; --muted: #9aa3ae; --line: #2d323a;
  --accent: #6cf2a2; --accent-soft: #163626; --wool: #e3cfa6; --wool-soft: #3a3226;
  --warn: #e3b75c; --warn-soft: #3a3020; --err: #f2998f; --err-soft: #3d2320;
  --code-bg: #262a30; --wash: #191c20; color-scheme: dark; } }
:root[data-theme="dark"] {
  --bg: #14161a; --card: #1d2025; --ink: #e7ebef; --muted: #9aa3ae; --line: #2d323a;
  --accent: #6cf2a2; --accent-soft: #163626; --wool: #e3cfa6; --wool-soft: #3a3226;
  --warn: #e3b75c; --warn-soft: #3a3020; --err: #f2998f; --err-soft: #3d2320;
  --code-bg: #262a30; --wash: #191c20; color-scheme: dark; }
body { background: var(--bg); color: var(--ink); font: 15px/1.6 var(--sans); }
.page { max-width: 1120px; margin: 0 auto; padding-inline: 16px; padding-block: 28px 56px; }
a { color: var(--accent); }
code { font-family: var(--mono); font-size: .84em; background: var(--code-bg); padding: .08em .35em; border-radius: 4px; overflow-wrap: anywhere; }
h1, h2, h3 { text-wrap: balance; margin: 0; }

.tete { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 20px 32px; align-items: end; padding-bottom: 22px; border-bottom: 1px solid var(--line); }
.surtitre { margin: 0 0 6px; font-size: 12px; letter-spacing: .1em; text-transform: uppercase; color: var(--wool); font-weight: 700; }
.tete h1 { font-family: var(--pixel); font-weight: 400; font-size: clamp(34px, 7vw, 54px); letter-spacing: .02em; line-height: 1; }
.chapo { margin: 12px 0 0; color: var(--muted); max-width: 60ch; }
.lcd { background: var(--lcd-bg); color: var(--lcd-fg); border-radius: 6px; padding: 10px 16px 12px; min-width: 190px;
  box-shadow: inset 0 0 0 1px #000, inset 0 2px 0 rgba(0,0,0,.6), 0 1px 0 rgba(255,255,255,.08); }
.lcd .etiq { font-family: var(--pixel); font-size: 11px; letter-spacing: .08em; color: var(--lcd-fg); opacity: .7; }
.lcd .vnum { font-family: var(--pixel); font-size: 34px; line-height: 1.15; font-variant-numeric: tabular-nums; text-shadow: 0 0 8px rgba(108,242,162,.35); }
.lcd .vdate { font-size: 12px; color: var(--lcd-fg); opacity: .75; }
.legende { grid-column: 1 / -1; display: flex; flex-wrap: wrap; gap: 6px 18px; margin: 0; font-size: 13px; color: var(--muted); }
.legende > span { display: inline-flex; align-items: baseline; gap: 7px; }

.outils { display: flex; flex-wrap: wrap; gap: 10px 14px; align-items: center; padding-block: 16px; }
.outils input { flex: 1 1 240px; min-width: 0; font: inherit; color: var(--ink); background: var(--card); border: 1px solid var(--line); border-radius: 8px; padding: 8px 12px; }
.outils input:focus-visible, .filtres button:focus-visible, .index a:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.filtres { display: flex; flex-wrap: wrap; gap: 6px; }
.filtres button { font: inherit; font-size: 13px; font-weight: 700; color: var(--ink); background: transparent; border: 1px solid var(--line); border-radius: 999px; padding: 4px 12px; cursor: pointer; }
.filtres button[aria-pressed="true"] { background: var(--ink); border-color: var(--ink); color: var(--bg); }
.compte { font-size: 13px; color: var(--muted); margin: 0; flex-basis: 100%; }

.corps { display: grid; grid-template-columns: 140px minmax(0, 1fr); gap: 28px; align-items: start; }
.index { position: sticky; top: calc(env(safe-area-inset-top, 0px) + 16px); max-height: calc(100vh - 32px); overflow-y: auto; font-size: 14px; }
.index ol { list-style: none; margin: 0; padding: 0; }
.index a { display: grid; grid-template-columns: 10px 1fr auto; gap: 8px; align-items: center; padding: 4px 8px; border-radius: 6px; text-decoration: none; color: var(--ink); }
.index a:hover { background: var(--card); }
.index a.eteint { opacity: .35; }
.inum { font-family: var(--mono); font-variant-numeric: tabular-nums; font-size: 13px; }
.ijour { font-size: 12px; color: var(--muted); font-variant-numeric: tabular-nums; }
.pt { width: 8px; height: 8px; border-radius: 2px; background: var(--muted); flex: none; }
.pt.n-mineur { background: var(--accent); } .pt.n-correctif { background: var(--warn); }
.pt.n-majeur { background: var(--err); } .pt.n-premiere { background: var(--wool); }

.fil { display: grid; gap: 18px; min-width: 0; }
.version { background: var(--card); border: 1px solid var(--line); border-radius: 12px; padding: 20px 22px; min-width: 0; scroll-margin-top: 16px; }
.version.preparation { background: transparent; border-style: dashed; }
.vtete { display: flex; flex-wrap: wrap; align-items: baseline; gap: 6px 12px; }
.num { font-family: var(--mono); font-size: 24px; font-weight: 700; font-variant-numeric: tabular-nums; }
.preparation .num { font-family: var(--sans); font-size: 18px; }
.quand { color: var(--muted); font-size: 14px; }
.nature { font-size: 12px; font-weight: 700; padding: 2px 9px; border-radius: 999px; cursor: help; }
.n-mineur.nature { background: var(--accent-soft); color: var(--accent); }
.n-correctif.nature { background: var(--warn-soft); color: var(--warn); }
.n-majeur.nature { background: var(--err-soft); color: var(--err); }
.n-premiere.nature { background: var(--wool-soft); color: var(--wool); }
.n-preparation.nature { background: var(--code-bg); color: var(--muted); }
.comps { display: inline-flex; flex-wrap: wrap; gap: 4px; margin-left: auto; }
.comp, .etag { font-size: 11px; letter-spacing: .05em; text-transform: uppercase; color: var(--muted); border: 1px solid var(--line); border-radius: 4px; padding: 0 6px; }
.etag { display: inline-block; margin-right: 2px; line-height: 1.6; transform: translateY(-1px); }

.service { margin-top: 14px; background: var(--wash); border: 1px solid var(--line); border-radius: 8px; padding: 10px 14px; font-size: 14px; }
.service p { margin: 6px 0; }
.etiquette, .rubrique h3 { margin: 0; font-size: 11px; letter-spacing: .1em; text-transform: uppercase; font-weight: 700; color: var(--muted); }
.rubrique { margin-top: 16px; }
.rubrique h3 { display: flex; align-items: center; gap: 8px; }
.rubrique h3::before { content: ""; width: 8px; height: 8px; border-radius: 2px; background: var(--muted); flex: none; }
.r-ajoute h3 { color: var(--accent); } .r-ajoute h3::before { background: var(--accent); }
.r-modifie h3 { color: var(--wool); } .r-modifie h3::before { background: var(--wool); }
.r-corrige h3 { color: var(--warn); } .r-corrige h3::before { background: var(--warn); }
.r-securite h3 { color: var(--err); } .r-securite h3::before { background: var(--err); }
.entrees { margin: 8px 0 0; padding-left: 20px; display: grid; gap: 8px; }
.entree::marker { color: var(--muted); }
.entree strong { font-weight: 700; }
.vide { margin: 0; padding: 28px; text-align: center; color: var(--muted); border: 1px dashed var(--line); border-radius: 12px; }
.pied { margin-top: 32px; font-size: 13px; color: var(--muted); }

@media (max-width: 760px) {
  .tete { grid-template-columns: minmax(0, 1fr); }
  .lcd { justify-self: start; }
  .corps { grid-template-columns: minmax(0, 1fr); gap: 14px; }
  .index { position: static; max-height: none; overflow-x: auto; padding-bottom: 6px; }
  .index ol { display: flex; gap: 4px; }
  .index a { grid-template-columns: 8px auto; white-space: nowrap; border: 1px solid var(--line); }
  .ijour { display: none; }
  .version { padding: 16px; }
  .comps { margin-left: 0; flex-basis: 100%; }
}
@media (prefers-reduced-motion: no-preference) { html { scroll-behavior: smooth; } }
</style>
"#;

const SCRIPT: &str = r#"
<script>
(function () {
  var champ = document.getElementById('recherche');
  var boutons = Array.prototype.slice.call(document.querySelectorAll('.filtres button'));
  var compte = document.getElementById('compte');
  var vide = document.getElementById('vide');
  var filtre = 'tout';
  try { var f = localStorage.getItem('alpagamp-versions-filtre'); if (f && document.querySelector('.filtres button[data-c="' + f + '"]')) filtre = f; } catch (e) {}
  function norm(s) { return s.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, ''); }
  function appliquer() {
    var q = norm(champ.value.trim());
    var nbV = 0, nbE = 0;
    boutons.forEach(function (b) { b.setAttribute('aria-pressed', b.dataset.c === filtre ? 'true' : 'false'); });
    document.querySelectorAll('.version').forEach(function (v) {
      var visibles = 0;
      v.querySelectorAll('.entree').forEach(function (e) {
        var okC = filtre === 'tout' || (' ' + e.dataset.c + ' ').indexOf(' ' + filtre + ' ') >= 0;
        var okQ = !q || norm(e.textContent).indexOf(q) >= 0;
        e.hidden = !(okC && okQ);
        if (!e.hidden) visibles++;
      });
      v.querySelectorAll('.rubrique').forEach(function (r) { r.hidden = !r.querySelector('.entree:not([hidden])'); });
      var montre = visibles > 0 || (filtre === 'tout' && !q);
      v.hidden = !montre;
      var lien = document.querySelector('.index a[data-v="' + v.id + '"]');
      if (lien) lien.classList.toggle('eteint', !montre);
      if (montre) { nbV++; nbE += visibles; }
    });
    vide.hidden = nbV > 0;
    compte.textContent = (filtre === 'tout' && !q) ? compte.dataset.total : nbV + ' version(s) · ' + nbE + ' changement(s)';
  }
  boutons.forEach(function (b) { b.addEventListener('click', function () { filtre = b.dataset.c; try { localStorage.setItem('alpagamp-versions-filtre', filtre); } catch (e) {} appliquer(); }); });
  champ.addEventListener('input', appliquer);
  appliquer();
})();
</script>
"#;
