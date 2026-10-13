//! Disposition des fenêtres : aimantation aux bords (écran et autres fenêtres), groupe accroché à la fenêtre
//! principale, décalage des fenêtres collées quand une fenêtre change de taille.
//!
//! Tout est en pixels physiques ; les fonctions sont pures pour être testées sans écran.

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
}
