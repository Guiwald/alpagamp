//! Formatage des durées.

/// `m:ss`, ou `h:mm:ss` au-delà d'une heure.
pub fn format_duration(ms: u64) -> String {
    let s = ms / 1000;
    let (h, m, s) = (s / 3600, (s / 60) % 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

/// Minutes et secondes affichées par les chiffres de la fenêtre principale (minutes jusqu'à 999).
pub fn display_minutes_seconds(ms: u64) -> (u64, u64) {
    let s = ms / 1000;
    ((s / 60).min(999), s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(format_duration(0), "0:00");
        assert_eq!(format_duration(65_000), "1:05");
        assert_eq!(format_duration(3_725_000), "1:02:05");
        assert_eq!(display_minutes_seconds(6_000_000), (100, 0));
    }
}
