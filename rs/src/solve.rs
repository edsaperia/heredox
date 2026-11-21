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
use heredox::{DISTINCT_ROTATIONS, TILES};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default)]
pub struct Fx(u64);
impl Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(0x517cc1b727220a95);
        }
    }
    fn write_u64(&mut self, v: u64) {
        self.0 = (self.0.rotate_left(5) ^ v).wrapping_mul(0x517cc1b727220a95);
    }
    fn write_u32(&mut self, v: u32) {
        self.write_u64(v as u64)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct State {
    pub mask: u16,        // filled cells, bit y*4+x, shape pushed to the top-left
    pub cells: [u8; 16],  // edges, 2 bits per direction
    pub counts: [u8; 5],  // tiles left, by type
    pub diff: i8,         // mover's symbols minus the other's
}

fn pack(e: [u8; 4]) -> u8 {
    e[0] | e[1] << 2 | e[2] << 4 | e[3] << 6
}
fn edge(c: u8, d: usize) -> u8 {
    (c >> (2 * d)) & 3
}
fn swap_sym(c: u8) -> u8 {
    // 1 <-> 2 in every 2-bit field, 0 stays 0
    let lo = c & 0x55;
    let hi = (c >> 1) & 0x55;
    hi | lo << 1
}

impl State {
    pub fn start() -> State {
        State { mask: 0, cells: [0; 16], counts: heredox::COUNTS, diff: 0 }
    }
    pub fn remaining(&self) -> u8 {
        self.counts.iter().sum()
    }
    fn size(&self) -> (i32, i32) {
        let (mut w, mut h) = (0, 0);
        for i in 0..16 {
            if self.mask >> i & 1 == 1 {
                w = w.max(i as i32 % 4 + 1);
                h = h.max(i as i32 / 4 + 1);
            }
        }
        (w, h)
    }
    fn filled(&self, x: i32, y: i32) -> bool {
        (0..4).contains(&x) && (0..4).contains(&y) && self.mask >> (y * 4 + x) & 1 == 1
    }

    /// Cells where a tile may go, in window coordinates (may be -1).
    pub fn legal_cells(&self) -> Vec<(i32, i32)> {
        if self.mask == 0 {
            return vec![(0, 0)];
        }
        let (w, h) = self.size();
        let mut out = Vec::new();
        for y in -1..=h.min(3) {
            for x in -1..=w.min(3) {
                if self.filled(x, y) {
                    continue;
                }
                if (x == -1 && w == 4) || (y == -1 && h == 4) {
                    continue;
                }
                if (x == -1 || x == w) && w == 4 {
                    continue;
                }
                if (y == -1 || y == h) && h == 4 {
                    continue;
                }
                if self.filled(x - 1, y) || self.filled(x + 1, y) || self.filled(x, y - 1) || self.filled(x, y + 1) {
                    out.push((x, y));
                }
            }
        }
        out
    }

    /// Place tile type t, rotation r, at (x, y); return the position as seen
    /// by the next player.
    pub fn place(&self, t: usize, r: u8, x: i32, y: i32) -> State {
        let e = heredox::rotate(TILES[t], r);
        let mut diff = self.diff as i32;
        for d in 0..4 {
            let (dx, dy) = heredox::DIRS[d];
            let (nx, ny) = (x + dx, y + dy);
            if e[d] != 0 && self.filled(nx, ny) && edge(self.cells[(ny * 4 + nx) as usize], (d + 2) % 4) == e[d] {
                diff += if e[d] == 1 { 1 } else { -1 };
            }
        }
        let (sx, sy) = ((x == -1) as i32, (y == -1) as i32);
        let mut n = State { mask: 0, cells: [0; 16], counts: self.counts, diff: -(diff as i8) };
        for i in 0..16i32 {
            if self.mask >> i & 1 == 1 {
                let j = ((i / 4 + sy) * 4 + i % 4 + sx) as usize;
                n.mask |= 1 << j;
                n.cells[j] = swap_sym(self.cells[i as usize]);
            }
        }
        let j = ((y + sy) * 4 + x + sx) as usize;
        n.mask |= 1 << j;
        n.cells[j] = swap_sym(pack(e));
        n.counts[t] -= 1;
        n.counts.swap(0, 1); // symbol swap exchanges types A and B
        n
    }

    fn normalise(&mut self) {
        let (mut minx, mut miny) = (4, 4);
        for i in 0..16 {
            if self.mask >> i & 1 == 1 {
                minx = minx.min(i % 4);
                miny = miny.min(i / 4);
            }
        }
        if self.mask == 0 || (minx == 0 && miny == 0) {
            return;
        }
        let (m, c) = (self.mask, self.cells);
        self.mask = 0;
        self.cells = [0; 16];
        for i in 0..16 {
            if m >> i & 1 == 1 {
                let j = (i / 4 - miny) * 4 + i % 4 - minx;
                self.mask |= 1 << j;
                self.cells[j] = c[i];
            }
        }
    }
    fn rot90(&self) -> State {
        let mut n = State { mask: 0, cells: [0; 16], counts: self.counts, diff: self.diff };
        for i in 0..16usize {
            if self.mask >> i & 1 == 1 {
                let (x, y) = (i % 4, i / 4);
                let j = x * 4 + (3 - y);
                n.mask |= 1 << j;
                let c = self.cells[i];
                n.cells[j] = (c << 2) | (c >> 6);
            }
        }
        n.normalise();
        n
    }
    fn mirror(&self) -> State {
        let mut n = State { mask: 0, cells: [0; 16], counts: self.counts, diff: self.diff };
        n.counts.swap(0, 1);
        for i in 0..16usize {
            if self.mask >> i & 1 == 1 {
                let (x, y) = (i % 4, i / 4);
                let j = y * 4 + (3 - x);
                n.mask |= 1 << j;
                let c = self.cells[i];
                n.cells[j] = (c & 0x33) | ((c >> 4) & 0x0c) | ((c & 0x0c) << 4);
            }
        }
        n.normalise();
        n
    }

    fn raw_key(&self) -> (u64, u32) {
        let (w, h) = self.size();
        let mut edges: u64 = 0;
        let mut n = 0;
        for i in 0..16i32 {
            if self.mask >> i & 1 == 0 {
                continue;
            }
            let (x, y) = (i % 4, i / 4);
            for d in 0..4 {
                let (dx, dy) = heredox::DIRS[d];
                let (nx, ny) = (x + dx, y + dy);
                if self.filled(nx, ny) {
                    continue;
                }
                let open = (if dx != 0 { (0..4).contains(&nx) || w < 4 } else { true })
                    && (if dy != 0 { (0..4).contains(&ny) || h < 4 } else { true });
                // nx == w with w < 4 is inside the window, so covered above
                let v = if open { edge(self.cells[i as usize], d) as u64 } else { 0 };
                edges |= v << (2 * n);
                n += 1;
            }
        }
        assert!(n <= 29);
        edges |= ((self.diff as i64 + 12) as u64) << 58;
        let c = self.counts;
        let small = (self.mask as u32)
            | (c[0] as u32) << 16
            | (c[1] as u32) << 18
            | (c[2] as u32) << 20
            | (c[3] as u32) << 22
            | (c[4] as u32) << 25;
        (edges, small)
    }

    pub fn key(&self) -> (u64, u32) {
        let mut best = (u64::MAX, u32::MAX);
        let mut s = *self;
        for _ in 0..4 {
            best = best.min(s.raw_key()).min(s.mirror().raw_key());
            s = s.rot90();
        }
        best
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Objective {
    WinOnly, // maximise own win probability; a draw is as bad as a loss
    ZeroSum, // win 1, draw 1/2, loss 0
}

pub struct Solver {
    pub memo: HashMap<(u64, u32), (f64, f64), BuildHasherDefault<Fx>>,
    pub obj: Objective,
    pub use_memo: bool,
}

fn better(obj: Objective, a: (f64, f64), b: (f64, f64)) -> bool {
    const EPS: f64 = 1e-12;
    match obj {
        Objective::WinOnly => a.0 > b.0 + EPS || ((a.0 - b.0).abs() <= EPS && a.1 < b.1 - EPS),
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
            return if s.diff > 0 { (1.0, 0.0) } else if s.diff < 0 { (0.0, 1.0) } else { (0.0, 0.0) };
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

// --- small deterministic RNG for sampling positions ---------------------
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Play random moves from the start until `left` tiles remain.
pub fn random_position(rng: &mut Rng, left: u8) -> State {
    let mut s = State::start();
    while s.remaining() > left {
        let mut k = rng.below(s.remaining() as usize) as u8;
        let mut t = 0;
        while k >= s.counts[t] {
            k -= s.counts[t];
            t += 1;
        }
        let cells = s.legal_cells();
        let (x, y) = cells[rng.below(cells.len())];
        let r = rng.below(DISTINCT_ROTATIONS[t] as usize) as u8;
        s = s.place(t, r, x, y);
    }
    s
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
