//! Le skin par défaut d'Alpagamp : une création originale, dessinée par le programme.
//!
//! Il respecte la disposition du format classique (pour que tout skin .wsz s'y substitue), mais chaque
//! pixel est le nôtre : métal graphite brossé, afficheurs LCD vert menthe, accents couleur laine d'alpaga.

mod draw;
mod fonts;

use std::collections::HashMap;

use draw::*;
use fonts::*;

use crate::config::{PlEditConfig, VisColors};
use crate::image::{Image, rgb};
use crate::sprites::Sheet;
use crate::Skin;

// --- Palette ------------------------------------------------------------------------------------

const EDGE: u32 = rgb(0x0a, 0x0b, 0x0e);
const METAL_TOP: u32 = rgb(0x4b, 0x51, 0x5b);
const METAL_BOT: u32 = rgb(0x2a, 0x2e, 0x35);
const METAL_HI: u32 = rgb(0x80, 0x88, 0x95);
const METAL_LO: u32 = rgb(0x15, 0x17, 0x1b);
const KEY_TOP: u32 = rgb(0x5f, 0x66, 0x72);
const KEY_BOT: u32 = rgb(0x39, 0x3e, 0x47);
const KEY_DOWN_TOP: u32 = rgb(0x26, 0x2a, 0x30);
const KEY_DOWN_BOT: u32 = rgb(0x34, 0x39, 0x41);
const LCD_BG: u32 = rgb(0x05, 0x0f, 0x0a);
const LCD_GHOST: u32 = rgb(0x0f, 0x22, 0x18);
const LCD_DIM: u32 = rgb(0x24, 0x5e, 0x40);
const LCD_FG: u32 = rgb(0x6c, 0xf2, 0xa2);
const LCD_HI: u32 = rgb(0xc8, 0xff, 0xde);
const WOOL: u32 = rgb(0xec, 0xdc, 0xbb);
const WOOL_DIM: u32 = rgb(0x98, 0x8c, 0x76);
const WOOL_DARK: u32 = rgb(0x6e, 0x5f, 0x4a);
const LABEL: u32 = rgb(0xb4, 0xbb, 0xc6);
const ENGRAVE: u32 = rgb(0x1a, 0x1d, 0x22);
const LED_ON: u32 = rgb(0xff, 0xa8, 0x3c);
const LED_ON_HI: u32 = rgb(0xff, 0xe2, 0xa8);
const LED_OFF: u32 = rgb(0x46, 0x34, 0x22);
const RED: u32 = rgb(0xf0, 0x5a, 0x4a);

/// Construit le skin par défaut complet.
pub fn build() -> Skin {
    let mut sheets = HashMap::new();
    sheets.insert(Sheet::Main, main_bmp());
    sheets.insert(Sheet::Titlebar, titlebar_bmp());
    sheets.insert(Sheet::Cbuttons, cbuttons_bmp());
    sheets.insert(Sheet::Shufrep, shufrep_bmp());
    sheets.insert(Sheet::Posbar, posbar_bmp());
    sheets.insert(Sheet::Volume, volume_bmp());
    sheets.insert(Sheet::Balance, balance_bmp());
    sheets.insert(Sheet::Monoster, monoster_bmp());
    sheets.insert(Sheet::Playpaus, playpaus_bmp());
    sheets.insert(Sheet::NumsEx, nums_ex_bmp());
    sheets.insert(Sheet::Numbers, numbers_bmp());
    sheets.insert(Sheet::Text, text_bmp());
    sheets.insert(Sheet::EqMain, eqmain_bmp());
    sheets.insert(Sheet::EqEx, eq_ex_bmp());
    sheets.insert(Sheet::PlEdit, pledit_bmp());
    sheets.insert(Sheet::Gen, gen_bmp());
    sheets.insert(Sheet::GenEx, genex_bmp());
    let mut skin = Skin::from_parts("Alpagamp", sheets, vis_colors(), pledit_config());
    skin.readme = Some(
        "Alpagamp — skin par défaut.\r\nCréation originale (métal graphite, LCD menthe, laine d'alpaga).\r\nLicence MIT, comme le lecteur.\r\n".into(),
    );
    skin
}

fn vis_colors() -> VisColors {
    let mut c = [0u32; 24];
    c[0] = LCD_BG;
    c[1] = rgb(0x14, 0x2c, 0x20);
    // Dégradé de l'analyseur : 2 (haut) → 17 (bas) : ambre chaud → menthe → vert profond.
    let stops = [
        (0.0, rgb(0xff, 0x8a, 0x4c)),
        (0.25, rgb(0xff, 0xc8, 0x5a)),
        (0.5, rgb(0xb8, 0xf5, 0x7a)),
        (0.75, rgb(0x5c, 0xe8, 0x9a)),
        (1.0, rgb(0x1e, 0x9e, 0x6a)),
    ];
    for i in 0..16 {
        let t = i as f32 / 15.0;
        let mut col = stops[0].1;
        for w in stops.windows(2) {
            if t >= w[0].0 && t <= w[1].0 {
                col = mix(w[0].1, w[1].1, (t - w[0].0) / (w[1].0 - w[0].0));
            }
        }
        c[2 + i] = col;
    }
    // Oscilloscope : clair au centre, plus sombre vers les bords.
    c[18] = LCD_HI;
    c[19] = rgb(0x9c, 0xf7, 0xc2);
    c[20] = LCD_FG;
    c[21] = rgb(0x44, 0xc0, 0x80);
    c[22] = rgb(0x2a, 0x88, 0x5c);
    c[23] = WOOL;
    VisColors(c)
}

fn pledit_config() -> PlEditConfig {
    PlEditConfig {
        normal: LCD_FG,
        current: LCD_HI,
        normal_bg: LCD_BG,
        selected_bg: rgb(0x1f, 0x4a, 0x38),
        mb_fg: LCD_FG,
        mb_bg: LCD_BG,
        font: "Arial".into(),
    }
}

// --- Éléments communs -----------------------------------------------------------------------------

/// Corps métallique d'une fenêtre (contour, biseau, métal brossé).
fn body(img: &mut Image, x: i32, y: i32, w: i32, h: i32, seed: u32) {
    brushed(img, x, y, w, h, METAL_TOP, METAL_BOT, seed, 5.0);
    frame(img, x, y, w, h, EDGE);
    bevel(img, x + 1, y + 1, w - 2, h - 2, METAL_HI, METAL_LO);
}

/// Écran LCD creusé.
fn lcd(img: &mut Image, x: i32, y: i32, w: i32, h: i32) {
    recess(img, x, y, w, h, LCD_BG, METAL_LO, METAL_HI);
    frame(img, x + 1, y + 1, w - 2, h - 2, EDGE);
    rect(img, x + 2, y + 2, w - 4, h - 4, LCD_BG);
}

/// Étiquette gravée : ombre sombre décalée + texte clair.
fn engraved(img: &mut Image, x: i32, y: i32, text: &str) {
    tiny_text(img, x + 1, y + 1, text, ENGRAVE);
    tiny_text(img, x, y, text, LABEL);
}

/// Bouton de fenêtre 9x9 : touche + icône 5x5.
fn win_button(img: &mut Image, x: i32, y: i32, icon: &[&str], pressed: bool) {
    if pressed {
        key(img, x, y, 9, 9, KEY_DOWN_TOP, KEY_DOWN_BOT, METAL_LO, KEY_TOP, EDGE);
        glyph(img, x + 2, y + 2, icon, WOOL);
    } else {
        key(img, x, y, 9, 9, KEY_TOP, KEY_BOT, METAL_HI, METAL_LO, EDGE);
        glyph(img, x + 2, y + 2, icon, WOOL);
    }
}

const ICON_OPTIONS: &[&str] = &["#####", ".....", "#####", ".....", "#####"];
const ICON_MINIMIZE: &[&str] = &[".....", ".....", ".....", "#####", "#####"];
const ICON_SHADE: &[&str] = &["#####", "#####", ".....", ".....", "....."];
const ICON_UNSHADE: &[&str] = &["#####", "#...#", "#...#", "#...#", "#####"];
const ICON_CLOSE: &[&str] = &["#...#", ".#.#.", "..#..", ".#.#.", "#...#"];

/// Bande de titre 275x14 (ou de largeur `w`) avec un texte centré et des rainures.
fn title_strip(img: &mut Image, x: i32, y: i32, w: i32, title: &str, active: bool, buttons: &[(i32, &[&str])]) {
    let (top, bot) = if active { (rgb(0x5a, 0x61, 0x6d), rgb(0x33, 0x38, 0x40)) } else { (rgb(0x3e, 0x43, 0x4b), rgb(0x2a, 0x2e, 0x34)) };
    brushed(img, x, y, w, 14, top, bot, 17, 4.0);
    frame(img, x, y, w, 14, EDGE);
    hline(img, x + 1, y + 1, w - 2, if active { METAL_HI } else { rgb(0x5a, 0x60, 0x6a) });
    let tw = small_text_width(title);
    let tx = x + (w - tw) / 2;
    // Rainures de préhension de part et d'autre du titre.
    let left_start = x + 18;
    let left_end = tx - 5;
    let right_start = tx + tw + 5;
    let right_end = buttons.iter().map(|b| x + b.0).filter(|&bx| bx > tx).min().unwrap_or(x + w - 4) - 4;
    for gy in [5, 8] {
        let (hi, lo) = if active { (rgb(0x8e, 0x96, 0xa3), EDGE) } else { (rgb(0x5e, 0x64, 0x6e), EDGE) };
        if left_end > left_start {
            hline(img, left_start, y + gy, left_end - left_start, hi);
            hline(img, left_start, y + gy + 1, left_end - left_start, lo);
        }
        if right_end > right_start {
            hline(img, right_start, y + gy, right_end - right_start, hi);
            hline(img, right_start, y + gy + 1, right_end - right_start, lo);
        }
    }
    let color = if active { WOOL } else { WOOL_DIM };
    small_text(img, tx + 1, y + 5, title, EDGE);
    small_text(img, tx, y + 4, title, color);
    for (bx, icon) in buttons {
        win_button(img, x + bx, y + 3, icon, false);
    }
}

/// Petite LED 3x3.
fn led(img: &mut Image, x: i32, y: i32, on: bool) {
    if on {
        rect(img, x, y, 3, 3, LED_ON);
        px(img, x, y, LED_ON_HI);
        frame(img, x - 1, y - 1, 5, 5, rgb(0x5a, 0x30, 0x10));
    } else {
        rect(img, x, y, 3, 3, LED_OFF);
        frame(img, x - 1, y - 1, 5, 5, EDGE);
    }
}

/// Une touche rectangulaire (normale ou enfoncée).
fn button(img: &mut Image, x: i32, y: i32, w: i32, h: i32, pressed: bool) {
    if pressed {
        key(img, x, y, w, h, KEY_DOWN_TOP, KEY_DOWN_BOT, METAL_LO, rgb(0x4a, 0x50, 0x5a), EDGE);
    } else {
        key(img, x, y, w, h, KEY_TOP, KEY_BOT, METAL_HI, METAL_LO, EDGE);
    }
}

/// Tête d'alpaga (logo), 13x15.
const ALPACA: &[&str] = &[
    "..oo....oo...",
    "..ow....wo...",
    ".owwwwwwwwo..",
    "owwwwwwwwwwo.",
    "owwwwwwwwwwo.",
    ".owewwwwewo..",
    ".owwwwwwwwo..",
    "..owwwwwwo...",
    "..owwnnwwo...",
    "..owwwwwwo...",
    "...owwwwo....",
    "...owwwwo....",
    "...owwwwo....",
    "...owwwwwo...",
    "...oooooo....",
];

fn alpaca(img: &mut Image, x: i32, y: i32) {
    pattern(img, x, y, ALPACA, &[('o', WOOL_DARK), ('w', WOOL), ('e', EDGE), ('n', rgb(0x8a, 0x5a, 0x4a))]);
}

// --- main.bmp --------------------------------------------------------------------------------------

fn main_bmp() -> Image {
    let mut img = Image::new(275, 116, METAL_BOT);
    body(&mut img, 0, 0, 275, 116, 1);

    // Afficheur de gauche (clutterbar, état, temps, visualiseur).
    lcd(&mut img, 8, 20, 99, 47);
    // Chiffres fantômes « 88:88 » (visibles à l'arrêt) et deux-points.
    for x in [48, 60, 78, 90] {
        seven_seg(&mut img, x, 26, 8, false, LCD_GHOST, LCD_GHOST);
    }
    rect(&mut img, 72, 29, 2, 2, LCD_DIM);
    rect(&mut img, 72, 34, 2, 2, LCD_DIM);

    // Afficheur du titre.
    lcd(&mut img, 108, 22, 159, 15);
    // Débit et fréquence.
    lcd(&mut img, 108, 40, 21, 11);
    engraved(&mut img, 131, 43, "KBPS");
    lcd(&mut img, 153, 40, 16, 11);
    engraved(&mut img, 171, 43, "KHZ");
    // Logement mono / stéréo.
    lcd(&mut img, 210, 39, 59, 15);

    // Rail du volume et de la balance (visibles avant que les curseurs ne soient dessinés).
    recess(&mut img, 107, 57, 68, 13, METAL_LO, EDGE, METAL_HI);
    recess(&mut img, 177, 57, 38, 13, METAL_LO, EDGE, METAL_HI);

    // Rainure de la barre de position (visible à l'arrêt).
    posbar_groove(&mut img, 16, 72);

    // Logement des boutons de transport.
    recess(&mut img, 14, 86, 118, 22, rgb(0x1e, 0x21, 0x27), EDGE, METAL_HI);
    recess(&mut img, 134, 87, 26, 20, rgb(0x1e, 0x21, 0x27), EDGE, METAL_HI);
    recess(&mut img, 162, 87, 78, 19, rgb(0x1e, 0x21, 0x27), EDGE, METAL_HI);

    // Grille décorative et logo.
    for gx in (242..251).step_by(2) {
        vline(&mut img, gx, 89, 16, EDGE);
        vline(&mut img, gx + 1, 89, 16, rgb(0x56, 0x5d, 0x68));
    }
    alpaca(&mut img, 253, 91);
    img
}

/// Rainure de la barre de position (248x10).
fn posbar_groove(img: &mut Image, x: i32, y: i32) {
    recess(img, x, y, 248, 10, rgb(0x1a, 0x1d, 0x22), EDGE, METAL_HI);
    hline(img, x + 3, y + 4, 242, EDGE);
    hline(img, x + 3, y + 5, 242, rgb(0x2c, 0x31, 0x38));
    // Graduations.
    for i in 0..=10 {
        let gx = x + 3 + i * 242 / 10;
        px(img, gx, y + 2, rgb(0x3c, 0x42, 0x4b));
        px(img, gx, y + 7, rgb(0x3c, 0x42, 0x4b));
    }
}

// --- Chiffres 7 segments -------------------------------------------------------------------------

/// Segments allumés par chiffre (a, b, c, d, e, f, g). 10 = aucun, 11 = signe moins.
fn segments(d: u8) -> [bool; 7] {
    match d {
        0 => [true, true, true, true, true, true, false],
        1 => [false, true, true, false, false, false, false],
        2 => [true, true, false, true, true, false, true],
        3 => [true, true, true, true, false, false, true],
        4 => [false, true, true, false, false, true, true],
        5 => [true, false, true, true, false, true, true],
        6 => [true, false, true, true, true, true, true],
        7 => [true, true, true, false, false, false, false],
        8 => [true; 7],
        9 => [true, true, true, true, false, true, true],
        11 => [false, false, false, false, false, false, true],
        _ => [false; 7],
    }
}

/// Dessine un chiffre 7 segments dans une cellule 9x13 en (x, y) ; segments éteints en `ghost`.
fn seven_seg(img: &mut Image, x: i32, y: i32, d: u8, with_bg: bool, ghost: u32, lit_color: u32) {
    if with_bg {
        rect(img, x, y, 9, 13, LCD_BG);
    }
    let on = segments(d);
    let ox = x + 1;
    let oy = y + 1;
    // (segment, rectangles) : verticaux 2 px, horizontaux 1 px.
    let shapes: [(usize, [(i32, i32, i32, i32); 1]); 7] = [
        (0, [(1, 0, 5, 1)]),
        (1, [(5, 1, 2, 4)]),
        (2, [(5, 6, 2, 4)]),
        (3, [(1, 10, 5, 1)]),
        (4, [(0, 6, 2, 4)]),
        (5, [(0, 1, 2, 4)]),
        (6, [(1, 5, 5, 1)]),
    ];
    for (seg, rects) in shapes {
        let c = if on[seg] { lit_color } else { ghost };
        for (rx, ry, rw, rh) in rects {
            rect(img, ox + rx, oy + ry, rw, rh, c);
        }
    }
    // Reflet : premier pixel des segments horizontaux allumés plus clair.
    if lit_color == LCD_FG {
        for (seg, ry) in [(0usize, 0)] {
            if on[seg] {
                px(img, ox + 1, oy + ry, LCD_HI);
            }
        }
    }
}

fn nums_ex_bmp() -> Image {
    let mut img = Image::new(108, 13, LCD_BG);
    for d in 0..10u8 {
        seven_seg(&mut img, d as i32 * 9, 0, d, true, LCD_GHOST, LCD_FG);
    }
    seven_seg(&mut img, 90, 0, 10, true, LCD_GHOST, LCD_FG);
    seven_seg(&mut img, 99, 0, 11, true, LCD_GHOST, LCD_FG);
    img
}

fn numbers_bmp() -> Image {
    let mut img = Image::new(99, 13, LCD_BG);
    for d in 0..10u8 {
        seven_seg(&mut img, d as i32 * 9, 0, d, true, LCD_GHOST, LCD_FG);
    }
    img
}

// --- text.bmp --------------------------------------------------------------------------------------

fn text_bmp() -> Image {
    let mut img = Image::new(155, 18, LCD_BG);
    let rows: [&str; 3] = [
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ\"@   ",
        "0123456789….:()-'!_+\\/[]^&%,=$#",
        "ÅÖÄ?*                          ",
    ];
    for (r, line) in rows.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let g = small(ch);
            let w = g.first().map(|l| l.len()).unwrap_or(0) as i32;
            // Centre les glyphes étroits dans la cellule de 5 px (4 utiles + 1 d'espace).
            let off = if w <= 2 { 1 } else if w == 3 { 0 } else { 0 };
            glyph(&mut img, c as i32 * 5 + off, r as i32 * 6, g, LCD_FG);
        }
    }
    img
}

// --- titlebar.bmp ----------------------------------------------------------------------------------

fn titlebar_bmp() -> Image {
    let mut img = Image::new(344, 87, METAL_BOT);
    brushed(&mut img, 0, 0, 344, 87, METAL_TOP, METAL_BOT, 3, 4.0);

    // Boutons 9x9.
    win_button(&mut img, 0, 0, ICON_OPTIONS, false);
    win_button(&mut img, 0, 9, ICON_OPTIONS, true);
    win_button(&mut img, 9, 0, ICON_MINIMIZE, false);
    win_button(&mut img, 9, 9, ICON_MINIMIZE, true);
    win_button(&mut img, 18, 0, ICON_CLOSE, false);
    win_button(&mut img, 18, 9, ICON_CLOSE, true);
    win_button(&mut img, 0, 18, ICON_SHADE, false);
    win_button(&mut img, 9, 18, ICON_SHADE, true);
    win_button(&mut img, 0, 27, ICON_UNSHADE, false);
    win_button(&mut img, 9, 27, ICON_UNSHADE, true);

    let buttons: [(i32, &[&str]); 4] = [(6, ICON_OPTIONS), (244, ICON_MINIMIZE), (254, ICON_SHADE), (264, ICON_CLOSE)];
    title_strip(&mut img, 27, 0, 275, "ALPAGAMP", true, &buttons);
    title_strip(&mut img, 27, 15, 275, "ALPAGAMP", false, &buttons);
    title_strip(&mut img, 27, 57, 275, "ALPAGAMP * 100% LAINE", true, &buttons);
    title_strip(&mut img, 27, 72, 275, "ALPAGAMP * 100% LAINE", false, &buttons);
    // Rangées 28 et 41 : séparateurs entre bandes (une ligne commune, comme le format d'origine).
    shade_strip(&mut img, 27, 29, true);
    shade_strip(&mut img, 27, 42, false);

    // Mini barre de position du mode bandeau.
    rect(&mut img, 0, 36, 17, 7, rgb(0x1a, 0x1d, 0x22));
    frame(&mut img, 0, 36, 17, 7, EDGE);
    hline(&mut img, 2, 39, 13, rgb(0x2c, 0x31, 0x38));
    for (i, x) in [17, 20, 23].into_iter().enumerate() {
        rect(&mut img, x, 36, 3, 7, WOOL_DIM);
        vline(&mut img, x + i as i32 % 3, 37, 5, WOOL);
        frame(&mut img, x, 36, 3, 7, EDGE);
        let _ = i;
    }

    clutterbar(&mut img, 304, 0, true);
    clutterbar(&mut img, 312, 0, false);
    // Lettres enfoncées : O A I D V.
    let downs = [(304, 47, 8, 'O'), (312, 55, 7, 'A'), (320, 62, 7, 'I'), (328, 69, 8, 'D'), (336, 77, 7, 'V')];
    for (x, y, h, ch) in downs {
        rect(&mut img, x, y, 8, h, WOOL);
        vline(&mut img, x, y, h, EDGE);
        vline(&mut img, x + 7, y, h, EDGE);
        glyph(&mut img, x + 3, y + (h - 5) / 2 + 1, &tiny(ch)[..], EDGE);
    }
    img
}

/// Bande du mode bandeau (fenêtre principale) : 275x14.
fn shade_strip(img: &mut Image, x: i32, y: i32, active: bool) {
    let (top, bot) = if active { (rgb(0x5a, 0x61, 0x6d), rgb(0x33, 0x38, 0x40)) } else { (rgb(0x3e, 0x43, 0x4b), rgb(0x2a, 0x2e, 0x34)) };
    brushed(img, x, y, 275, 14, top, bot, 23, 4.0);
    frame(img, x, y, 275, 14, EDGE);
    win_button(img, x + 6, y + 3, ICON_OPTIONS, false);
    let color = if active { WOOL } else { WOOL_DIM };
    small_text(img, x + 19, y + 4, "ALPAGAMP", color);
    // Mini visualiseur et temps.
    rect(img, x + 78, y + 4, 40, 7, LCD_BG);
    frame(img, x + 77, y + 3, 42, 9, EDGE);
    rect(img, x + 125, y + 3, 33, 8, LCD_BG);
    frame(img, x + 124, y + 2, 35, 10, EDGE);
    // Mini transport : précédent, lecture, pause, stop, suivant, éjecter.
    let ic = if active { WOOL } else { WOOL_DIM };
    glyph(img, x + 170, y + 4, &["#..#", "#.##", "####", "#.##", "#..#"], ic);
    glyph(img, x + 179, y + 4, &["#..", "##.", "###", "##.", "#.."], ic);
    glyph(img, x + 188, y + 4, &["#.#", "#.#", "#.#", "#.#", "#.#"], ic);
    glyph(img, x + 197, y + 4, &["####", "####", "####", "####"], ic);
    glyph(img, x + 207, y + 4, &["#..#", "##.#", "####", "##.#", "#..#"], ic);
    glyph(img, x + 217, y + 4, &["..#..", ".###.", "#####", ".....", "#####"], ic);
    // Logement de la mini barre de position.
    frame(img, x + 225, y + 3, 19, 9, EDGE);
    win_button(img, x + 244, y + 3, ICON_MINIMIZE, false);
    win_button(img, x + 254, y + 3, ICON_UNSHADE, false);
    win_button(img, x + 264, y + 3, ICON_CLOSE, false);
}

fn clutterbar(img: &mut Image, x: i32, y: i32, enabled: bool) {
    vgrad(img, x, y, 8, 43, rgb(0x2a, 0x2e, 0x35), rgb(0x1e, 0x21, 0x27));
    vline(img, x, y, 43, EDGE);
    vline(img, x + 7, y, 43, METAL_HI);
    let color = if enabled { WOOL_DIM } else { rgb(0x4a, 0x46, 0x40) };
    for (ch, oy, h) in [('O', 3, 8), ('A', 11, 7), ('I', 18, 7), ('D', 25, 8), ('V', 33, 7)] {
        glyph(img, x + 3, y + oy + (h - 5) / 2 + 1, &tiny(ch)[..], color);
    }
}

// --- cbuttons.bmp ----------------------------------------------------------------------------------

const ICON_PREV: &[&str] = &["#...#...#", "#..##..##", "#.###.###", "#########", "#.###.###", "#..##..##", "#...#...#"];
const ICON_PLAY: &[&str] = &["#....", "##...", "###..", "####.", "###..", "##...", "#...."];
const ICON_PAUSE: &[&str] = &["##.##", "##.##", "##.##", "##.##", "##.##", "##.##", "##.##"];
const ICON_STOP: &[&str] = &["######", "######", "######", "######", "######", "######"];
const ICON_NEXT: &[&str] = &["#...#...#", "##..##..#", "###.###.#", "#########", "###.###.#", "##..##..#", "#...#...#"];
const ICON_EJECT: &[&str] = &["...#...", "..###..", ".#####.", "#######", ".......", "#######"];

fn cbuttons_bmp() -> Image {
    let mut img = Image::new(136, 36, METAL_BOT);
    let buttons: [(i32, i32, &[&str]); 5] = [(0, 23, ICON_PREV), (23, 23, ICON_PLAY), (46, 23, ICON_PAUSE), (69, 23, ICON_STOP), (92, 22, ICON_NEXT)];
    for (x, w, icon) in buttons {
        for (row, pressed) in [(0, false), (18, true)] {
            button(&mut img, x, row, w, 18, pressed);
            let iw = icon[0].len() as i32;
            let ih = icon.len() as i32;
            let o = if pressed { 1 } else { 0 };
            let ix = x + (w - iw) / 2 + o;
            let iy = row + (18 - ih) / 2 + o;
            glyph(&mut img, ix + 1, iy + 1, icon, EDGE);
            glyph(&mut img, ix, iy, icon, if pressed { WOOL_DIM } else { WOOL });
        }
    }
    for (row, pressed) in [(0, false), (16, true)] {
        button(&mut img, 114, row, 22, 16, pressed);
        let o = if pressed { 1 } else { 0 };
        glyph(&mut img, 122 + o, row + 6 + o, ICON_EJECT, EDGE);
        glyph(&mut img, 121 + o, row + 5 + o, ICON_EJECT, if pressed { WOOL_DIM } else { WOOL });
    }
    img
}

// --- shufrep.bmp -----------------------------------------------------------------------------------

const ICON_REPEAT: &[&str] = &["..#######.", ".##......#", "###......#", ".#.......#", "..#######."];

fn shufrep_bmp() -> Image {
    let mut img = Image::new(92, 85, METAL_BOT);
    // Rangées : 0 éteint, 15 éteint enfoncé, 30 allumé, 45 allumé enfoncé.
    for (row, on, pressed) in [(0, false, false), (15, false, true), (30, true, false), (45, true, true)] {
        // Répéter.
        button(&mut img, 0, row, 28, 15, pressed);
        let o = pressed as i32;
        led(&mut img, 4 + o, row + 6 + o, on);
        glyph(&mut img, 13 + o, row + 5 + o, ICON_REPEAT, if on { WOOL } else { WOOL_DIM });
        // Aléatoire.
        button(&mut img, 28, row, 47, 15, pressed);
        led(&mut img, 32 + o, row + 6 + o, on);
        let c = if on { WOOL } else { WOOL_DIM };
        tiny_text(&mut img, 39 + o, row + 5 + o, "SHUFFLE", c);
    }
    // Boutons EQ / PL : 61 éteint, 73 allumé ; enfoncés en x + 46.
    for (row, on) in [(61, false), (73, true)] {
        for (x, label, pressed) in [(0, "EQ", false), (23, "PL", false), (46, "EQ", true), (69, "PL", true)] {
            button(&mut img, x, row, 23, 12, pressed);
            let o = pressed as i32;
            led(&mut img, x + 4 + o, row + 5 + o, on);
            tiny_text(&mut img, x + 11 + o, row + 4 + o, label, if on { WOOL } else { WOOL_DIM });
        }
    }
    img
}

// --- posbar.bmp ------------------------------------------------------------------------------------

fn posbar_bmp() -> Image {
    let mut img = Image::new(307, 10, METAL_BOT);
    posbar_groove(&mut img, 0, 0);
    for (x, pressed) in [(248, false), (278, true)] {
        slider_thumb(&mut img, x, 0, 29, 10, pressed);
    }
    img
}

/// Curseur métallique avec rainures de préhension.
fn slider_thumb(img: &mut Image, x: i32, y: i32, w: i32, h: i32, pressed: bool) {
    if pressed {
        key(img, x, y, w, h, rgb(0xd8, 0xc8, 0xa6), rgb(0xa8, 0x98, 0x7c), WOOL, WOOL_DARK, EDGE);
    } else {
        key(img, x, y, w, h, rgb(0x8a, 0x92, 0x9e), rgb(0x55, 0x5c, 0x67), rgb(0xb8, 0xc0, 0xcc), METAL_LO, EDGE);
    }
    let cx = x + w / 2;
    for gx in [cx - 3, cx, cx + 3] {
        if gx > x + 2 && gx < x + w - 3 {
            vline(img, gx, y + 3, h - 6, EDGE);
            vline(img, gx + 1, y + 3, h - 6, if pressed { LED_ON_HI } else { rgb(0xa8, 0xb0, 0xbc) });
        }
    }
}

// --- volume.bmp / balance.bmp -----------------------------------------------------------------------

/// Couleur du niveau (0 = bas, 1 = haut) : menthe → ambre → corail.
fn level_color(t: f32) -> u32 {
    if t < 0.6 {
        mix(rgb(0x2e, 0xb8, 0x7a), rgb(0x9c, 0xf0, 0x7a), t / 0.6)
    } else if t < 0.85 {
        mix(rgb(0x9c, 0xf0, 0x7a), LED_ON, (t - 0.6) / 0.25)
    } else {
        mix(LED_ON, RED, (t - 0.85) / 0.15)
    }
}

fn slider_track(img: &mut Image, x: i32, y: i32, w: i32) {
    recess(img, x, y, w, 13, rgb(0x16, 0x19, 0x1e), EDGE, METAL_HI);
    rect(img, x + 3, y + 5, w - 6, 3, rgb(0x0c, 0x0e, 0x11));
}

fn volume_bmp() -> Image {
    let mut img = Image::new(68, 433, METAL_BOT);
    for n in 0..28 {
        let y = n * 15;
        slider_track(&mut img, 0, y, 68);
        let fill = (62.0 * n as f32 / 27.0).round() as i32;
        for i in 0..fill {
            let c = level_color(i as f32 / 61.0);
            vline(&mut img, 3 + i, y + 5, 3, c);
        }
        rect(&mut img, 0, y + 13, 68, 2, METAL_BOT);
    }
    slider_thumb(&mut img, 0, 422, 14, 11, true);
    slider_thumb(&mut img, 15, 422, 14, 11, false);
    img
}

fn balance_bmp() -> Image {
    let mut img = Image::new(68, 433, METAL_BOT);
    for n in 0..28 {
        let y = n * 15;
        slider_track(&mut img, 9, y, 38);
        let t = n as f32 / 27.0;
        let c = mix(rgb(0x2e, 0xb8, 0x7a), LED_ON, t);
        rect(&mut img, 12, y + 5, 32, 3, mix(rgb(0x0c, 0x0e, 0x11), c, 0.35 + 0.65 * t));
        // Repère central.
        vline(&mut img, 27, y + 4, 5, WOOL_DIM);
        vline(&mut img, 28, y + 4, 5, WOOL_DIM);
        rect(&mut img, 0, y + 13, 68, 2, METAL_BOT);
    }
    slider_thumb(&mut img, 0, 422, 14, 11, true);
    slider_thumb(&mut img, 15, 422, 14, 11, false);
    img
}

// --- monoster.bmp / playpaus.bmp ---------------------------------------------------------------------

fn monoster_bmp() -> Image {
    let mut img = Image::new(58, 24, LCD_BG);
    for (row, on) in [(0, true), (12, false)] {
        let c = if on { LCD_FG } else { LCD_GHOST };
        rect(&mut img, 0, row, 58, 12, LCD_BG);
        let sw = tiny_width("STEREO");
        tiny_text(&mut img, (29 - sw) / 2, row + 4, "STEREO", c);
        let mw = tiny_width("MONO");
        tiny_text(&mut img, 29 + (27 - mw) / 2, row + 4, "MONO", c);
    }
    img
}

fn playpaus_bmp() -> Image {
    let mut img = Image::new(42, 9, LCD_BG);
    glyph(&mut img, 2, 1, &["#....", "##...", "###..", "####.", "#####", "####.", "###..", "##...", "#...."][1..8], LCD_FG);
    glyph(&mut img, 11, 1, &["##.##", "##.##", "##.##", "##.##", "##.##", "##.##", "##.##"], LED_ON);
    glyph(&mut img, 20, 2, &["#####", "#####", "#####", "#####", "#####"], RED);
    // 27..28 : rien ; 36 : en synchro ; 39 : en mémoire tampon.
    rect(&mut img, 36, 0, 3, 9, LCD_BG);
    vline(&mut img, 37, 2, 5, LCD_FG);
    rect(&mut img, 39, 0, 3, 9, LCD_BG);
    vline(&mut img, 40, 2, 5, RED);
    img
}

// --- eqmain.bmp --------------------------------------------------------------------------------------

/// Libellés des bandes (ceux de Winamp ; les filtres réels sont dans le moteur audio).
const EQ_LABELS: [&str; 10] = ["60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K"];

fn eqmain_bmp() -> Image {
    let mut img = Image::new(275, 315, METAL_BOT);
    brushed(&mut img, 0, 116, 275, 199, METAL_TOP, METAL_BOT, 9, 3.0);
    // Fond de la fenêtre.
    body(&mut img, 0, 0, 275, 116, 5);
    // Panneau des curseurs.
    recess(&mut img, 15, 35, 246, 76, rgb(0x22, 0x26, 0x2c), EDGE, METAL_HI);
    for (i, label) in EQ_LABELS.iter().enumerate() {
        let x = 78 + 18 * i as i32;
        slider_slot(&mut img, x, 38);
        let w = tiny_width(label);
        engraved(&mut img, x + 7 - w / 2, 103, label);
    }
    slider_slot(&mut img, 21, 38);
    engraved(&mut img, 18, 103, "PRE");
    // Échelle en dB.
    engraved(&mut img, 42, 39, "+12");
    engraved(&mut img, 50, 67, "0");
    engraved(&mut img, 42, 95, "-12");
    for y in [41, 69, 97] {
        hline(&mut img, 56, y, 14, rgb(0x5a, 0x61, 0x6c));
    }
    // Logement du graphe.
    lcd(&mut img, 84, 15, 117, 23);

    // Bouton de fermeture (0,116) et enfoncé (0,125).
    win_button(&mut img, 0, 116, ICON_CLOSE, false);
    win_button(&mut img, 0, 125, ICON_CLOSE, true);
    // ON / AUTO : éteint, allumé, éteint enfoncé, allumé enfoncé.
    for (i, (on, pressed)) in [(false, false), (true, false), (false, true), (true, true)].into_iter().enumerate() {
        let base = 10 + 59 * i as i32;
        toggle_key(&mut img, base, 119, 26, "ON", on, pressed);
        toggle_key(&mut img, base + 26, 119, 32, "AUTO", on, pressed);
    }
    // Bandes de titre.
    let buttons: [(i32, &[&str]); 2] = [(254, ICON_SHADE), (264, ICON_CLOSE)];
    title_strip(&mut img, 0, 134, 275, "EQUALIZER", true, &buttons);
    title_strip(&mut img, 0, 149, 275, "EQUALIZER", false, &buttons);
    // Curseur (normal, enfoncé).
    slider_thumb(&mut img, 0, 164, 11, 11, false);
    slider_thumb(&mut img, 0, 176, 11, 11, true);
    // 28 fonds de curseur.
    for n in 0..28 {
        let (x, y) = if n < 14 { (13 + 15 * n, 164) } else { (13 + 15 * (n - 14), 229) };
        eq_slider_frame(&mut img, x, y, n);
    }
    // PRESETS.
    for (y, pressed) in [(164, false), (176, true)] {
        button(&mut img, 224, y, 44, 12, pressed);
        let o = pressed as i32;
        tiny_text(&mut img, 224 + (44 - tiny_width("PRESETS")) / 2 + o, y + 4 + o, "PRESETS", if pressed { WOOL_DIM } else { WOOL });
    }
    // Graphe : fond, couleurs de la courbe, ligne du préampli.
    rect(&mut img, 0, 294, 113, 19, LCD_BG);
    for gx in (0..113).step_by(8) {
        for gy in (1..19).step_by(3) {
            px(&mut img, gx, 294 + gy, rgb(0x10, 0x24, 0x1a));
        }
    }
    hline(&mut img, 0, 294 + 9, 113, LCD_GHOST);
    for r in 0..19 {
        let t = (r as f32 - 9.0).abs() / 9.0;
        px(&mut img, 115, 294 + r, mix(LCD_FG, LED_ON, t));
    }
    hline(&mut img, 0, 314, 113, WOOL_DIM);
    img
}

/// Logement d'un curseur d'égaliseur (14x63) gravé dans le fond.
fn slider_slot(img: &mut Image, x: i32, y: i32) {
    recess(img, x, y, 14, 63, rgb(0x12, 0x15, 0x19), EDGE, METAL_HI);
    vline(img, x + 6, y + 3, 57, EDGE);
    vline(img, x + 7, y + 3, 57, rgb(0x2a, 0x2f, 0x36));
}

/// Fond de curseur d'égaliseur n (0 = vide, 27 = plein jusqu'en haut).
fn eq_slider_frame(img: &mut Image, x: i32, y: i32, n: i32) {
    slider_slot(img, x, y);
    let fill = (57.0 * n as f32 / 27.0).round() as i32;
    for i in 0..fill {
        let row = y + 59 - i;
        let c = level_color(i as f32 / 56.0);
        vline(img, x + 5, row, 1, mix(EDGE, c, 0.5));
        hline(img, x + 6, row, 2, c);
    }
}

fn toggle_key(img: &mut Image, x: i32, y: i32, w: i32, label: &str, on: bool, pressed: bool) {
    button(img, x, y, w, 12, pressed);
    let o = pressed as i32;
    led(img, x + 4 + o, y + 5 + o, on);
    tiny_text(img, x + 10 + o, y + 4 + o, label, if on { WOOL } else { WOOL_DIM });
}

// --- eq_ex.bmp -----------------------------------------------------------------------------------------

fn eq_ex_bmp() -> Image {
    let mut img = Image::new(275, 56, METAL_BOT);
    brushed(&mut img, 0, 0, 275, 56, METAL_TOP, METAL_BOT, 11, 3.0);
    for (y, active) in [(0, true), (15, false)] {
        let (top, bot) = if active { (rgb(0x5a, 0x61, 0x6d), rgb(0x33, 0x38, 0x40)) } else { (rgb(0x3e, 0x43, 0x4b), rgb(0x2a, 0x2e, 0x34)) };
        brushed(&mut img, 0, y, 275, 14, top, bot, 29, 4.0);
        frame(&mut img, 0, y, 275, 14, EDGE);
        small_text(&mut img, 6, y + 4, "EQ", if active { WOOL } else { WOOL_DIM });
        // Glissières de volume et de balance.
        rect(&mut img, 61, y + 4, 97, 7, rgb(0x16, 0x19, 0x1e));
        frame(&mut img, 60, y + 3, 99, 9, EDGE);
        hline(&mut img, 62, y + 7, 95, level_color(0.3));
        rect(&mut img, 164, y + 4, 43, 7, rgb(0x16, 0x19, 0x1e));
        frame(&mut img, 163, y + 3, 45, 9, EDGE);
        hline(&mut img, 165, y + 7, 41, LED_ON);
        engraved(&mut img, 30, y + 5, "VOL");
        engraved(&mut img, 214, y + 5, "BAL");
        win_button(&mut img, 254, y + 3, ICON_UNSHADE, false);
        win_button(&mut img, 264, y + 3, ICON_CLOSE, false);
    }
    for (i, x) in [1, 4, 7].into_iter().enumerate() {
        rect(&mut img, x, 30, 3, 7, WOOL);
        frame(&mut img, x, 30, 3, 7, EDGE);
        let _ = i;
    }
    for x in [11, 14, 17] {
        rect(&mut img, x, 30, 3, 7, LED_ON);
        frame(&mut img, x, 30, 3, 7, EDGE);
    }
    win_button(&mut img, 1, 38, ICON_UNSHADE, true);
    win_button(&mut img, 1, 47, ICON_SHADE, true);
    win_button(&mut img, 11, 38, ICON_CLOSE, false);
    win_button(&mut img, 11, 47, ICON_CLOSE, true);
    img
}

// --- pledit.bmp ----------------------------------------------------------------------------------------

fn pledit_bmp() -> Image {
    let mut img = Image::new(280, 186, METAL_BOT);
    brushed(&mut img, 0, 0, 280, 186, METAL_TOP, METAL_BOT, 13, 3.0);

    for (y, active) in [(0, true), (21, false)] {
        let (top, bot) = if active { (rgb(0x5a, 0x61, 0x6d), rgb(0x33, 0x38, 0x40)) } else { (rgb(0x3e, 0x43, 0x4b), rgb(0x2a, 0x2e, 0x34)) };
        let hi = if active { rgb(0x8e, 0x96, 0xa3) } else { rgb(0x5e, 0x64, 0x6e) };
        // Coin gauche, titre, tuile, coin droit.
        for (x, w) in [(0, 25), (26, 100), (127, 25), (153, 25)] {
            brushed(&mut img, x, y, w, 20, top, bot, 31, 4.0);
            hline(&mut img, x, y, w, EDGE);
            hline(&mut img, x, y + 1, w, hi);
            hline(&mut img, x, y + 19, w, EDGE);
        }
        vline(&mut img, 0, y, 20, EDGE);
        vline(&mut img, 177, y, 20, EDGE);
        // Rainures sur les tuiles et les coins.
        for gy in [7, 10] {
            for (x, w) in [(4, 21), (127, 25), (153, 0)] {
                if w > 0 {
                    hline(&mut img, x, y + gy, w, hi);
                    hline(&mut img, x, y + gy + 1, w, EDGE);
                }
            }
        }
        let label = "PLAYLIST";
        let tw = small_text_width(label);
        let tx = 26 + (100 - tw) / 2;
        small_text(&mut img, tx + 1, y + 8, label, EDGE);
        small_text(&mut img, tx, y + 7, label, if active { WOOL } else { WOOL_DIM });
        for gy in [7, 10] {
            hline(&mut img, 30, y + gy, tx - 34, hi);
            hline(&mut img, 30, y + gy + 1, tx - 34, EDGE);
            hline(&mut img, tx + tw + 4, y + gy, 122 - (tx + tw + 4), hi);
            hline(&mut img, tx + tw + 4, y + gy + 1, 122 - (tx + tw + 4), EDGE);
        }
        // Boutons (fermer à droite - 11, bandeau à droite - 21) dans le coin droit.
        win_button(&mut img, 153 + 4, y + 3, ICON_SHADE, false);
        win_button(&mut img, 153 + 14, y + 3, ICON_CLOSE, false);
    }

    // Bas : tuile (179,0) et visualiseur (205,0).
    bottom_tile(&mut img, 179, 0, 25);
    bottom_tile(&mut img, 205, 0, 75);
    lcd(&mut img, 205 + 1, 11, 74, 22);

    // Bords latéraux.
    side_tile(&mut img, 0, 42, 12, false);
    side_tile(&mut img, 31, 42, 20, true);
    // Boutons enfoncés et poignée de défilement.
    win_button(&mut img, 52, 42, ICON_CLOSE, true);
    win_button(&mut img, 62, 42, ICON_SHADE, true);
    slider_thumb(&mut img, 52, 53, 8, 18, false);
    slider_thumb(&mut img, 61, 53, 8, 18, true);

    // Mode bandeau.
    for (y, active) in [(42, true), (57, false)] {
        let (top, bot) = if active { (rgb(0x5a, 0x61, 0x6d), rgb(0x33, 0x38, 0x40)) } else { (rgb(0x3e, 0x43, 0x4b), rgb(0x2a, 0x2e, 0x34)) };
        brushed(&mut img, 99, y, 50, 14, top, bot, 37, 4.0);
        frame(&mut img, 99, y, 50, 14, EDGE);
        win_button(&mut img, 99 + 30, y + 3, ICON_UNSHADE, false);
        win_button(&mut img, 99 + 40, y + 3, ICON_CLOSE, false);
    }
    brushed(&mut img, 72, 42, 25, 14, rgb(0x5a, 0x61, 0x6d), rgb(0x33, 0x38, 0x40), 41, 4.0);
    frame(&mut img, 72, 42, 26, 14, EDGE);
    brushed(&mut img, 72, 57, 25, 14, rgb(0x4a, 0x50, 0x5a), rgb(0x30, 0x34, 0x3b), 43, 4.0);
    hline(&mut img, 72, 57, 25, EDGE);
    hline(&mut img, 72, 70, 25, EDGE);
    win_button(&mut img, 150, 42, ICON_UNSHADE, true);

    // Bas gauche (0,72) et bas droit (126,72).
    bottom_tile(&mut img, 0, 72, 125);
    bottom_tile(&mut img, 126, 72, 150);
    for (x, label) in [(14, "ADD"), (43, "REM"), (72, "SEL"), (101, "MISC")] {
        button(&mut img, x, 72 + 8, 22, 18, false);
        tiny_text(&mut img, x + (22 - tiny_width(label)) / 2, 72 + 15, label, WOOL);
    }
    {
        let x = 126 + 106;
        button(&mut img, x, 72 + 8, 22, 18, false);
        tiny_text(&mut img, x + (22 - tiny_width("LIST")) / 2, 72 + 15, "LIST", WOOL);
    }
    // Afficheur du temps total et mini transport.
    lcd(&mut img, 126 + 5, 72 + 8, 92, 12);
    let icons: [&[&str]; 6] = [
        &["#..#", "#.##", "####", "#.##", "#..#"],
        &["#..", "##.", "###", "##.", "#.."],
        &["#.#", "#.#", "#.#", "#.#", "#.#"],
        &["###", "###", "###", "###"],
        &["#..#", "##.#", "####", "##.#", "#..#"],
        &["..#..", ".###.", "#####", ".....", "#####"],
    ];
    for (i, icon) in icons.iter().enumerate() {
        let x = 126 + 3 + 10 * i as i32;
        glyph(&mut img, x + 3, 72 + 24, icon, WOOL_DIM);
    }
    lcd(&mut img, 126 + 64, 72 + 21, 30, 9);
    // Poignée de redimensionnement.
    for k in 0..4 {
        let o = 3 * k;
        hline(&mut img, 126 + 150 - 4 - o, 72 + 38 - 4, 2 + o, METAL_HI);
        vline(&mut img, 126 + 150 - 4, 72 + 38 - 6 - o, 2, METAL_HI);
    }
    // Flèches de défilement.
    glyph(&mut img, 126 + 136, 72 + 2, &["..#..", ".###.", "#####"], WOOL_DIM);
    glyph(&mut img, 126 + 136, 72 + 9, &["#####", ".###.", "..#.."], WOOL_DIM);

    // Menus en bande (22x18) : normal en x, enfoncé en x + 23.
    let menus: [(i32, &[&str]); 5] = [
        (0, &["URL", "DIR", "FILE"]),
        (54, &["ALL", "CROP", "SEL", "MISC"]),
        (104, &["INV", "NONE", "ALL"]),
        (154, &["SORT", "INFO", "OPTS"]),
        (204, &["NEW", "SAVE", "LOAD"]),
    ];
    for (x, labels) in menus {
        for (i, label) in labels.iter().enumerate() {
            let y = 111 + 19 * i as i32;
            for (dx, pressed) in [(0, false), (23, true)] {
                button(&mut img, x + dx, y, 22, 18, pressed);
                let lw = tiny_width(label);
                let o = pressed as i32;
                tiny_text(&mut img, x + dx + (22 - lw) / 2 + o, y + 7 + o, label, if pressed { LED_ON } else { WOOL });
            }
        }
    }
    for (x, h) in [(48, 54), (100, 72), (150, 54), (200, 54), (250, 54)] {
        rect(&mut img, x, 111, 3, h, WOOL_DARK);
        vline(&mut img, x, 111, h, WOOL);
    }
    img
}

fn bottom_tile(img: &mut Image, x: i32, y: i32, w: i32) {
    brushed(img, x, y, w, 38, METAL_TOP, METAL_BOT, 47, 4.0);
    hline(img, x, y, w, METAL_HI);
    hline(img, x, y + 37, w, EDGE);
}

fn side_tile(img: &mut Image, x: i32, y: i32, w: i32, scroll: bool) {
    brushed(img, x, y, w, 29, METAL_TOP, METAL_BOT, 53, 3.0);
    if scroll {
        vline(img, x + w - 1, y, 29, EDGE);
        rect(img, x + 5, y, 10, 29, rgb(0x16, 0x19, 0x1e));
        vline(img, x + 5, y, 29, EDGE);
        vline(img, x + 14, y, 29, METAL_HI);
    } else {
        vline(img, x, y, 29, EDGE);
        vline(img, x + 1, y, 29, METAL_HI);
        vline(img, x + w - 1, y, 29, EDGE);
    }
}

// --- gen.bmp / genex.bmp -------------------------------------------------------------------------------

fn gen_bmp() -> Image {
    let mut img = Image::new(194, 109, METAL_BOT);
    brushed(&mut img, 0, 0, 194, 109, METAL_TOP, METAL_BOT, 59, 3.0);
    for (y, active) in [(0, true), (21, false)] {
        let (top, bot) = if active { (rgb(0x5a, 0x61, 0x6d), rgb(0x33, 0x38, 0x40)) } else { (rgb(0x3e, 0x43, 0x4b), rgb(0x2a, 0x2e, 0x34)) };
        let hi = if active { rgb(0x8e, 0x96, 0xa3) } else { rgb(0x5e, 0x64, 0x6e) };
        for x in [0, 26, 52, 78, 104, 130] {
            brushed(&mut img, x, y, 25, 20, top, bot, 61, 4.0);
            hline(&mut img, x, y, 25, EDGE);
            hline(&mut img, x, y + 19, 25, EDGE);
            for gy in [7, 10] {
                hline(&mut img, x, y + gy, 25, hi);
                hline(&mut img, x, y + gy + 1, 25, EDGE);
            }
        }
        vline(&mut img, 0, y, 20, EDGE);
        vline(&mut img, 154, y, 20, EDGE);
        win_button(&mut img, 130 + 14, y + 3, ICON_CLOSE, false);
    }
    // Bas gauche / bas droit / remplissage.
    for (x, y, w) in [(0, 42, 125), (0, 57, 125), (127, 72, 25)] {
        brushed(&mut img, x, y, w, 14, METAL_TOP, METAL_BOT, 67, 3.0);
        hline(&mut img, x, y + 13, w, EDGE);
    }
    vline(&mut img, 0, 42, 14, EDGE);
    vline(&mut img, 124, 57, 14, EDGE);
    for (x, y, w, h) in [(127, 42, 11, 29), (158, 42, 11, 24)] {
        brushed(&mut img, x, y, w, h, METAL_TOP, METAL_BOT, 71, 3.0);
        vline(&mut img, x, y, h, EDGE);
        vline(&mut img, x + 1, y, h, METAL_HI);
    }
    for (x, y, w, h) in [(139, 42, 8, 29), (170, 42, 8, 24)] {
        brushed(&mut img, x, y, w, h, METAL_TOP, METAL_BOT, 73, 3.0);
        vline(&mut img, x + w - 1, y, h, EDGE);
    }
    win_button(&mut img, 148, 42, ICON_CLOSE, true);
    // Police du titre (lignes 88 actif / 96 inactif) : lettres A–Z à largeur variable, séparées par la
    // couleur de fond prise en (0, ligne).
    for (y, color) in [(88, WOOL), (96, WOOL_DIM)] {
        let bg = METAL_BOT;
        rect(&mut img, 0, y, 194, 7, bg);
        let mut x = 1;
        for ch in 'A'..='Z' {
            let g = small(ch);
            glyph(&mut img, x, y + 1, g, color);
            x += small_glyph_width(ch) + 1;
        }
    }
    img
}

fn genex_bmp() -> Image {
    let mut img = Image::new(112, 59, METAL_BOT);
    brushed(&mut img, 0, 0, 112, 59, METAL_TOP, METAL_BOT, 79, 3.0);
    // Bouton (0,0) normal et (0,15) enfoncé, 47x15.
    button(&mut img, 0, 0, 47, 15, false);
    button(&mut img, 0, 15, 47, 15, true);
    // Flèches de défilement 14x14 et poignées.
    let up: &[&str] = &["..#..", ".###.", "#####"];
    let down: &[&str] = &["#####", ".###.", "..#.."];
    for (x, icon, pressed) in [(0, up, false), (14, down, false), (28, up, true), (42, down, true)] {
        button(&mut img, x, 31, 14, 14, pressed);
        glyph(&mut img, x + 5, 31 + 6, icon, WOOL);
    }
    let left: &[&str] = &["..#", ".##", "###", ".##", "..#"];
    let right: &[&str] = &["#..", "##.", "###", "##.", "#.."];
    for (x, icon, pressed) in [(0, left, false), (14, right, false), (28, left, true), (42, right, true)] {
        button(&mut img, x, 45, 14, 14, pressed);
        glyph(&mut img, x + 6, 45 + 5, icon, WOOL);
    }
    slider_thumb(&mut img, 56, 31, 14, 28, false);
    slider_thumb(&mut img, 70, 31, 14, 28, true);
    slider_thumb(&mut img, 84, 31, 28, 14, false);
    slider_thumb(&mut img, 84, 45, 28, 14, true);
    // Couleurs sur la ligne 0 (x = 48, 50, … 90) : fond de liste, texte, etc.
    let colors = [
        LCD_BG,              // fond des éléments
        LCD_FG,              // texte des éléments
        WOOL,                // texte de fenêtre
        rgb(0x2a, 0x2e, 0x35), // couleur de séparation
        LCD_BG,              // fond d'éditeur
        LCD_HI,              // texte d'éditeur
        rgb(0x1f, 0x4a, 0x38), // sélection
        LCD_HI,              // texte sélectionné
        rgb(0x1f, 0x4a, 0x38), // sélection inactive
        LCD_FG,              // texte sélectionné inactif
        METAL_TOP,           // fond de bouton
        WOOL,                // texte de bouton
        METAL_HI,            // clair 3D
        METAL_LO,            // sombre 3D
        LCD_FG,              // texte de liste
        WOOL_DIM,            // texte désactivé
        LED_ON,              // lien
        LCD_FG,              // titre
        LCD_BG,
        LCD_BG,
        LCD_BG,
        LCD_BG,
    ];
    for (i, c) in colors.iter().enumerate() {
        px(&mut img, 48 + 2 * i as i32, 0, *c);
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_sheets_have_reference_size() {
        let skin = build();
        for sheet in Sheet::ALL {
            let img = skin.sheet(sheet);
            let (w, h) = sheet.reference_size();
            assert!(img.width >= w && img.height >= h, "{sheet:?} {}x{} < {w}x{h}", img.width, img.height);
        }
    }

    #[test]
    fn minus_sign_from_numbers_is_middle_bar_of_two() {
        let skin = build();
        let n = skin.sheet(Sheet::Numbers);
        for x in 20..25 {
            assert_eq!(n.get(x, 6), Some(LCD_FG), "pixel {x}");
        }
    }
}
