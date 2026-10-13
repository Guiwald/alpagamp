//! La playlist : entrées, sélection, tri, navigation (aléatoire, répétition, avance manuelle).

use std::collections::HashSet;

use crate::source::Source;

/// Une ligne de la playlist.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub source: Source,
    /// Titre affiché (« Artiste - Titre »), s'il est connu.
    pub title: Option<String>,
    /// Durée en millisecondes, si connue (`None` pour un flux ou une durée pas encore lue).
    pub duration_ms: Option<u64>,
    pub selected: bool,
    /// Les métadonnées ont-elles déjà été lues ?
    pub meta_loaded: bool,
}

impl Entry {
    pub fn new(source: Source) -> Self {
        Self { source, title: None, duration_ms: None, selected: false, meta_loaded: false }
    }

    pub fn with_meta(source: Source, title: Option<String>, duration_ms: Option<u64>) -> Self {
        let meta_loaded = title.is_some();
        Self { source, title, duration_ms, selected: false, meta_loaded }
    }

    /// Titre affiché, avec repli sur le nom du fichier.
    pub fn display_title(&self) -> String {
        self.title.clone().unwrap_or_else(|| self.source.fallback_title())
    }
}

/// Critères de tri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Title,
    FileName,
    Path,
}

/// Durée totale d'un ensemble d'entrées.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TotalTime {
    pub ms: u64,
    /// Certaines entrées ont une durée inconnue.
    pub partial: bool,
    /// Aucune entrée n'a de durée connue (alors que la liste n'est pas vide).
    pub unknown: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Playlist {
    entries: Vec<Entry>,
    /// Index du morceau courant (celui que l'on joue ou qu'on jouera).
    current: Option<usize>,
    /// Ordre aléatoire (permutation des index) et position courante dans cet ordre.
    shuffle_order: Vec<usize>,
    shuffle_pos: usize,
    /// Ancre des sélections au Shift-clic.
    anchor: Option<usize>,
    /// Modifiée depuis le dernier enregistrement.
    pub dirty: bool,
}

impl Playlist {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn entry_mut(&mut self, i: usize) -> Option<&mut Entry> {
        self.entries.get_mut(i)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn current(&self) -> Option<usize> {
        self.current
    }

    pub fn current_entry(&self) -> Option<&Entry> {
        self.current.and_then(|i| self.entries.get(i))
    }

    pub fn set_current(&mut self, i: Option<usize>) {
        self.current = i.filter(|&i| i < self.entries.len());
        if let Some(c) = self.current {
            if let Some(p) = self.shuffle_order.iter().position(|&x| x == c) {
                self.shuffle_pos = p;
            }
        }
    }

    // --- Modifications -------------------------------------------------------------------------

    /// Ajoute des entrées à la fin ; renvoie l'index de la première ajoutée.
    pub fn extend(&mut self, entries: impl IntoIterator<Item = Entry>) -> usize {
        let first = self.entries.len();
        self.entries.extend(entries);
        if self.current.is_none() && !self.entries.is_empty() {
            self.current = Some(first.min(self.entries.len() - 1));
        }
        self.changed();
        first
    }

    /// Insère des entrées à la position `at`.
    pub fn insert(&mut self, at: usize, entries: Vec<Entry>) {
        let at = at.min(self.entries.len());
        let n = entries.len();
        self.entries.splice(at..at, entries);
        if let Some(c) = self.current {
            if c >= at {
                self.current = Some(c + n);
            }
        } else if !self.entries.is_empty() {
            self.current = Some(at);
        }
        self.changed();
    }

    /// Vide la liste.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current = None;
        self.anchor = None;
        self.changed();
    }

    /// Retire les entrées pour lesquelles `remove` est vrai ; garde le morceau courant si possible.
    fn retain_by(&mut self, mut remove: impl FnMut(&Entry) -> bool) {
        let mut new_current = None;
        let mut kept = Vec::with_capacity(self.entries.len());
        for (i, e) in std::mem::take(&mut self.entries).into_iter().enumerate() {
            if remove(&e) {
                continue;
            }
            if Some(i) == self.current {
                new_current = Some(kept.len());
            }
            kept.push(e);
        }
        if new_current.is_none() && !kept.is_empty() {
            // Le courant a disparu : on garde la position la plus proche.
            new_current = self.current.map(|c| c.min(kept.len() - 1));
        }
        self.entries = kept;
        self.current = new_current;
        self.anchor = None;
        self.changed();
    }

    pub fn remove_selected(&mut self) {
        self.retain_by(|e| e.selected);
    }

    /// Garde seulement la sélection (« crop »).
    pub fn crop(&mut self) {
        self.retain_by(|e| !e.selected);
    }

    /// Retire les fichiers locaux introuvables.
    pub fn remove_missing(&mut self) {
        self.retain_by(|e| matches!(&e.source, Source::File(p) if !p.exists()));
    }

    /// Retire les entrées aux index donnés.
    pub fn remove_indices(&mut self, idx: &HashSet<usize>) {
        let mut i = 0;
        self.retain_by(|_| {
            let r = idx.contains(&i);
            i += 1;
            r
        });
    }

    // --- Sélection ------------------------------------------------------------------------------

    pub fn select_all(&mut self) {
        self.entries.iter_mut().for_each(|e| e.selected = true);
    }

    pub fn select_none(&mut self) {
        self.entries.iter_mut().for_each(|e| e.selected = false);
    }

    pub fn invert_selection(&mut self) {
        self.entries.iter_mut().for_each(|e| e.selected = !e.selected);
    }

    pub fn selected_indices(&self) -> Vec<usize> {
        self.entries.iter().enumerate().filter(|(_, e)| e.selected).map(|(i, _)| i).collect()
    }

    /// Clic sur une ligne : simple (sélection unique), Ctrl (bascule), Shift (plage depuis l'ancre).
    pub fn click(&mut self, i: usize, shift: bool, ctrl: bool) {
        if i >= self.entries.len() {
            return;
        }
        if shift {
            let a = self.anchor.unwrap_or(i);
            let (lo, hi) = (a.min(i), a.max(i));
            if !ctrl {
                self.select_none();
            }
            for e in &mut self.entries[lo..=hi] {
                e.selected = true;
            }
        } else if ctrl {
            self.entries[i].selected = !self.entries[i].selected;
            self.anchor = Some(i);
        } else {
            self.select_none();
            self.entries[i].selected = true;
            self.anchor = Some(i);
        }
    }

    /// Déplace la sélection de `delta` lignes (Alt+↑/↓, glisser). Renvoie vrai si quelque chose a bougé.
    pub fn move_selection(&mut self, delta: isize) -> bool {
        if delta == 0 || self.entries.is_empty() {
            return false;
        }
        let sel = self.selected_indices();
        if sel.is_empty() {
            return false;
        }
        let first = *sel.first().unwrap() as isize;
        let last = *sel.last().unwrap() as isize;
        let delta = delta.clamp(-first, self.entries.len() as isize - 1 - last);
        if delta == 0 {
            return false;
        }
        let current_source = self.current;
        let n = self.entries.len();
        let mut tagged: Vec<(Entry, bool)> = std::mem::take(&mut self.entries).into_iter().enumerate().map(|(i, e)| (e, Some(i) == current_source)).collect();
        // Déplacement pas à pas pour garder l'ordre relatif.
        let steps = delta.unsigned_abs();
        for _ in 0..steps {
            if delta > 0 {
                for i in (0..n - 1).rev() {
                    if tagged[i].0.selected && !tagged[i + 1].0.selected {
                        tagged.swap(i, i + 1);
                    }
                }
            } else {
                for i in 1..n {
                    if tagged[i].0.selected && !tagged[i - 1].0.selected {
                        tagged.swap(i, i - 1);
                    }
                }
            }
        }
        self.current = tagged.iter().position(|(_, c)| *c);
        self.entries = tagged.into_iter().map(|(e, _)| e).collect();
        self.changed();
        true
    }

    // --- Tri --------------------------------------------------------------------------------------

    fn reorder(&mut self, mut f: impl FnMut(&mut Vec<(Entry, bool)>)) {
        let cur = self.current;
        let mut tagged: Vec<(Entry, bool)> = std::mem::take(&mut self.entries).into_iter().enumerate().map(|(i, e)| (e, Some(i) == cur)).collect();
        f(&mut tagged);
        self.current = tagged.iter().position(|(_, c)| *c);
        self.entries = tagged.into_iter().map(|(e, _)| e).collect();
        self.changed();
    }

    /// Tri « naturel » insensible à la casse (comme Windows : « 2 » avant « 10 »).
    pub fn sort(&mut self, key: SortKey) {
        self.reorder(|v| {
            v.sort_by(|a, b| {
                let ka = sort_text(&a.0, key);
                let kb = sort_text(&b.0, key);
                natural_cmp(&ka, &kb)
            })
        });
    }

    pub fn reverse(&mut self) {
        self.reorder(|v| v.reverse());
    }

    pub fn randomize(&mut self) {
        self.reorder(|v| fastrand::shuffle(v));
    }

    // --- Temps ------------------------------------------------------------------------------------

    fn total_of<'a>(it: impl Iterator<Item = &'a Entry>) -> TotalTime {
        let mut t = TotalTime::default();
        let mut n = 0;
        let mut known = 0;
        for e in it {
            n += 1;
            match e.duration_ms {
                Some(d) => {
                    t.ms += d;
                    known += 1;
                }
                None => t.partial = true,
            }
        }
        t.unknown = n > 0 && known == 0;
        t
    }

    pub fn total_time(&self) -> TotalTime {
        Self::total_of(self.entries.iter())
    }

    pub fn selected_time(&self) -> TotalTime {
        Self::total_of(self.entries.iter().filter(|e| e.selected))
    }

    // --- Recherche (« Jump to file ») -------------------------------------------------------------

    /// Index des entrées dont le titre ou la source contiennent **tous** les mots de `query`
    /// (insensible à la casse).
    pub fn search(&self, query: &str) -> Vec<usize> {
        let terms: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                let hay = format!("{} {}", e.display_title(), e.source.to_playlist_string()).to_lowercase();
                terms.iter().all(|t| hay.contains(t.as_str()))
            })
            .map(|(i, _)| i)
            .collect()
    }

    // --- Navigation -------------------------------------------------------------------------------

    fn changed(&mut self) {
        self.dirty = true;
        self.rebuild_shuffle();
    }

    /// Refait l'ordre aléatoire en plaçant le morceau courant en tête.
    pub fn rebuild_shuffle(&mut self) {
        let n = self.entries.len();
        let mut order: Vec<usize> = (0..n).collect();
        fastrand::shuffle(&mut order);
        if let Some(c) = self.current {
            if let Some(p) = order.iter().position(|&x| x == c) {
                order.swap(0, p);
            }
        }
        self.shuffle_order = order;
        self.shuffle_pos = 0;
    }

    /// Index suivant pour le bouton « suivant ». `None` = rien (fin de liste sans répétition).
    pub fn next_index(&mut self, shuffle: bool, repeat: bool) -> Option<usize> {
        let n = self.entries.len();
        if n == 0 {
            return None;
        }
        if shuffle {
            if self.shuffle_order.len() != n {
                self.rebuild_shuffle();
            }
            if self.shuffle_pos + 1 < n {
                self.shuffle_pos += 1;
            } else if repeat {
                self.morph_shuffle();
                self.shuffle_pos = 0;
            } else {
                return None;
            }
            return Some(self.shuffle_order[self.shuffle_pos]);
        }
        match self.current {
            None => Some(0),
            Some(c) if c + 1 < n => Some(c + 1),
            Some(_) if repeat => Some(0),
            Some(_) => None,
        }
    }

    /// Index précédent pour le bouton « précédent » (toujours le morceau d'avant, jamais un redémarrage).
    pub fn prev_index(&mut self, shuffle: bool, repeat: bool) -> Option<usize> {
        let n = self.entries.len();
        if n == 0 {
            return None;
        }
        if shuffle {
            if self.shuffle_order.len() != n {
                self.rebuild_shuffle();
            }
            if self.shuffle_pos > 0 {
                self.shuffle_pos -= 1;
            } else if repeat {
                self.shuffle_pos = n - 1;
            } else {
                return None;
            }
            return Some(self.shuffle_order[self.shuffle_pos]);
        }
        match self.current {
            None => Some(0),
            Some(c) if c > 0 => Some(c - 1),
            Some(_) if repeat => Some(n - 1),
            Some(_) => None,
        }
    }

    /// Que jouer quand un morceau se termine tout seul ?
    pub fn auto_advance(&mut self, shuffle: bool, repeat: bool, manual_advance: bool) -> Option<usize> {
        if manual_advance {
            return if repeat { self.current } else { None };
        }
        self.next_index(shuffle, repeat)
    }

    /// Ce que `auto_advance` choisirait, sans rien modifier (pour préparer l'enchaînement sans blanc).
    /// `None` quand on ne peut pas le prévoir (fin de cycle aléatoire avec remélange).
    pub fn peek_auto_advance(&self, shuffle: bool, repeat: bool, manual_advance: bool) -> Option<usize> {
        let n = self.entries.len();
        if n == 0 {
            return None;
        }
        if manual_advance {
            return if repeat { self.current } else { None };
        }
        if shuffle {
            if self.shuffle_order.len() != n {
                return None;
            }
            return self.shuffle_order.get(self.shuffle_pos + 1).copied();
        }
        match self.current {
            None => Some(0),
            Some(c) if c + 1 < n => Some(c + 1),
            Some(_) if repeat => Some(0),
            Some(_) => None,
        }
    }

    /// Remélange partiel en fin de cycle (fenêtre glissante, taux 50 %) : évite de rejouer
    /// immédiatement les derniers morceaux entendus.
    fn morph_shuffle(&mut self) {
        let n = self.shuffle_order.len();
        if n < 3 {
            fastrand::shuffle(&mut self.shuffle_order);
            return;
        }
        let window = (n / 2).max(2);
        let last = self.shuffle_order[n - 1];
        for i in 0..n {
            let j = (i + fastrand::usize(0..window)).min(n - 1);
            self.shuffle_order.swap(i, j);
        }
        if self.shuffle_order[0] == last {
            self.shuffle_order.swap(0, n - 1);
        }
    }
}

fn sort_text(e: &Entry, key: SortKey) -> String {
    match key {
        SortKey::Title => e.display_title(),
        SortKey::FileName => e.source.file_name(),
        SortKey::Path => e.source.to_playlist_string(),
    }
    .to_lowercase()
}

/// Comparaison « naturelle » : les suites de chiffres sont comparées comme des nombres.
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(&c) = ai.peek().filter(|c| c.is_ascii_digit()) {
                    na.push(c);
                    ai.next();
                }
                let mut nb = String::new();
                while let Some(&c) = bi.peek().filter(|c| c.is_ascii_digit()) {
                    nb.push(c);
                    bi.next();
                }
                let ta = na.trim_start_matches('0');
                let tb = nb.trim_start_matches('0');
                let o = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(&y);
                }
                ai.next();
                bi.next();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn pl(n: usize) -> Playlist {
        let mut p = Playlist::new();
        p.extend((0..n).map(|i| Entry::with_meta(Source::File(PathBuf::from(format!("/m/{i}.mp3"))), Some(format!("Track {i}")), Some(1000 * (i as u64 + 1)))));
        p
    }

    #[test]
    fn next_prev_without_shuffle() {
        let mut p = pl(3);
        assert_eq!(p.current(), Some(0));
        assert_eq!(p.next_index(false, false), Some(1));
        p.set_current(Some(2));
        assert_eq!(p.next_index(false, false), None);
        assert_eq!(p.next_index(false, true), Some(0));
        p.set_current(Some(0));
        assert_eq!(p.prev_index(false, false), None);
        assert_eq!(p.prev_index(false, true), Some(2));
    }

    #[test]
    fn shuffle_visits_everything_once() {
        let mut p = pl(20);
        let mut seen = HashSet::new();
        seen.insert(p.current().unwrap());
        while let Some(i) = p.next_index(true, false) {
            assert!(seen.insert(i), "{i} rejoué");
            p.set_current(Some(i));
        }
        assert_eq!(seen.len(), 20);
    }

    #[test]
    fn manual_advance_rules() {
        let mut p = pl(3);
        assert_eq!(p.auto_advance(false, false, true), None);
        assert_eq!(p.auto_advance(false, true, true), Some(0));
        assert_eq!(p.auto_advance(false, false, false), Some(1));
    }

    #[test]
    fn remove_and_crop_keep_current() {
        let mut p = pl(5);
        p.set_current(Some(3));
        p.click(1, false, false);
        p.click(2, false, true);
        p.remove_selected();
        assert_eq!(p.len(), 3);
        assert_eq!(p.current_entry().unwrap().display_title(), "Track 3");
        p.click(0, false, false);
        p.crop();
        assert_eq!(p.len(), 1);
    }

    #[test]
    fn move_selection_keeps_current_identity() {
        let mut p = pl(5);
        p.set_current(Some(4));
        p.click(0, false, false);
        p.click(1, true, false);
        assert!(p.move_selection(2));
        let titles: Vec<String> = p.entries().iter().map(|e| e.display_title()).collect();
        assert_eq!(titles, ["Track 2", "Track 3", "Track 0", "Track 1", "Track 4"]);
        assert_eq!(p.current(), Some(4));
    }

    #[test]
    fn natural_sort_and_search_and_totals() {
        let mut p = Playlist::new();
        for t in ["b10", "B2", "a1"] {
            p.extend([Entry::with_meta(Source::File(PathBuf::from(t)), Some(t.to_string()), None)]);
        }
        p.sort(SortKey::Title);
        let titles: Vec<String> = p.entries().iter().map(|e| e.display_title()).collect();
        assert_eq!(titles, ["a1", "B2", "b10"]);
        assert_eq!(p.search("b 1"), vec![2]);
        let t = p.total_time();
        assert!(t.unknown && t.partial);
        let q = pl(3);
        assert_eq!(q.total_time(), TotalTime { ms: 6000, partial: false, unknown: false });
    }
}
