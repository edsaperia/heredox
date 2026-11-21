//! Self-play trainer for the two-tile game.
//!
//!   heredox-train train <threads> <hours>   train; resumes from net.txt if present
//!   heredox-train eval <games>              net.txt against the one-reply-ahead player
//!   heredox-train selfcheck                 solver with memo against plain search
//!   heredox-train dump <stack> <n>          positions with exact values (JSON lines)
//!   heredox-train dumpnet <n>               positions with net.txt's values (JSON lines)
//!
//! The net judges positions with more than EXACT_FROM tiles face down; below
//! that play is exact, and those exact values are what training is anchored to.
use heredox::pos::{Rng, State};
use heredox::two::*;
use std::fs;
use std::io::Write;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy)]
struct Sample {
    s: State,
    h: u8,
    y: f32,
}

fn seed(extra: u64) -> u64 {
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
    (t ^ extra.wrapping_mul(0x9e3779b97f4a7c15)) | 1
}

/// One training game. Stops as soon as the rest can be solved exactly.
fn selfplay(net: &Net, solver: &mut Solver2, rng: &mut Rng, eps: f64, lambda: f64, out: &mut Vec<Sample>) {
    let mut s = State::start();
    let mut hand = vec![draw(&mut s, rng), draw(&mut s, rng)];
    // (position after the move, tile left, best value the mover saw, explored?)
    let mut recs: Vec<(State, u8, f64, bool)> = Vec::new();
    let last_y;
    loop {
        let opts = options(&s, &hand);
        let (mut top, mut best) = (-1.0, 0);
        for (i, (_, child, other)) in opts.iter().enumerate() {
            let q = 1.0 - net.value(child, *other) as f64;
            if q > top {
                top = q;
                best = i;
            }
        }
        let mut pickd = best;
        if (rng.next() >> 11) as f64 / (1u64 << 53) as f64 <= eps {
            pickd = rng.below(opts.len());
        }
        let (_, child, other) = opts[pickd];
        recs.push((child, other, top, pickd != best));
        if child.remaining() <= EXACT_FROM + 1 {
            // after the next draw everything is exact, so this position's
            // true value can be computed outright
            last_y = solver.solve(&child, other);
            break;
        }
        s = child;
        hand = vec![other, draw(&mut s, rng)];
    }
    let n = recs.len();
    let mut ys = vec![0.0f64; n];
    ys[n - 1] = last_y;
    for t in (0..n - 1).rev() {
        let (_, _, next_top, next_explored) = recs[t + 1];
        // value to the player who moves next: what they saw, blended with
        // what actually followed (unless they then explored)
        let followed = if next_explored { next_top } else { 1.0 - ys[t + 1] };
        ys[t] = (1.0 - lambda) * next_top + lambda * followed;
    }
    for t in 0..n {
        out.push(Sample { s: recs[t].0, h: recs[t].1, y: ys[t] as f32 });
    }
}

fn augment(smp: &Sample, rng: &mut Rng) -> (State, u8) {
    let mut s = smp.s;
    let mut h = smp.h;
    for _ in 0..rng.below(4) {
        s = s.rot90();
    }
    if rng.below(2) == 1 {
        s = s.mirror();
        h = swap_ab(h);
    }
    (s, h)
}

fn save(net: &Net, games: u64) {
    let flat = net.flat();
    let mut txt = format!("heredox-net 1 {} {} {} {}\n", NF, H1, H2, games);
    for v in &flat {
        txt.push_str(&format!("{:e}\n", v));
    }
    fs::write("net.tmp", txt).unwrap();
    fs::rename("net.tmp", "net.txt").unwrap();
    let nums: Vec<String> = flat.iter().map(|v| format!("{:.5}", v)).collect();
    let json = format!(
        "{{\"nf\":{},\"h1\":{},\"h2\":{},\"exactFrom\":{},\"games\":{},\"w\":[{}]}}\n",
        NF, H1, H2, EXACT_FROM, games, nums.join(",")
    );
    fs::write("net.json", json).unwrap();
}

fn load() -> Option<(Net, u64)> {
    let txt = fs::read_to_string("net.txt").ok()?;
    let mut lines = txt.lines();
    let head: Vec<&str> = lines.next()?.split_whitespace().collect();
    if head.len() < 6 || head[0] != "heredox-net" || head[2] != NF.to_string() || head[3] != H1.to_string() || head[4] != H2.to_string() {
        return None;
    }
    let games: u64 = head[5].parse().ok()?;
    let flat: Vec<f32> = lines.filter_map(|l| l.trim().parse().ok()).collect();
    let mut net = Net::new(&mut Rng(1));
    if flat.len() != net.n_params() {
        return None;
    }
    net.set_flat(&flat);
    Some((net, games))
}

/// `n` games in each seat between `net` and an opponent: another net if
/// `other` is given, else the one-reply-ahead player.
/// Returns (net's wins moving first, net's wins moving second).
fn evaluate(net: &Arc<Net>, other: Option<&Arc<Net>>, threads: usize, n: usize) -> (usize, usize) {
    let per = (n + threads - 1) / threads;
    let mut tot = (0, 0);
    std::thread::scope(|sc| {
        let hs: Vec<_> = (0..threads)
            .map(|i| {
                let net = Arc::clone(net);
                let other = other.map(Arc::clone);
                sc.spawn(move || {
                    let mut rng = Rng(seed(1000 + i as u64));
                    let mut solver = Solver2::default();
                    let (mut a, mut b) = (0, 0);
                    let opp = if other.is_some() { Player::Net } else { Player::Look };
                    let on = other.as_deref();
                    for _ in 0..per {
                        if play_game2(Player::Net, opp, Some(&net), on, &mut solver, &mut rng) {
                            a += 1;
                        }
                        if !play_game2(opp, Player::Net, on, Some(&net), &mut solver, &mut rng) {
                            b += 1;
                        }
                        if solver.memo.len() > 3_000_000 {
                            solver.memo.clear();
                        }
                    }
                    (a, b)
                })
            })
            .collect();
        for h in hs {
            let (a, b) = h.join().unwrap();
            tot.0 += a;
            tot.1 += b;
        }
    });
    (tot.0 * n / (per * threads), tot.1 * n / (per * threads))
}

fn log(line: &str) {
    println!("{}", line);
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open("train.log") {
        let _ = writeln!(f, "{}", line);
    }
}

fn train(threads: usize, hours: f64) {
    let t0 = Instant::now();
    let (mut net, mut games) = match load() {
        Some((n, g)) => {
            log(&format!("resuming from net.txt at {} games", g));
            (n, g)
        }
        None => (Net::new(&mut Rng(seed(7))), 0),
    };
    let np = net.n_params();
    let (mut am, mut av) = (vec![0f32; np], vec![0f32; np]);
    let mut step: u64 = 0;
    let mut buf: Vec<Sample> = Vec::new();
    let cap = 300_000;
    let mut wpos = 0;
    let mut rng = Rng(seed(99));
    let games_per_thread = 25;
    let batch = 256;
    let mut round = 0u64;
    let mut loss_avg = 0.0f64;
    // earlier versions of the net, to measure progress once the fixed opponent is beaten
    let mut history: Vec<Arc<Net>> = vec![Arc::new(net.clone())];
    let mut older = Arc::clone(&history[0]);
    let mut checks = 0usize;
    log(&format!("training on {} threads for {} hours; {} weights", threads, hours, np));
    loop {
        let frac = (t0.elapsed().as_secs_f64() / 3600.0 / hours).min(1.0);
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
                        let mut solver = Solver2::default();
                        let mut out = Vec::new();
                        for _ in 0..games_per_thread {
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
        games += (threads * games_per_thread) as u64;
        let n_new = fresh.len();
        for smp in fresh {
            if buf.len() < cap {
                buf.push(smp);
            } else {
                buf[wpos] = smp;
                wpos = (wpos + 1) % cap;
            }
        }
        // each new sample is seen about four times
        let steps = (n_new * 4 / batch).max(1);
        let mut flat = net.flat();
        let mut g = vec![0f32; np];
        for _ in 0..steps {
            g.iter_mut().for_each(|v| *v = 0.0);
            let mut loss = 0.0;
            let mut x = [0f32; NF];
            for _ in 0..batch {
                let smp = buf[rng.below(buf.len())];
                let (s, h) = augment(&smp, &mut rng);
                features(&s, h, &mut x);
                loss += net.grad(&x, smp.y, &mut g);
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
            loss_avg = if loss_avg == 0.0 { loss as f64 / batch as f64 } else { 0.99 * loss_avg + 0.01 * loss as f64 / batch as f64 };
        }
        round += 1;
        let done = t0.elapsed().as_secs_f64() / 3600.0 >= hours;
        if round % 40 == 0 || done {
            save(&net, games);
            let cur = Arc::new(net.clone());
            let (a, b) = evaluate(&cur, None, threads, 200);
            let (c, d) = evaluate(&cur, Some(&older), threads, 200);
            let (e, _) = evaluate(&cur, Some(&cur), threads, 200);
            log(&format!(
                "{:6.2} h {:8} games  loss {:.4}  v one-reply-ahead {:3}+{:3} of 400  v its self of {} checks earlier {:3}+{:3} of 400  first seat wins {:3} of 200 in self-play",
                t0.elapsed().as_secs_f64() / 3600.0, games, loss_avg, a, b, checks.min(10), c, d, e
            ));
            checks += 1;
            history.push(cur);
            if history.len() > 10 {
                history.remove(0);
            }
            older = Arc::clone(&history[0]);
        }
        if done {
            break;
        }
    }
    log("finished; send back net.json");
}

fn state_json(s: &State) -> String {
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
    format!(
        "\"mask\":{},\"cells\":[{}],\"counts\":[{},{},{},{},{}],\"diff\":{}",
        s.mask, cells.join(","), s.counts[0], s.counts[1], s.counts[2], s.counts[3], s.counts[4], s.diff
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let num = |i: usize, d: f64| args.get(i).and_then(|s| s.parse::<f64>().ok()).unwrap_or(d);
    match args.get(1).map(|s| s.as_str()).unwrap_or("") {
        "train" => train(num(2, 4.0) as usize, num(3, 8.0)),
        "eval" => {
            let (net, games) = load().expect("no usable net.txt here");
            let n = num(2, 400.0) as usize;
            let (a, b) = evaluate(&Arc::new(net), None, 2, n);
            println!("net after {} games vs one-reply-ahead: wins {}/{} moving first, {}/{} moving second", games, a, n, b, n);
        }
        "selfcheck" => {
            let mut rng = Rng(0x5eed5eed);
            let mut worst: f64 = 0.0;
            for i in 0..200 {
                let (s, hand) = random_decision(&mut rng, (i % 3) as u8);
                let mut a = Solver2::default();
                let mut b = Solver2::default();
                b.use_memo = false;
                worst = worst.max((a.best(&s, &hand).0 - b.best(&s, &hand).0).abs());
            }
            println!("selfcheck: 200 two-tile endgames, memo against plain search, largest disagreement {:.2e}", worst);
        }
        "dump" => {
            let stack = num(2, 2.0) as u8;
            let mut rng = Rng(777 + stack as u64);
            let mut solver = Solver2::default();
            for _ in 0..num(3, 50.0) as usize {
                let (s, hand) = random_decision(&mut rng, stack);
                let v = solver.best(&s, &hand).0;
                println!("{{{},\"hand\":[{},{}],\"v\":{:.12}}}", state_json(&s), hand[0], hand[1], v);
            }
        }
        "dumpnet" => {
            let (net, _) = load().expect("no usable net.txt here");
            let mut rng = Rng(4321);
            for i in 0..num(2, 100.0) as usize {
                let (s, hand) = random_decision(&mut rng, 4 + (i % 10) as u8);
                let opts = options(&s, &hand);
                let (_, child, other) = opts[rng.below(opts.len())];
                println!("{{{},\"h\":{},\"v\":{:.7}}}", state_json(&child), other, net.value(&child, other));
            }
        }
        "speed" => {
            let net = Net::new(&mut Rng(5));
            let mut rng = Rng(seed(3));
            let mut solver = Solver2::default();
            let mut out = Vec::new();
            let t = Instant::now();
            let n = num(2, 50.0) as usize;
            for _ in 0..n {
                selfplay(&net, &mut solver, &mut rng, 0.1, 0.8, &mut out);
            }
            println!("{} training games in {:.1}s ({:.1} per second on one thread), memo {}", n, t.elapsed().as_secs_f64(), n as f64 / t.elapsed().as_secs_f64(), solver.memo.len());
        }
        _ => eprintln!("usage: heredox-train train <threads> <hours> | eval <games> | selfcheck | dump <stack> <n> | dumpnet <n> | speed <n>"),
    }
}
