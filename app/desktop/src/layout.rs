//! Disposition des fenêtres : aimantation aux bords (écran et autres fenêtres), groupe accroché à la fenêtre
//! principale, décalage des fenêtres collées quand une fenêtre change de taille.
//!
//! Tout est en unités de disposition (demi-points sur macOS, pixels physiques ailleurs) ; les fonctions sont pures
//! pour être testées sans écran.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct R {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl R {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> R {
        R { x, y, w, h }
    }
    fn right(&self) -> i32 {
        self.x + self.w
    }
    fn bottom(&self) -> i32 {
        self.y + self.h
    }
    fn moved(&self, dx: i32, dy: i32) -> R {
        R::new(self.x + dx, self.y + dy, self.w, self.h)
    }
}

fn overlap(a0: i32, a1: i32, b0: i32, b1: i32) -> bool {
    a0 < b1 && b0 < a1
}

/// Deux fenêtres se touchent-elles (bord commun avec un recouvrement) ?
pub fn touches(a: &R, b: &R) -> bool {
    ((a.right() == b.x || b.right() == a.x) && overlap(a.y, a.bottom(), b.y, b.bottom()))
        || ((a.bottom() == b.y || b.bottom() == a.y) && overlap(a.x, a.right(), b.x, b.right()))
}

/// Index des fenêtres reliées à `start` de proche en proche (y compris `start`).
pub fn docked_group(rects: &[Option<R>], start: usize) -> Vec<usize> {
    let mut group = vec![start];
    let mut i = 0;
    while i < group.len() {
        let a = rects[group[i]];
        for (j, b) in rects.iter().enumerate() {
            if let (Some(a), Some(b)) = (a, b) {
                if !group.contains(&j) && touches(&a, b) {
                    group.push(j);
                }
            }
        }
        i += 1;
    }
    group
}

/// Décalage à appliquer pour aimanter `moving` (déjà déplacées) aux `targets` et aux `screens`, à moins de `d`.
pub fn snap(moving: &[R], targets: &[R], screens: &[R], d: i32) -> (i32, i32) {
    let mut best_x: Option<i32> = None;
    let mut best_y: Option<i32> = None;
    let consider = |slot: &mut Option<i32>, delta: i32| {
        if delta.abs() < d && slot.is_none_or(|b| delta.abs() < b.abs()) {
            *slot = Some(delta);
        }
    };
    for m in moving {
        for t in targets {
            if overlap(m.y - d, m.bottom() + d, t.y, t.bottom()) {
                for delta in [t.right() - m.x, t.x - m.right(), t.x - m.x, t.right() - m.right()] {
                    consider(&mut best_x, delta);
                }
            }
            if overlap(m.x - d, m.right() + d, t.x, t.right()) {
                for delta in [t.bottom() - m.y, t.y - m.bottom(), t.y - m.y, t.bottom() - m.bottom()] {
                    consider(&mut best_y, delta);
                }
            }
        }
        for s in screens {
            for delta in [s.x - m.x, s.right() - m.right()] {
                consider(&mut best_x, delta);
            }
            for delta in [s.y - m.y, s.bottom() - m.bottom()] {
                consider(&mut best_y, delta);
            }
        }
    }
    (best_x.unwrap_or(0), best_y.unwrap_or(0))
}

/// Fenêtres à décaler quand `k` change de taille : celles empilées sous son bord bas (de proche en proche, vers
/// le bas) et celles alignées contre son bord droit (vers la droite).
pub fn attached(rects: &[Option<R>], k: usize) -> (Vec<usize>, Vec<usize>) {
    let grow = |vertical: bool| -> Vec<usize> {
        let mut set: Vec<usize> = Vec::new();
        let mut frontier = vec![k];
        while let Some(f) = frontier.pop() {
            let Some(a) = rects[f] else { continue };
            for (j, b) in rects.iter().enumerate() {
                let Some(b) = b else { continue };
                if j == k || set.contains(&j) {
                    continue;
                }
                let hit = if vertical {
                    b.y == a.bottom() && overlap(a.x, a.right(), b.x, b.right())
                } else {
                    b.x == a.right() && overlap(a.y, a.bottom(), b.y, b.bottom())
                };
                if hit {
                    set.push(j);
                    frontier.push(j);
                }
            }
        }
        set
    };
    (grow(true), grow(false))
}

/// Nouvelles positions après le changement de taille de `k` (`new_w` x `new_h`).
pub fn resize(rects: &[Option<R>], k: usize, new_w: i32, new_h: i32) -> Vec<Option<R>> {
    let Some(old) = rects[k] else { return rects.to_vec() };
    let (below, right) = attached(rects, k);
    let (dw, dh) = (new_w - old.w, new_h - old.h);
    let mut out = rects.to_vec();
    out[k] = Some(R::new(old.x, old.y, new_w, new_h));
    for j in below {
        out[j] = out[j].map(|r| r.moved(0, dh));
    }
    for j in right {
        out[j] = out[j].map(|r| r.moved(dw, 0));
    }
    out
}

/// Bord par lequel une fenêtre est collée à une autre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Below,
    Above,
    Right,
    Left,
}

/// `child` touche-t-il `parent` (à `tol` près) ? Bord, et décalage le long de ce bord (x pour dessus/dessous, y pour
/// gauche/droite).
fn contact(parent: &R, child: &R, tol: i32) -> Option<(Side, i32)> {
    let horiz = overlap(parent.x, parent.right(), child.x, child.right());
    let vert = overlap(parent.y, parent.bottom(), child.y, child.bottom());
    if horiz && (child.y - parent.bottom()).abs() <= tol {
        Some((Side::Below, child.x - parent.x))
    } else if horiz && (parent.y - child.bottom()).abs() <= tol {
        Some((Side::Above, child.x - parent.x))
    } else if vert && (child.x - parent.right()).abs() <= tol {
        Some((Side::Right, child.y - parent.y))
    } else if vert && (parent.x - child.right()).abs() <= tol {
        Some((Side::Left, child.y - parent.y))
    } else {
        None
    }
}

/// Arbre des fenêtres collées, en largeur depuis la fenêtre 0 (la principale) : `(fenêtre, parent)` dans l'ordre où
/// les placer, le parent étant `(index, bord, décalage)`. Les fenêtres jamais atteintes deviennent des racines.
/// `can_parent[i] == false` (fenêtre fermée) : elle peut s'accrocher à une autre, mais rien ne s'accroche à elle.
pub fn dock_tree(rects: &[Option<R>], can_parent: &[bool], tol: i32) -> Vec<(usize, Option<(usize, Side, i32)>)> {
    let mut out: Vec<(usize, Option<(usize, Side, i32)>)> = Vec::new();
    let mut seen = vec![false; rects.len()];
    for root in 0..rects.len() {
        if seen[root] || rects[root].is_none() {
            continue;
        }
        seen[root] = true;
        let start = out.len();
        out.push((root, None));
        let mut i = start;
        while i < out.len() {
            let p = out[i].0;
            i += 1;
            if !can_parent.get(p).copied().unwrap_or(true) {
                continue;
            }
            let Some(pr) = rects[p] else { continue };
            for (c, cr) in rects.iter().enumerate() {
                let Some(cr) = cr else { continue };
                if seen[c] {
                    continue;
                }
                if let Some((side, along)) = contact(&pr, cr, tol) {
                    seen[c] = true;
                    out.push((c, Some((p, side, along))));
                }
            }
        }
    }
    out
}

/// Recolle les fenêtres après un changement de taille (autre agrandissement, autres unités) : chaque racine garde
/// sa position, les autres reprennent leur place contre leur parent, à leur nouvelle taille `sizes`. Le décalage le
/// long du bord est mis à l'échelle par `ratio` (nouvelle taille d'un pixel skin / ancienne). `main_at` : nouvelle
/// position de la fenêtre 0 (celle que le système lui a vraiment donnée), que son groupe suit.
pub fn redock(saved: &[Option<R>], can_parent: &[bool], sizes: &[Option<(i32, i32)>], ratio: f64, tol: i32, main_at: Option<(i32, i32)>) -> Vec<Option<R>> {
    let mut out: Vec<Option<R>> = vec![None; saved.len()];
    for (i, parent) in dock_tree(saved, can_parent, tol) {
        let Some(old) = saved[i] else { continue };
        let (w, h) = sizes.get(i).copied().flatten().unwrap_or((old.w, old.h));
        out[i] = Some(match parent.and_then(|(p, side, along)| out[p].map(|pr| (pr, side, along))) {
            None => match main_at {
                Some((x, y)) if i == 0 => R::new(x, y, w, h),
                _ => R::new(old.x, old.y, w, h),
            },
            Some((p, side, along)) => {
                let a = (along as f64 * ratio).round() as i32;
                match side {
                    Side::Below => R::new(p.x + a, p.bottom(), w, h),
                    Side::Above => R::new(p.x + a, p.y - h, w, h),
                    Side::Right => R::new(p.right(), p.y + a, w, h),
                    Side::Left => R::new(p.x - w, p.y + a, w, h),
                }
            }
        });
    }
    out
}

/// Décalage vertical (positif ou nul) qui garde le haut de chaque fenêtre sous le haut de la zone utile (sous la
/// barre de menus du Mac). Pour une fenêtre, la limite est le haut de la zone la plus haute parmi celles qui couvrent
/// son milieu ; une fenêtre au-dessus d'aucun écran n'impose rien.
pub fn clamp_top(moving: &[R], areas: &[R]) -> i32 {
    let mut d = 0;
    for m in moving {
        let cx = m.x + m.w / 2;
        if let Some(top) = areas.iter().filter(|a| cx >= a.x && cx < a.right()).map(|a| a.y).min() {
            d = d.max(top - m.y);
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_and_snap() {
        let main = R::new(100, 100, 275, 116);
        let eq = R::new(100, 216, 275, 116);
        let pl = R::new(375, 100, 275, 232);
        let far = R::new(900, 900, 50, 50);
        let rects = [Some(main), Some(eq), Some(pl), Some(far)];
        let mut g = docked_group(&rects, 0);
        g.sort();
        assert_eq!(g, vec![0, 1, 2]);
        // Une fenêtre lâchée à 6 px sous l'égaliseur s'y colle.
        let moving = R::new(104, 338, 275, 116);
        assert_eq!(snap(&[moving], &[main, eq, pl], &[], 10), (-4, -6));
        // Bord de l'écran.
        assert_eq!(snap(&[R::new(5, 500, 10, 10)], &[], &[R::new(0, 0, 1920, 1080)], 10), (-5, 0));
    }

    #[test]
    fn resize_moves_stacked_windows() {
        let rects = [Some(R::new(0, 0, 275, 116)), Some(R::new(0, 116, 275, 116)), Some(R::new(0, 232, 275, 232)), Some(R::new(275, 0, 275, 116))];
        let out = resize(&rects, 0, 275, 14);
        assert_eq!(out[1].unwrap().y, 14);
        assert_eq!(out[2].unwrap().y, 130);
        assert_eq!(out[3].unwrap().x, 275);
        let out = resize(&rects, 0, 550, 232);
        assert_eq!(out[1].unwrap().y, 232);
        assert_eq!(out[3].unwrap().x, 550);
    }

    #[test]
    fn redock_after_zoom_change() {
        // ×2 (demi-points : ×4) : principale, égaliseur dessous, playlist (174 px skin) dessous.
        let saved = [Some(R::new(100, 60, 1100, 464)), Some(R::new(100, 524, 1100, 464)), Some(R::new(100, 988, 1100, 696))];
        let all = [true, true, true];
        // ×1,5 (demi-points : ×3).
        let sizes = [Some((825, 348)), Some((825, 348)), Some((825, 522))];
        let out = redock(&saved, &all, &sizes, 0.75, 2, None);
        assert_eq!(out[0], Some(R::new(100, 60, 825, 348)));
        assert_eq!(out[1], Some(R::new(100, 408, 825, 348)));
        assert_eq!(out[2], Some(R::new(100, 756, 825, 522)));
        // Bandeaux : égaliseur et playlist de 14 px skin sous la principale.
        let saved = [Some(R::new(0, 0, 1100, 464)), Some(R::new(0, 464, 1100, 56)), Some(R::new(0, 520, 1100, 56))];
        let sizes = [Some((825, 348)), Some((825, 42)), Some((825, 42))];
        let out = redock(&saved, &all, &sizes, 0.75, 2, None);
        assert_eq!(out[1].unwrap().y, 348);
        assert_eq!(out[2].unwrap().y, 390);
        // La principale a été replacée par le système (sous la barre de menus) : le groupe la suit.
        let out = redock(&saved, &all, &sizes, 0.75, 2, Some((10, 48)));
        assert_eq!(out[0], Some(R::new(10, 48, 825, 348)));
        assert_eq!((out[2].unwrap().x, out[2].unwrap().y), (10, 438));
    }

    #[test]
    fn redock_side_and_detached_windows() {
        // Playlist à droite de la principale, décalée de 40 vers le bas ; égaliseur loin, non collé.
        let saved = [Some(R::new(0, 0, 100, 50)), Some(R::new(500, 500, 100, 50)), Some(R::new(100, 40, 100, 100))];
        let all = [true, true, true];
        let sizes = [Some((150, 75)), Some((150, 75)), Some((150, 150))];
        let out = redock(&saved, &all, &sizes, 1.5, 2, Some((0, 0)));
        assert_eq!(out[2], Some(R::new(150, 60, 150, 150)));
        assert_eq!(out[1], Some(R::new(500, 500, 150, 75)));
        // Une fenêtre fermée ne sert pas de pont.
        let saved = [Some(R::new(0, 0, 100, 50)), Some(R::new(0, 50, 100, 50)), Some(R::new(0, 100, 100, 50))];
        let tree = dock_tree(&saved, &[true, false, true], 0);
        assert_eq!(tree[2], (2, None));
        assert_eq!(tree[1], (1, Some((0, Side::Below, 0))));
    }

    #[test]
    fn clamp_under_menu_bar() {
        // Zone utile sous une barre de menus de 48 demi-points ; un second écran au-dessus, à droite.
        let areas = [R::new(0, 48, 3024, 1900), R::new(3024, -2160, 3840, 2160)];
        assert_eq!(clamp_top(&[R::new(100, 10, 825, 348)], &areas), 38);
        assert_eq!(clamp_top(&[R::new(100, 60, 825, 348)], &areas), 0);
        // Le groupe entier est retenu par sa fenêtre la plus haute.
        assert_eq!(clamp_top(&[R::new(100, 400, 825, 348), R::new(100, 30, 825, 348)], &areas), 18);
        // Sur l'écran du dessus, la limite est son propre haut.
        assert_eq!(clamp_top(&[R::new(4000, -100, 825, 348)], &areas), 0);
        assert_eq!(clamp_top(&[R::new(4000, -2200, 825, 348)], &areas), 40);
    }
}
