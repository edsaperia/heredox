//! Exact solver for face-down Heredox.
//!
//! A position is always stored from the point of view of the player to move,
//! whose symbol is written as 1 (the board and the remaining tiles are
//! symbol-swapped after every move). The value of a position is the pair
//! (P(mover wins), P(other wins)) under the chosen objective.
//!
//! What the future depends on is only: which cells are filled, the halves on
//! edges that can still get a neighbour, the tiles left, and the score
//! difference so far. That is the memo key, reduced by the 8 board symmetries
//! (reflections also swap the mirror-image tile types A and B).
use heredox::pos::*;
use heredox::DISTINCT_ROTATIONS;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

#[derive(Clone, Copy, PartialEq)]
pub enum Objective {
    WinOnly, // maximise own win probability; a draw is as bad as a loss
    ZeroSum, // win 1, draw 1/2, loss 0
    TieFirst, // no draws: equal counts are a win for the player who moved first
}

pub struct Solver {
    pub memo: HashMap<(u64, u32), (f64, f64), BuildHasherDefault<Fx>>,
    pub obj: Objective,
    pub use_memo: bool,
}

fn better(obj: Objective, a: (f64, f64), b: (f64, f64)) -> bool {
    const EPS: f64 = 1e-12;
    match obj {
        Objective::WinOnly | Objective::TieFirst => a.0 > b.0 + EPS || ((a.0 - b.0).abs() <= EPS && a.1 < b.1 - EPS),
        Objective::ZeroSum => a.0 - a.1 > b.0 - b.1 + EPS || (((a.0 - a.1) - (b.0 - b.1)).abs() <= EPS && a.0 > b.0 + EPS),
    }
}

impl Solver {
    pub fn new(obj: Objective) -> Solver {
        Solver { memo: HashMap::default(), obj, use_memo: true }
    }

    /// Value to the mover of placing type t at the best spot: (mine, theirs).
    pub fn best_for_tile(&mut self, s: &State, t: usize) -> ((f64, f64), (u8, i32, i32)) {
        let mut best = (-1.0, 2.0);
        let mut mv = (0, 0, 0);
        let rots = if s.mask == 0 { 1 } else { DISTINCT_ROTATIONS[t] };
        for (x, y) in s.legal_cells() {
            for r in 0..rots {
                let (cm, co) = self.solve(&s.place(t, r, x, y));
                let mine = (co, cm);
                if better(self.obj, mine, best) {
                    best = mine;
                    mv = (r, x, y);
                }
            }
        }
        (best, mv)
    }

    /// (P(mover wins), P(other wins)) before the mover draws.
    pub fn solve(&mut self, s: &State) -> (f64, f64) {
        let rem = s.remaining();
        if rem == 0 {
            return if s.diff > 0 || (s.diff == 0 && self.obj == Objective::TieFirst) { (1.0, 0.0) } else if s.diff < 0 { (0.0, 1.0) } else { (0.0, 0.0) };
        }
        let key = if self.use_memo { s.key() } else { (0, 0) };
        if self.use_memo {
            if let Some(&(a, b)) = self.memo.get(&key) {
                return (a, b);
            }
        }
        let mut acc = (0.0, 0.0);
        for t in 0..5 {
            if s.counts[t] == 0 {
                continue;
            }
            let p = s.counts[t] as f64 / rem as f64;
            let (v, _) = self.best_for_tile(s, t);
            acc.0 += p * v.0;
            acc.1 += p * v.1;
        }
        if self.use_memo {
            self.memo.insert(key, acc);
        }
        acc
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("endgame");
    match cmd {
        // check the memoised, symmetry-reduced solver against plain search
        "selfcheck" => {
            let mut rng = Rng(0x9e3779b97f4a7c15);
            for obj in [Objective::WinOnly, Objective::ZeroSum] {
                let mut worst: f64 = 0.0;
                for i in 0..300 {
                    let s = random_position(&mut rng, 3 + (i % 3) as u8);
                    let mut a = Solver::new(obj);
                    let mut b = Solver::new(obj);
                    b.use_memo = false;
                    let (x, y) = (a.solve(&s), b.solve(&s));
                    worst = worst.max((x.0 - y.0).abs()).max((x.1 - y.1).abs());
                }
                println!("selfcheck: 300 positions with 3 to 5 tiles left, largest disagreement {:.2e}", worst);
            }
        }
        // value of random positions with n tiles left, and memo growth
        "endgame" => {
            let left: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(6);
            let n: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(200);
            let mut rng = Rng(12345);
            let mut sv = Solver::new(Objective::WinOnly);
            let t0 = std::time::Instant::now();
            let (mut pm, mut po) = (0.0, 0.0);
            for _ in 0..n {
                let s = random_position(&mut rng, left);
                let v = sv.solve(&s);
                pm += v.0;
                po += v.1;
            }
            println!(
                "{} tiles left, {} random positions: mover wins {:.3}, other wins {:.3}, draw {:.3}; memo {} states, {:.1}s",
                left, n, pm / n as f64, po / n as f64, 1.0 - (pm + po) / n as f64, sv.memo.len(), t0.elapsed().as_secs_f64()
            );
        }

        // seat `exact_seat` plays greedy until k tiles are left, then exactly;
        // the other seat is greedy throughout. k = 0 is greedy against greedy.
        "duel" => {
            let k: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
            let n: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(200);
            let mut rng = Rng(777);
            let mut sv = Solver::new(Objective::WinOnly);
            for exact_seat in 0..2usize {
                let mut res = [0usize; 3]; // exact seat wins, draws, loses
                for _ in 0..n {
                    let mut s = State::start();
                    for turn in 0..16usize {
                        let mut j = rng.below(s.remaining() as usize) as u8;
                        let mut t = 0;
                        while j >= s.counts[t] {
                            j -= s.counts[t];
                            t += 1;
                        }
                        let (r, x, y) = if turn % 2 == exact_seat && s.remaining() <= k {
                            sv.best_for_tile(&s, t).1
                        } else {
                            let mut best = Vec::new();
                            let mut bg = -99;
                            for (x, y) in s.legal_cells() {
                                for r in 0..DISTINCT_ROTATIONS[t] {
                                    let g = -(s.place(t, r, x, y).diff as i32) - s.diff as i32;
                                    if g > bg {
                                        bg = g;
                                        best.clear();
                                    }
                                    if g == bg {
                                        best.push((r, x, y));
                                    }
                                }
                            }
                            best[rng.below(best.len())]
                        };
                        s = s.place(t, r, x, y);
                    }
                    // after 16 moves the stored view is seat 0's
                    let d = if exact_seat == 0 { s.diff } else { -s.diff };
                    res[if d > 0 { 0 } else if d == 0 { 1 } else { 2 }] += 1;
                }
                println!(
                    "exact from {} left, moving {}: wins {} draws {} losses {} of {}",
                    k, if exact_seat == 0 { "first" } else { "second" }, res[0], res[1], res[2], n
                );
            }
        }

        // positions with their exact values, one JSON object per line, for
        // checking the JavaScript solver
        "dump" => {
            let left: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
            let n: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(100);
            let mut rng = Rng(424242 + left as u64);
            let mut sv = Solver::new(Objective::TieFirst);
            for _ in 0..n {
                let s = random_position(&mut rng, left);
                let v = sv.solve(&s);
                let cells: Vec<String> = (0..16)
                    .map(|i| {
                        if s.mask >> i & 1 == 1 {
                            let c = s.cells[i];
                            format!("[{},{},{},{}]", c & 3, c >> 2 & 3, c >> 4 & 3, c >> 6 & 3)
                        } else {
                            "null".to_string()
                        }
                    })
                    .collect();
                println!(
                    "{{\"mask\":{},\"cells\":[{}],\"counts\":[{},{},{},{},{}],\"diff\":{},\"pm\":{:.12},\"po\":{:.12}}}",
                    s.mask, cells.join(","), s.counts[0], s.counts[1], s.counts[2], s.counts[3], s.counts[4], s.diff, v.0, v.1
                );
            }
        }

        // both seats greedy until k tiles are left, then both exact, under
        // the normal rule and under "equal counts: the first player wins"
        "fair" => {
            let k: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
            let n: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(200);
            for obj in [Objective::WinOnly, Objective::TieFirst] {
                let mut rng = Rng(2024);
                let mut sv = Solver::new(obj);
                let mut res = [0usize; 3]; // first seat wins, level, second seat wins (by count)
                for _ in 0..n {
                    let mut s = State::start();
                    for _turn in 0..16usize {
                        let mut j = rng.below(s.remaining() as usize) as u8;
                        let mut t = 0;
                        while j >= s.counts[t] {
                            j -= s.counts[t];
                            t += 1;
                        }
                        let (r, x, y) = if s.remaining() <= k {
                            sv.best_for_tile(&s, t).1
                        } else {
                            let mut best = Vec::new();
                            let mut bg = -99;
                            for (x, y) in s.legal_cells() {
                                for r in 0..DISTINCT_ROTATIONS[t] {
                                    let g = -(s.place(t, r, x, y).diff as i32) - s.diff as i32;
                                    if g > bg {
                                        bg = g;
                                        best.clear();
                                    }
                                    if g == bg {
                                        best.push((r, x, y));
                                    }
                                }
                            }
                            best[rng.below(best.len())]
                        };
                        s = s.place(t, r, x, y);
                    }
                    res[if s.diff > 0 { 0 } else if s.diff == 0 { 1 } else { 2 }] += 1;
                }
                println!(
                    "{} exact from {} left: first seat ahead {}, level {}, second seat ahead {} of {}",
                    if obj == Objective::TieFirst { "ties to first," } else { "normal rule,  " }, k, res[0], res[1], res[2], n
                );
            }
        }
        "root" => {
            let obj = if args.get(2).map(|s| s.as_str()) == Some("zerosum") { Objective::ZeroSum } else { Objective::WinOnly };
            let mut sv = Solver::new(obj);
            let t0 = std::time::Instant::now();
            let v = sv.solve(&State::start());
            println!(
                "start: first player wins {:.5}, second wins {:.5}, draw {:.5}; memo {} states, {:.1}s",
                v.0, v.1, 1.0 - v.0 - v.1, sv.memo.len(), t0.elapsed().as_secs_f64()
            );
        }
        _ => eprintln!("usage: solve selfcheck | endgame <left> <n> | root [zerosum]"),
    }
}
