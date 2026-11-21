//! Experiment: what happens to seat balance if making a split diocese (a
//! heredox) costs someone their choice of tile?
//!
//!   Rule::None    today's game
//!   Rule::Reward  make a split and your opponent's next turn has no choice:
//!                 they must play the tile you left them
//!   Rule::Cost    make a split and your own next turn has no choice: you
//!                 must play the tile your opponent leaves you
//! After a turn with no choice nothing is left over, so the next player draws
//! two. A restricted player who is left nothing draws one.
//!
//!   exp <none|reward|cost> <minutes> <threads> [games] [castian]
//! trains a net by self-play for that long, then plays it against itself.
//! With `castian`, level games go to Castian (the second player) instead of
//! Veridican. Results are appended to exp.log.
use heredox::pos::{Fx, Rng, State};
use heredox::two::{mover_is_first, placed, put, swap_ab, Move, NONE};
use heredox::{rotate, DIRS, DISTINCT_ROTATIONS, TILES};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const NF: usize = 168;
const H1: usize = 128;
const H2: usize = 64;
/// Play is exact once this few tiles remain unplaced.
const EXACT_UNPLACED: u32 = 5;
/// Level games go to the first player (Veridican) unless this is set.
static LEVEL_TO_SECOND: AtomicBool = AtomicBool::new(false);

/// Does the player to move win a finished game?
fn mover_wins(s: &State) -> bool {
    s.diff > 0 || (s.diff == 0 && (mover_is_first(s) != LEVEL_TO_SECOND.load(Ordering::Relaxed)))
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Rule {
    None,
    Reward,
    Cost,
}

/// A position before the mover takes their tiles.
#[derive(Clone, Copy)]
pub struct Pos {
    s: State,
    h: u8,    // tile left for the mover, or NONE
    rn: bool, // the mover has no choice this turn
    ra: bool, // the player after the mover will have no choice
}

fn features(p: &Pos, out: &mut [f32; NF]) {
    let mut base = [0f32; heredox::two::NF];
    heredox::two::features(&p.s, p.h, &mut base);
    out[..166].copy_from_slice(&base);
    out[166] = p.rn as u8 as f32;
    out[167] = p.ra as u8 as f32;
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
    pub fn value(&self, s: &Pos) -> f32 {
        let mut x = [0f32; NF];
        let (mut a1, mut a2) = ([0f32; H1], [0f32; H2]);
        features(s, &mut x);
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


fn made_split(s: &State, t: u8, r: u8, x: i32, y: i32) -> bool {
    let e = rotate(TILES[t as usize], r);
    for d in 0..4 {
        let (nx, ny) = (x + DIRS[d].0, y + DIRS[d].1);
        if e[d] != 0 && s.filled(nx, ny) {
            let o = (s.cells[(ny * 4 + nx) as usize] >> (2 * ((d + 2) % 4))) & 3;
            if o != 0 && o != e[d] {
                return true;
            }
        }
    }
    false
}

/// Every way the mover's tiles can turn out: (probability, stack after, hand).
fn deal(p: &Pos) -> Vec<(f64, State, Vec<u8>)> {
    let s = p.s;
    let rem = s.remaining() as f64;
    let one = |hand_first: Option<u8>| -> Vec<(f64, State, Vec<u8>)> {
        (0..5u8)
            .filter(|&d| s.counts[d as usize] > 0)
            .map(|d| {
                let mut n = s;
                n.counts[d as usize] -= 1;
                let hand = match hand_first {
                    Some(h) => vec![h, d],
                    None => vec![d],
                };
                (s.counts[d as usize] as f64 / rem, n, hand)
            })
            .collect()
    };
    if p.rn {
        return if p.h != NONE { vec![(1.0, s, vec![p.h])] } else if rem > 0.0 { one(None) } else { vec![] };
    }
    if p.h != NONE {
        return if rem == 0.0 { vec![(1.0, s, vec![p.h])] } else { one(Some(p.h)) };
    }
    if rem >= 2.0 {
        let mut out = Vec::new();
        for a in 0..5u8 {
            for b in a..5u8 {
                let (ca, cb) = (s.counts[a as usize] as f64, s.counts[b as usize] as f64);
                let pr = if a == b { ca * (ca - 1.0) } else { 2.0 * ca * cb } / (rem * (rem - 1.0));
                if pr > 0.0 {
                    let mut n = s;
                    n.counts[a as usize] -= 1;
                    n.counts[b as usize] -= 1;
                    out.push((pr, n, vec![a, b]));
                }
            }
        }
        return out;
    }
    if rem == 1.0 { one(None) } else { vec![] }
}

fn options(s: &State, hand: &[u8], ra_in: bool, rule: Rule) -> Vec<(Move, Pos, bool)> {
    let mut out = Vec::new();
    for (w, &t) in hand.iter().enumerate() {
        if w == 1 && hand[0] == hand[1] {
            continue;
        }
        let other = if hand.len() == 2 { swap_ab(hand[1 - w]) } else { NONE };
        let rots = if s.mask == 0 { 1 } else { DISTINCT_ROTATIONS[t as usize] };
        for (x, y) in s.legal_cells() {
            for r in 0..rots {
                let split = made_split(s, t, r, x, y);
                let child = Pos { s: put(s, t, r, x, y), h: other, rn: ra_in || (rule == Rule::Reward && split), ra: rule == Rule::Cost && split };
                out.push((Move { w, r, x, y }, child, split));
            }
        }
    }
    out
}

fn key(p: &Pos) -> (u64, u32, u8) {
    let flags = (p.rn as u8) << 4 | (p.ra as u8) << 5;
    let mut best = (u64::MAX, u32::MAX, u8::MAX);
    let mut t = p.s;
    for _ in 0..4 {
        let a = t.raw_key();
        best = best.min((a.0, a.1, p.h | flags));
        let b = t.mirror().raw_key();
        best = best.min((b.0, b.1, swap_ab(p.h) | flags));
        t = t.rot90();
    }
    best
}

struct Solver {
    memo: HashMap<(u64, u32, u8), f64, BuildHasherDefault<Fx>>,
    rule: Rule,
    use_memo: bool,
}

impl Solver {
    fn new(rule: Rule) -> Solver {
        Solver { memo: HashMap::default(), rule, use_memo: true }
    }
    fn best(&mut self, s: &State, hand: &[u8], ra: bool) -> (f64, Move, Pos, bool) {
        let mut best: Option<(f64, Move, Pos, bool)> = None;
        for (m, child, split) in options(s, hand, ra, self.rule) {
            let v = 1.0 - self.solve(&child);
            if best.map_or(true, |b| v > b.0 + 1e-12) {
                best = Some((v, m, child, split));
            }
        }
        best.unwrap()
    }
    /// P(mover wins).
    fn solve(&mut self, p: &Pos) -> f64 {
        let outcomes = deal(p);
        if outcomes.is_empty() {
            return if mover_wins(&p.s) { 1.0 } else { 0.0 };
        }
        let k = if self.use_memo { key(p) } else { (0, 0, 0) };
        if self.use_memo {
            if let Some(&v) = self.memo.get(&k) {
                return v;
            }
        }
        let mut acc = 0.0;
        for (pr, st, hand) in outcomes {
            acc += pr * self.best(&st, &hand, p.ra).0;
        }
        if self.use_memo {
            self.memo.insert(k, acc);
        }
        acc
    }
}

fn seed(extra: u64) -> u64 {
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
    (t ^ extra.wrapping_mul(0x9e3779b97f4a7c15)) | 1
}
fn unit(rng: &mut Rng) -> f64 {
    (rng.next() >> 11) as f64 / (1u64 << 53) as f64
}
fn sample(p: &Pos, rng: &mut Rng) -> Option<(State, Vec<u8>)> {
    let outcomes = deal(p);
    if outcomes.is_empty() {
        return None;
    }
    let mut u = unit(rng);
    for (pr, st, hand) in &outcomes {
        if u < *pr {
            return Some((*st, hand.clone()));
        }
        u -= pr;
    }
    let l = outcomes.last().unwrap();
    Some((l.1, l.2.clone()))
}

#[derive(Clone, Copy)]
struct Sample {
    p: Pos,
    y: f32,
}

fn selfplay(net: &Net, solver: &mut Solver, rng: &mut Rng, eps: f64, lambda: f64, out: &mut Vec<Sample>) {
    let rule = solver.rule;
    let mut pos = Pos { s: State::start(), h: NONE, rn: false, ra: false };
    let mut recs: Vec<(Pos, f64, bool)> = Vec::new();
    let last_y;
    loop {
        let (s, hand) = sample(&pos, rng).unwrap();
        let opts = options(&s, &hand, pos.ra, rule);
        let (mut top, mut best) = (-1.0, 0);
        for (i, (_, child, _)) in opts.iter().enumerate() {
            let q = 1.0 - net.value(child) as f64;
            if q > top {
                top = q;
                best = i;
            }
        }
        let mut pick = best;
        if unit(rng) <= eps {
            pick = rng.below(opts.len());
        }
        let child = opts[pick].1;
        recs.push((child, top, pick != best));
        if 16 - placed(&child.s) <= EXACT_UNPLACED {
            last_y = solver.solve(&child);
            break;
        }
        pos = child;
    }
    let n = recs.len();
    let mut ys = vec![0.0f64; n];
    ys[n - 1] = last_y;
    for t in (0..n - 1).rev() {
        let (_, next_top, next_explored) = recs[t + 1];
        let followed = if next_explored { next_top } else { 1.0 - ys[t + 1] };
        ys[t] = (1.0 - lambda) * next_top + lambda * followed;
    }
    for t in 0..n {
        out.push(Sample { p: recs[t].0, y: ys[t] as f32 });
    }
}

fn augment(smp: &Sample, rng: &mut Rng) -> Pos {
    let mut p = smp.p;
    for _ in 0..rng.below(4) {
        p.s = p.s.rot90();
    }
    if rng.below(2) == 1 {
        p.s = p.s.mirror();
        p.h = swap_ab(p.h);
    }
    p
}

struct Tally {
    first_wins: usize,
    games: usize,
    split_moves: usize,   // moves that made a split diocese
    forced_turns: usize,  // turns played with no choice of tile (not counting the last tile)
    splits_at_end: usize, // split dioceses on the finished floor
    symbols: usize,       // dioceses held by either side at the end
    level: usize,         // games that finished level
}

fn count_joins(s: &State) -> (usize, usize) {
    let (mut held, mut split) = (0, 0);
    for i in 0..16i32 {
        for d in [1usize, 2] {
            let (nx, ny) = (i % 4 + DIRS[d].0, i / 4 + DIRS[d].1);
            if !(0..4).contains(&nx) || !(0..4).contains(&ny) {
                continue;
            }
            let a = (s.cells[i as usize] >> (2 * d)) & 3;
            let b = (s.cells[(ny * 4 + nx) as usize] >> (2 * ((d + 2) % 4))) & 3;
            if a != 0 && b != 0 {
                if a == b { held += 1 } else { split += 1 }
            }
        }
    }
    (held, split)
}

/// The net against itself, greedy, exact at the end.
fn play(net: &Net, solver: &mut Solver, rng: &mut Rng, t: &mut Tally) {
    let rule = solver.rule;
    let mut pos = Pos { s: State::start(), h: NONE, rn: false, ra: false };
    loop {
        let (s, hand) = match sample(&pos, rng) {
            Some(x) => x,
            None => break,
        };
        if pos.rn && 16 - placed(&s) > 1 {
            t.forced_turns += 1;
        }
        let (child, split) = if 16 - placed(&s) <= EXACT_UNPLACED && s.mask != 0 {
            let b = solver.best(&s, &hand, pos.ra);
            (b.2, b.3)
        } else {
            let opts = options(&s, &hand, pos.ra, rule);
            let mut best = (-1.0, 0);
            for (i, (_, c, _)) in opts.iter().enumerate() {
                let q = 1.0 - net.value(c) as f64;
                if q > best.0 {
                    best = (q, i);
                }
            }
            (opts[best.1].1, opts[best.1].2)
        };
        if split {
            t.split_moves += 1;
        }
        pos = child;
    }
    // board full: the view is the first player's
    if mover_wins(&pos.s) {
        t.first_wins += 1;
    }
    if pos.s.diff == 0 {
        t.level += 1;
    }
    t.games += 1;
    let (held, split) = count_joins(&pos.s);
    t.symbols += held;
    t.splits_at_end += split;
}

fn self_play_tally(net: &Arc<Net>, rule: Rule, threads: usize, eval_games: usize) -> Tally {
    let shared = Arc::clone(net);
    let mut total = Tally { first_wins: 0, games: 0, split_moves: 0, forced_turns: 0, splits_at_end: 0, symbols: 0, level: 0 };
    std::thread::scope(|sc| {
        let hs: Vec<_> = (0..threads)
            .map(|i| {
                let net = Arc::clone(&shared);
                sc.spawn(move || {
                    let mut rng = Rng(seed(5000 + i as u64));
                    let mut solver = Solver::new(rule);
                    let mut t = Tally { first_wins: 0, games: 0, split_moves: 0, forced_turns: 0, splits_at_end: 0, symbols: 0, level: 0 };
                    for _ in 0..eval_games / threads {
                        play(&net, &mut solver, &mut rng, &mut t);
                        if solver.memo.len() > 3_000_000 {
                            solver.memo.clear();
                        }
                    }
                    t
                })
            })
            .collect();
        for h in hs {
            let t = h.join().unwrap();
            total.first_wins += t.first_wins;
            total.games += t.games;
            total.split_moves += t.split_moves;
            total.forced_turns += t.forced_turns;
            total.splits_at_end += t.splits_at_end;
            total.symbols += t.symbols;
            total.level += t.level;
        }
    });
    total
}

fn report(rule: Rule, games: u64, hours: f64, t: &Tally) -> String {
    let n = t.games as f64;
    format!(
        "{:?}{} {:5.2} h {:8} training games | self-play {} games: Veridican (first) wins {:.1}%, level {:.1}% | per game: {:.1} moves split a diocese, {:.1} turns with no choice, {:.1} held, {:.1} split",
        rule, if LEVEL_TO_SECOND.load(Ordering::Relaxed) { " (level games to Castian)" } else { "" }, hours, games, t.games,
        100.0 * t.first_wins as f64 / n, 100.0 * t.level as f64 / n, t.split_moves as f64 / n, t.forced_turns as f64 / n, t.symbols as f64 / n, t.splits_at_end as f64 / n
    )
}
fn log(line: &str) {
    use std::io::Write;
    println!("{}", line);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("exp.log") {
        let _ = writeln!(f, "{}", line);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rule = match args.get(1).map(|s| s.as_str()) {
        Some("none") => Rule::None,
        Some("reward") => Rule::Reward,
        Some("cost") => Rule::Cost,
        Some("selfcheck") => {
            for rule in [Rule::None, Rule::Reward, Rule::Cost] {
                let mut rng = Rng(99);
                let net = Net::new(&mut Rng(3));
                let mut worst: f64 = 0.0;
                for _ in 0..40 {
                    // random play down to four unplaced tiles, then compare memo with plain search
                    let mut pos = Pos { s: State::start(), h: NONE, rn: false, ra: false };
                    while 16 - placed(&pos.s) > 4 {
                        let (s, hand) = sample(&pos, &mut rng).unwrap();
                        let opts = options(&s, &hand, pos.ra, rule);
                        pos = opts[rng.below(opts.len())].1;
                    }
                    let _ = &net;
                    let (mut a, mut b) = (Solver::new(rule), Solver::new(rule));
                    b.use_memo = false;
                    worst = worst.max((a.solve(&pos) - b.solve(&pos)).abs());
                    if rule == Rule::None {
                        // with no rule this must agree with the game's own solver
                        let v = heredox::two::Solver2::default().solve(&pos.s, pos.h);
                        worst = worst.max((a.solve(&pos) - v).abs());
                    }
                }
                println!("{:?}: 40 endgames, memo against plain search, largest disagreement {:.2e}", rule, worst);
            }
            return;
        }
        _ => {
            eprintln!("usage: exp <none|reward|cost> <minutes> <threads> [games] | exp selfcheck");
            return;
        }
    };
    let minutes: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5.0);
    let threads: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(2);
    let eval_games: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(2000);
    if args.get(5).map(|s| s.as_str()) == Some("castian") {
        LEVEL_TO_SECOND.store(true, Ordering::Relaxed);
    }
    let mut next_check = 0.125;
    let t0 = Instant::now();
    let mut net = Net::new(&mut Rng(seed(7)));
    let np = net.n_params();
    let (mut am, mut av) = (vec![0f32; np], vec![0f32; np]);
    let (mut step, mut games, mut round) = (0u64, 0u64, 0u64);
    let mut buf: Vec<Sample> = Vec::new();
    let (cap, mut wpos, batch) = (300_000, 0, 256);
    let mut rng = Rng(seed(99));
    loop {
        let frac = (t0.elapsed().as_secs_f64() / 60.0 / minutes).min(1.0);
        if frac >= 1.0 {
            break;
        }
        if frac >= next_check {
            next_check += 0.125;
            let t = self_play_tally(&Arc::new(net.clone()), rule, threads, (eval_games / 10).max(100));
            log(&format!("      {}", report(rule, games, t0.elapsed().as_secs_f64() / 3600.0, &t)));
        }
        let eps = 0.15 - 0.10 * frac;
        let lr = if frac < 0.5 { 1e-3 } else if frac < 0.85 { 3e-4 } else { 1e-4 };
        let shared = Arc::new(net.clone());
        let mut fresh: Vec<Sample> = Vec::new();
        std::thread::scope(|sc| {
            let hs: Vec<_> = (0..threads)
                .map(|i| {
                    let net = Arc::clone(&shared);
                    sc.spawn(move || {
                        let mut rng = Rng(seed(i as u64 + round * 131));
                        let mut solver = Solver::new(rule);
                        let mut out = Vec::new();
                        for _ in 0..25 {
                            selfplay(&net, &mut solver, &mut rng, eps, 0.8, &mut out);
                        }
                        out
                    })
                })
                .collect();
            for h in hs {
                fresh.extend(h.join().unwrap());
            }
        });
        games += (threads * 25) as u64;
        round += 1;
        let n_new = fresh.len();
        for smp in fresh {
            if buf.len() < cap {
                buf.push(smp);
            } else {
                buf[wpos] = smp;
                wpos = (wpos + 1) % cap;
            }
        }
        let steps = (n_new * 4 / batch).max(1);
        let mut flat = net.flat();
        let mut g = vec![0f32; np];
        let mut x = [0f32; NF];
        for _ in 0..steps {
            g.iter_mut().for_each(|v| *v = 0.0);
            for _ in 0..batch {
                let smp = buf[rng.below(buf.len())];
                features(&augment(&smp, &mut rng), &mut x);
                net.grad(&x, smp.y, &mut g);
            }
            step += 1;
            let (b1, b2) = (0.9f32, 0.999f32);
            let c1 = 1.0 - b1.powi(step.min(10_000) as i32);
            let c2 = 1.0 - b2.powi(step.min(100_000) as i32);
            for i in 0..np {
                let gi = g[i] / batch as f32;
                am[i] = b1 * am[i] + (1.0 - b1) * gi;
                av[i] = b2 * av[i] + (1.0 - b2) * gi * gi;
                flat[i] -= lr * (am[i] / c1) / ((av[i] / c2).sqrt() + 1e-8);
            }
            net.set_flat(&flat);
        }
    }
    let total = self_play_tally(&Arc::new(net), rule, threads, eval_games);
    log(&format!("FINAL {}", report(rule, games, t0.elapsed().as_secs_f64() / 3600.0, &total)));
}
