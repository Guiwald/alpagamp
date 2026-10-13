//! **Minuterie de sommeil** : au bout de N minutes, le son baisse doucement puis la musique s'arrête (ADR 0013).

use alpagamp_plugin::host::{self, Command, State, tr};
use alpagamp_plugin::{General, Plugin};

const INFO: &str = r#"{
  "name": {"en": "Sleep Timer", "fr": "Minuterie de sommeil", "es": "Temporizador de apagado", "de": "Schlummer-Timer", "it": "Timer di spegnimento", "ca": "Temporitzador per a dormir", "ga": "Lasc ama codlata"},
  "kind": "general",
  "author": "Alpagamp",
  "version": "1.0.0",
  "description": {
    "en": "Fall asleep to your music: after the time you choose, the volume fades out and playback stops. Turn it off and on again to restart the countdown.",
    "fr": "Pour s'endormir en musique : au bout du temps choisi, le son baisse doucement et la lecture s'arrête. Désactive-la puis réactive-la pour relancer le compte à rebours.",
    "es": "Para dormirte con música: tras el tiempo elegido, el volumen baja poco a poco y la reproducción se detiene. Desactívalo y vuelve a activarlo para reiniciar la cuenta atrás.",
    "de": "Mit Musik einschlafen: Nach der gewählten Zeit wird es langsam leiser und die Wiedergabe stoppt. Aus- und wieder einschalten startet den Countdown neu.",
    "it": "Per addormentarsi con la musica: dopo il tempo scelto il volume si abbassa piano e la riproduzione si ferma. Disattivalo e riattivalo per ricominciare il conto alla rovescia.",
    "ca": "Per a dormir-se amb música: passat el temps triat, el volum baixa a poc a poc i la reproducció s'atura. Desactiva'l i torna'l a activar per a reiniciar el compte arrere.",
    "ga": "Titim i do chodladh le ceol: tar éis an ama a roghnaíonn tú, ísleoidh an fhuaim go mall agus stopfaidh an seinm. Múch agus las arís é chun an t-áireamh a atosú."
  },
  "options": [
    {"id": "minutes", "label": {"en": "Stop after", "fr": "Arrêt au bout de", "es": "Parar tras", "de": "Stopp nach", "it": "Ferma dopo", "ca": "Atura després de", "ga": "Stop tar éis"},
     "values": [{"value": 5, "label": "5 min"}, {"value": 15, "label": "15 min"}, {"value": 30, "label": "30 min"}, {"value": 45, "label": "45 min"}, {"value": 60, "label": "1 h"}, {"value": 90, "label": "1 h 30"}],
     "default": 30},
    {"id": "fade", "label": {"en": "Fade out", "fr": "Fondu", "es": "Fundido", "de": "Ausblenden", "it": "Dissolvenza", "ca": "Fosa", "ga": "Céimniú"},
     "values": [
       {"value": 0, "label": {"en": "None", "fr": "Aucun", "es": "Ninguno", "de": "Keins", "it": "Nessuna", "ca": "Cap", "ga": "Dada"}},
       {"value": 30, "label": "30 s"}, {"value": 120, "label": "2 min"}, {"value": 300, "label": "5 min"}],
     "default": 120},
    {"id": "end", "label": {"en": "At the end", "fr": "À la fin", "es": "Al final", "de": "Am Ende", "it": "Alla fine", "ca": "Al final", "ga": "Ag an deireadh"},
     "values": [
       {"value": 0, "label": {"en": "Stop", "fr": "Arrêter", "es": "Detener", "de": "Stoppen", "it": "Ferma", "ca": "Atura", "ga": "Stop"}},
       {"value": 1, "label": {"en": "Pause", "fr": "Mettre en pause", "es": "Pausar", "de": "Pausieren", "it": "Pausa", "ca": "Pausa", "ga": "Sos"}}],
     "default": 0}
  ]
}"#;

pub struct Minuterie {
    minutes: i64,
    fade_s: i64,
    pause: bool,
    /// Début du compte à rebours (temps des `tick`).
    start: Option<i64>,
    done: bool,
    /// Volume d'avant le fondu, et dernier volume posé par le fondu.
    fade: Option<(f32, u8)>,
    warned: u8,
}

fn minutes_text(m: i64) -> String {
    let t = tr(["Sleep timer: music stops in {} min", "Minuterie : arrêt dans {} min", "Temporizador: parada en {} min", "Timer: Stopp in {} Min.", "Timer: stop tra {} min", "Temporitzador: s'atura en {} min", "Lasc ama: stopfaidh sé i gceann {} nóim."]);
    t.replace("{}", &m.to_string())
}

impl Plugin for Minuterie {
    const INFO: &'static str = INFO;

    fn new() -> Self {
        Minuterie { minutes: 30, fade_s: 120, pause: false, start: None, done: false, fade: None, warned: 0 }
    }

    fn option(&mut self, index: u32, value: i32) {
        match index {
            0 => self.minutes = value.clamp(1, 600) as i64,
            1 => self.fade_s = value.clamp(0, 1800) as i64,
            2 => self.pause = value == 1,
            _ => return,
        }
        // Nouveau réglage : le compte à rebours repart.
        self.restore();
        self.start = None;
        self.done = false;
        self.warned = 0;
    }

    fn quit(&mut self) {
        self.restore();
    }
}

impl Minuterie {
    /// Remet le volume d'avant le fondu.
    fn restore(&mut self) {
        if let Some((base, _)) = self.fade.take() {
            host::set_volume(base.round().clamp(0.0, 255.0) as u8);
        }
    }

    /// Temps restant (ms) au temps `now`.
    pub fn remaining(&self, now: i64) -> i64 {
        match self.start {
            Some(s) => self.minutes * 60_000 - (now - s),
            None => self.minutes * 60_000,
        }
    }
}

impl General for Minuterie {
    fn tick(&mut self, now: i64) {
        if self.done {
            return;
        }
        if self.start.is_none() {
            self.start = Some(now);
            host::notify(&minutes_text(self.minutes));
        }
        let left = self.remaining(now);
        // Rappels à 5 et 1 minute(s).
        for (i, m) in [5i64, 1].into_iter().enumerate() {
            let bit = 1 << i;
            if self.warned & bit == 0 && left <= m * 60_000 && self.minutes > m {
                self.warned |= bit;
                host::notify(&minutes_text(m));
            }
        }
        if left <= 0 {
            host::command(if self.pause { Command::Pause } else { Command::Stop });
            self.restore();
            self.done = true;
            host::notify(tr(["Good night!", "Bonne nuit !", "¡Buenas noches!", "Gute Nacht!", "Buona notte!", "Bona nit!", "Oíche mhaith!"]));
            return;
        }
        let fade_ms = self.fade_s.min(self.minutes * 60) * 1000;
        if fade_ms > 0 && left <= fade_ms && host::state() == State::Playing {
            let k = left as f32 / fade_ms as f32;
            let current = host::volume();
            let base = match self.fade {
                // Quelqu'un a touché au volume pendant le fondu : on part de son réglage.
                Some((_, set)) if set.abs_diff(current) > 1 => (current as f32 / k.max(0.05)).min(255.0),
                Some((base, _)) => base,
                None => current as f32,
            };
            let v = (base * k).round().clamp(0.0, 255.0) as u8;
            host::set_volume(v);
            self.fade = Some((base, v));
        }
    }
}

alpagamp_plugin::general_plugin!(Minuterie);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fades_then_stops_and_restores_volume() {
        let mut m = Minuterie::new();
        m.option(0, 5);
        m.option(1, 120);
        host::set_volume(200);
        host::command(Command::Play);
        let mut t = 0;
        while t <= 5 * 60_000 {
            m.tick(t);
            if t == 3 * 60_000 {
                assert!((198..=200).contains(&host::volume()), "début du fondu : {}", host::volume());
            }
            if t == 4 * 60_000 {
                let v = host::volume();
                assert!((95..=105).contains(&v), "milieu du fondu : {v}");
            }
            if t == 5 * 60_000 - 100 {
                assert!(host::volume() < 3, "fondu presque fini : {}", host::volume());
            }
            t += 100;
        }
        assert!(m.done);
        assert_eq!(host::state(), State::Stopped);
        assert_eq!(host::volume(), 200, "volume rendu");
    }

    #[test]
    fn option_change_restarts_the_countdown() {
        let mut m = Minuterie::new();
        m.tick(0);
        m.tick(10 * 60_000);
        assert_eq!(m.remaining(10 * 60_000), 20 * 60_000);
        m.option(0, 15);
        m.tick(10 * 60_000);
        assert_eq!(m.remaining(10 * 60_000), 15 * 60_000);
    }
}
