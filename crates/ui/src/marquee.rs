//! Titre défilant de la fenêtre principale.
//!
//! Le texte défile de 5 px (un caractère) toutes les 200 ms quand il dépasse 154 px, sous la forme
//! « texte  ***  texte ». Pendant un réglage (volume, position…), un message d'état le remplace.

use std::time::{Duration, Instant};

use crate::paint::text_width;

const AREA: i32 = 154;
const STEP_MS: u64 = 200;
const SEPARATOR: &str = "  ***  ";

pub struct Marquee {
    offset: i32,
    last_step: Instant,
    /// Défilement suspendu jusqu'à cet instant (après un glissement manuel).
    paused_until: Option<Instant>,
    status: Option<String>,
    /// Message bref (bascule aléatoire/répétition…).
    flash: Option<(String, Instant)>,
    dragging: bool,
    last_text: String,
}

impl Default for Marquee {
    fn default() -> Self {
        Self::new()
    }
}

impl Marquee {
    pub fn new() -> Self {
        Self { offset: 0, last_step: Instant::now(), paused_until: None, status: None, flash: None, dragging: false, last_text: String::new() }
    }

    /// Texte réellement affiché (message d'état prioritaire).
    pub fn text(&mut self, base: &str) -> String {
        if let Some(s) = &self.status {
            return s.clone();
        }
        if let Some((s, until)) = &self.flash {
            if Instant::now() < *until {
                return s.clone();
            }
            self.flash = None;
        }
        if base != self.last_text {
            self.last_text = base.to_string();
            self.offset = 0;
        }
        if text_width(base) > AREA { format!("{base}{SEPARATOR}{base}") } else { base.to_string() }
    }

    /// Décalage à appliquer pour ce texte (0 s'il tient dans la zone).
    pub fn offset_for(&self, shown: &str) -> i32 {
        if self.status.is_some() || self.flash.is_some() {
            return 0;
        }
        if text_width(&self.last_text) > AREA && shown.len() > self.last_text.len() { self.offset } else { 0 }
    }

    pub fn offset(&self) -> i32 {
        self.offset
    }

    /// Avance le défilement selon l'horloge.
    pub fn tick(&mut self, now: Instant) {
        if self.dragging || self.paused_until.is_some_and(|t| now < t) {
            self.last_step = now;
            return;
        }
        self.paused_until = None;
        let loop_w = text_width(&self.last_text) + text_width(SEPARATOR);
        if text_width(&self.last_text) <= AREA {
            self.offset = 0;
            self.last_step = now;
            return;
        }
        while now.duration_since(self.last_step) >= Duration::from_millis(STEP_MS) {
            self.last_step += Duration::from_millis(STEP_MS);
            self.offset = (self.offset + 5) % loop_w.max(1);
        }
    }

    pub fn set_status(&mut self, s: String) {
        self.status = Some(s);
    }

    pub fn clear_status(&mut self) {
        self.status = None;
    }

    pub fn flash(&mut self, s: &str) {
        self.flash = Some((s.to_string(), Instant::now() + Duration::from_millis(1500)));
    }

    pub fn drag_to(&mut self, offset: i32) {
        self.dragging = true;
        let loop_w = (text_width(&self.last_text) + text_width(SEPARATOR)).max(1);
        if text_width(&self.last_text) > AREA {
            self.offset = offset.rem_euclid(loop_w);
        }
    }

    pub fn release(&mut self) {
        self.dragging = false;
        self.paused_until = Some(Instant::now() + Duration::from_millis(2000));
    }
}
