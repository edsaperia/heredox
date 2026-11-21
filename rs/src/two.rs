//! The two-tile game: exact endgame solver, value network, and players.
//!
//! Rules: there are always two tiles to pick from. The mover places one; the
//! other stays for the opponent, who draws one more from the stack. Equal
//! counts at the end are a win for the player who moved first.
//!
//! Positions use `pos::State` (mover's symbol is 1) with `counts` holding the
//! face-down stack only. Because the symbols swap with every move, tile types
//! A and B swap names too: use `swap_ab` on a tile carried across a move.
use crate::pos::{Fx, Rng, State};
use crate::{DIRS, DISTINCT_ROTATIONS};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

pub const NONE: u8 = 9;

pub fn swap_ab(t: u8) -> u8 {
    match t {
        0 => 1,
        1 => 0,
        x => x,
    }
}

/// Place a tile that is in hand (not in the stack). Returns the position as
/// the next player sees it.
pub fn put(s: &State, t: u8, r: u8, x: i32, y: i32) -> State {
    let mut tmp = *s;
    tmp.counts[t as usize] += 1;
    tmp.place(t as usize, r, x, y)
}

pub fn placed(s: &State) -> u32 {
    s.mask.count_ones()
}
/// Is the player to move the one who moved first in the game?
pub fn mover_is_first(s: &State) -> bool {
    placed(s) % 2 == 0
}

fn key_h(s: &State, h: u8) -> (u64, u32, u8) {
    let mut best = (u64::MAX, u32::MAX, u8::MAX);
    let mut t = *s;
    for _ in 0..4 {
        let a = t.raw_key();
        best = best.min((a.0, a.1, h));
        let b = t.mirror().raw_key();
        best = best.min((b.0, b.1, swap_ab(h)));
        t = t.rot90();
    }
    best
}

#[derive(Clone, Copy, Debug)]
pub struct Move {
    pub w: usize, // which of the tiles in hand
    pub r: u8,
    pub x: i32,
    pub y: i32,
}

/// Every distinct way to play `hand` from `s`: (move, resulting position,
/// the tile left for the opponent as they will name it).
pub fn options(s: &State, hand: &[u8]) -> Vec<(Move, State, u8)> {
    let mut out = Vec::new();
    for (w, &t) in hand.iter().enumerate() {
        if w == 1 && hand[0] == hand[1] {
            continue;
        }
        let other = if hand.len() == 2 { swap_ab(hand[1 - w]) } else { NONE };
        let rots = if s.mask == 0 { 1 } else { DISTINCT_ROTATIONS[t as usize] };
        for (x, y) in s.legal_cells() {
            for r in 0..rots {
                out.push((Move { w, r, x, y }, put(s, t, r, x, y), other));
            }
        }
    }
    out
}

// --- exact solver ---------------------------------------------------------
pub struct Solver2 {
    pub memo: HashMap<(u64, u32, u8), f64, BuildHasherDefault<Fx>>,
    pub use_memo: bool,
}

impl Default for Solver2 {
    fn default() -> Self {
        Solver2 { memo: HashMap::default(), use_memo: true }
    }
}

impl Solver2 {
    /// Best play for the mover holding `hand`: (P(mover wins), move).
    pub fn best(&mut self, s: &State, hand: &[u8]) -> (f64, Move) {
        let mut best = -1.0;
        let mut mv = Move { w: 0, r: 0, x: 0, y: 0 };
        for (m, child, other) in options(s, hand) {
            let v = 1.0 - self.solve(&child, other);
            if v > best + 1e-12 {
                best = v;
                mv = m;
            }
        }
        (best, mv)
    }

    /// P(mover wins) before the mover draws, holding the tile `h` left for
    /// them (NONE if there is none).
    pub fn solve(&mut self, s: &State, h: u8) -> f64 {
        let rem = s.remaining();
        if rem == 0 && h == NONE {
            // board full: equal counts go to the first player
            return if s.diff > 0 || (s.diff == 0 && mover_is_first(s)) { 1.0 } else { 0.0 };
        }
        let key = if self.use_memo { key_h(s, h) } else { (0, 0, 0) };
        if self.use_memo {
            if let Some(&v) = self.memo.get(&key) {
                return v;
            }
        }
        let v = if rem == 0 {
            self.best(s, &[h]).0
        } else {
            let mut acc = 0.0;
            for d in 0..5u8 {
                let c = s.counts[d as usize];
                if c == 0 {
                    continue;
                }
                let mut n = *s;
                n.counts[d as usize] -= 1;
                let v = if h == NONE { self.best(&n, &[d]).0 } else { self.best(&n, &[h, d]).0 };
                acc += c as f64 / rem as f64 * v;
            }
            acc
        };
        if self.use_memo {
            self.memo.insert(key, v);
        }
        v
    }
}

// --- value network ----------------------------------------------------------
// Input: a position just after a tile was placed, seen by the player about to
// draw, plus the tile left for them. Output: P(that player wins).
pub const NF: usize = 166;
pub const H1: usize = 128;
pub const H2: usize = 64;

pub fn features(s: &State, h: u8, out: &mut [f32; NF]) {
    out.iter_mut().for_each(|v| *v = 0.0);
    let (w, hh) = s.size();
    for i in 0..16i32 {
        if s.mask >> i & 1 == 0 {
            continue;
        }
        let base = i as usize * 9;
        out[base] = 1.0;
        let (x, y) = (i % 4, i / 4);
        for d in 0..4 {
            let (dx, dy) = DIRS[d];
            let (nx, ny) = (x + dx, y + dy);
            if s.filled(nx, ny) {
                continue;
            }
            // an edge counts only while a tile could still be put against it
            let open = (dx == 0 || (0..4).contains(&nx) || w < 4) && (dy == 0 || (0..4).contains(&ny) || hh < 4);
            let v = (s.cells[i as usize] >> (2 * d)) & 3;
            if open && v != 0 {
                out[base + 2 * d + v as usize] = 1.0; // +1 own, +2 theirs
            }
        }
    }
    if s.mask != 0 {
        out[144 + w as usize - 1] = 1.0;
        out[148 + hh as usize - 1] = 1.0;
    }
    out[152 + if h == NONE { 5 } else { h as usize }] = 1.0;
    for t in 0..5 {
        out[158 + t] = s.counts[t] as f32 / 4.0;
    }
    out[163] = s.diff as f32 / 6.0;
    out[164] = s.remaining() as f32 / 14.0;
    out[165] = if mover_is_first(s) { 1.0 } else { 0.0 };
}

#[derive(Clone)]
pub struct Net {
    pub w1: Vec<f32>, // [input][H1]
    pub b1: Vec<f32>,
    pub w2: Vec<f32>, // [H2][H1]
    pub b2: Vec<f32>,
    pub w3: Vec<f32>, // [H2]
    pub b3: f32,
}

impl Net {
    pub fn new(rng: &mut Rng) -> Net {
        let mut u = |scale: f32| ((rng.next() >> 11) as f32 / (1u64 << 53) as f32 * 2.0 - 1.0) * scale;
        let s1 = (6.0f32 / 20.0).sqrt(); // about 20 inputs are ever on at once
        let s2 = (6.0f32 / H1 as f32).sqrt();
        let s3 = (6.0f32 / H2 as f32).sqrt();
        Net {
            w1: (0..NF * H1).map(|_| u(s1)).collect(),
            b1: vec![0.0; H1],
            w2: (0..H2 * H1).map(|_| u(s2)).collect(),
            b2: vec![0.0; H2],
            w3: (0..H2).map(|_| u(s3)).collect(),
            b3: 0.0,
        }
    }
    pub fn n_params(&self) -> usize {
        self.w1.len() + self.b1.len() + self.w2.len() + self.b2.len() + self.w3.len() + 1
    }
    pub fn flat(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(self.n_params());
        v.extend(&self.w1);
        v.extend(&self.b1);
        v.extend(&self.w2);
        v.extend(&self.b2);
        v.extend(&self.w3);
        v.push(self.b3);
        v
    }
    pub fn set_flat(&mut self, v: &[f32]) {
        assert_eq!(v.len(), self.n_params());
        let mut i = 0;
        for part in [&mut self.w1, &mut self.b1, &mut self.w2, &mut self.b2, &mut self.w3] {
            let n = part.len();
            part.copy_from_slice(&v[i..i + n]);
            i += n;
        }
        self.b3 = v[i];
    }
    /// Forward pass. Fills the hidden activations and returns the output.
    pub fn forward(&self, x: &[f32; NF], a1: &mut [f32; H1], a2: &mut [f32; H2]) -> f32 {
        a1.copy_from_slice(&self.b1);
        for i in 0..NF {
            let xi = x[i];
            if xi != 0.0 {
                let row = &self.w1[i * H1..(i + 1) * H1];
                for j in 0..H1 {
                    a1[j] += xi * row[j];
                }
            }
        }
        for v in a1.iter_mut() {
            if *v < 0.0 {
                *v = 0.0;
            }
        }
        let mut z = self.b3;
        for k in 0..H2 {
            let row = &self.w2[k * H1..(k + 1) * H1];
            let mut acc = self.b2[k];
            for j in 0..H1 {
                acc += row[j] * a1[j];
            }
            a2[k] = if acc > 0.0 { acc } else { 0.0 };
            z += self.w3[k] * a2[k];
        }
        1.0 / (1.0 + (-z).exp())
    }
    pub fn value(&self, s: &State, h: u8) -> f32 {
        let mut x = [0f32; NF];
        let (mut a1, mut a2) = ([0f32; H1], [0f32; H2]);
        features(s, h, &mut x);
        self.forward(&x, &mut a1, &mut a2)
    }
    /// Add the gradient of the cross-entropy loss for one sample to `g`
    /// (same layout as `flat`). Returns the loss.
    pub fn grad(&self, x: &[f32; NF], y: f32, g: &mut [f32]) -> f32 {
        let (mut a1, mut a2) = ([0f32; H1], [0f32; H2]);
        let p = self.forward(x, &mut a1, &mut a2);
        let dz = p - y;
        let (o_b1, o_w2) = (NF * H1, NF * H1 + H1);
        let (o_b2, o_w3) = (o_w2 + H2 * H1, o_w2 + H2 * H1 + H2);
        let mut d1 = [0f32; H1];
        for k in 0..H2 {
            g[o_w3 + k] += dz * a2[k];
            if a2[k] > 0.0 {
                let dk = dz * self.w3[k];
                g[o_b2 + k] += dk;
                let row = &self.w2[k * H1..(k + 1) * H1];
                let grow = &mut g[o_w2 + k * H1..o_w2 + (k + 1) * H1];
                for j in 0..H1 {
                    grow[j] += dk * a1[j];
                    d1[j] += dk * row[j];
                }
            }
        }
        g[o_w3 + H2] += dz;
        for j in 0..H1 {
            if a1[j] <= 0.0 {
                d1[j] = 0.0;
            }
            g[o_b1 + j] += d1[j];
        }
        for i in 0..NF {
            let xi = x[i];
            if xi != 0.0 {
                let grow = &mut g[i * H1..(i + 1) * H1];
                for j in 0..H1 {
                    grow[j] += xi * d1[j];
                }
            }
        }
        let pc = p.clamp(1e-6, 1.0 - 1e-6);
        -(y * pc.ln() + (1.0 - y) * (1.0 - pc).ln())
    }
}

// --- players ------------------------------------------------------------------
/// The stack size at or below which play is exact.
pub const EXACT_FROM: u8 = 3;

#[derive(Clone, Copy, PartialEq)]
pub enum Player {
    Greedy,
    Look, // one reply ahead, as the web game's stopgap Abbot
    Net,
}

fn gain(s: &State, child: &State) -> i32 {
    -(child.diff as i32) - s.diff as i32
}

fn pick<T: Copy>(v: &[T], rng: &mut Rng) -> T {
    v[rng.below(v.len())]
}

/// Choose a move. Returns (move, value of the best move as the mover sees it
/// where one is known, else NaN).
pub fn choose(p: Player, s: &State, hand: &[u8], net: Option<&Net>, solver: &mut Solver2, rng: &mut Rng) -> (Move, f64) {
    if s.remaining() <= EXACT_FROM && s.mask != 0 {
        let (v, m) = solver.best(s, hand);
        return (m, v);
    }
    let opts = options(s, hand);
    match p {
        Player::Greedy => {
            let top = opts.iter().map(|o| gain(s, &o.1)).max().unwrap();
            let best: Vec<Move> = opts.iter().filter(|o| gain(s, &o.1) == top).map(|o| o.0).collect();
            (pick(&best, rng), f64::NAN)
        }
        Player::Look => {
            let (mut top, mut best) = (-99.0, Vec::new());
            for (m, child, other) in &opts {
                let rem = child.remaining();
                let mut reply = 0.0;
                let best_gain = |c: &State, hand: &[u8]| options(c, hand).iter().map(|o| gain(c, &o.1)).max().unwrap() as f64;
                if rem > 0 {
                    for d in 0..5u8 {
                        let cnt = child.counts[d as usize];
                        if cnt == 0 {
                            continue;
                        }
                        let mut n = *child;
                        n.counts[d as usize] -= 1;
                        let h: Vec<u8> = if *other == NONE { vec![d] } else { vec![*other, d] };
                        reply += cnt as f64 / rem as f64 * best_gain(&n, &h);
                    }
                } else if *other != NONE {
                    reply = best_gain(child, &[*other]);
                }
                let val = gain(s, child) as f64 - reply;
                if val > top + 1e-9 {
                    top = val;
                    best.clear();
                }
                if (val - top).abs() <= 1e-9 {
                    best.push(*m);
                }
            }
            (pick(&best, rng), f64::NAN)
        }
        Player::Net => {
            let net = net.expect("net player needs a net");
            let (mut top, mut mv) = (-1.0, opts[0].0);
            for (m, child, other) in &opts {
                let q = 1.0 - net.value(child, *other) as f64;
                if q > top {
                    top = q;
                    mv = *m;
                }
            }
            (mv, top)
        }
    }
}

pub fn draw(s: &mut State, rng: &mut Rng) -> u8 {
    let mut k = rng.below(s.remaining() as usize) as u8;
    let mut t = 0;
    while k >= s.counts[t] {
        k -= s.counts[t];
        t += 1;
    }
    s.counts[t] -= 1;
    t as u8
}

/// Play one whole game. Returns true if the first player wins.
pub fn play_game(first: Player, second: Player, net: Option<&Net>, solver: &mut Solver2, rng: &mut Rng) -> bool {
    play_game2(first, second, net, net, solver, rng)
}

/// As `play_game`, with a separate net for each seat.
pub fn play_game2(first: Player, second: Player, net1: Option<&Net>, net2: Option<&Net>, solver: &mut Solver2, rng: &mut Rng) -> bool {
    let mut s = State::start();
    let mut hand = vec![draw(&mut s, rng), draw(&mut s, rng)];
    for turn in 0..16 {
        let (p, net) = if turn % 2 == 0 { (first, net1) } else { (second, net2) };
        let (m, _) = choose(p, &s, &hand, net, solver, rng);
        let child = put(&s, hand[m.w], m.r, m.x, m.y);
        let mut next: Vec<u8> = if hand.len() == 2 { vec![swap_ab(hand[1 - m.w])] } else { vec![] };
        s = child;
        if s.remaining() > 0 {
            next.push(draw(&mut s, rng));
        }
        hand = next;
    }
    // board full: the view is the first player's
    s.diff >= 0
}

/// A random position reached by random play, stopped when `stack` tiles are
/// left face down and the mover has just drawn. Returns (position, hand).
pub fn random_decision(rng: &mut Rng, stack: u8) -> (State, Vec<u8>) {
    let mut s = State::start();
    let mut hand = vec![draw(&mut s, rng), draw(&mut s, rng)];
    while s.remaining() > stack {
        let opts = options(&s, &hand);
        let (m, child, other) = opts[rng.below(opts.len())];
        let _ = m;
        s = child;
        hand = vec![other, draw(&mut s, rng)];
    }
    (s, hand)
}
