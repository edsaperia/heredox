//! Replays transcripts (one JSON object per line on stdin) and checks every
//! move is legal, the legal-move list at each turn matches in size, and the
//! final score agrees. Prints one line per game: the count of legal moves at
//! each turn, so two engines can be diffed.
use heredox::*;
use std::io::{self, BufRead};

fn field<'a>(line: &'a str, name: &str) -> &'a str {
    let key = format!("\"{}\":", name);
    let start = line.find(&key).expect("field missing") + key.len();
    &line[start..]
}

fn numbers(s: &str) -> Vec<i32> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        if ch == '-' || ch.is_ascii_digit() {
            cur.push(ch);
        } else if !cur.is_empty() {
            out.push(cur.parse().unwrap());
            cur.clear();
        }
    }
    if !cur.is_empty() {
        out.push(cur.parse().unwrap());
    }
    out
}

fn main() {
    let stdin = io::stdin();
    let mut games = 0;
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }
        let deck: String = field(&line, "deck").chars().skip(1).take(16).collect();
        let first = if field(&line, "first").starts_with("\"C") { C } else { V };
        let m = field(&line, "moves");
        let m = &m[..m.find("]]").unwrap()];
        let mv = numbers(m);
        let sc = numbers(field(&line, "score"));
        let mut g = Game::new(&deck, first);
        let mut counts = Vec::new();
        for ch in mv.chunks(3) {
            let legal = g.legal_moves();
            counts.push(legal.len().to_string());
            assert!(legal.contains(&(ch[0], ch[1], ch[2] as u8)), "move not in legal list, game {}", games);
            g.play(ch[0], ch[1], ch[2] as u8).unwrap();
        }
        assert!(g.over());
        let (c, v) = g.score();
        assert_eq!((c as i32, v as i32), (sc[0], sc[1]), "score mismatch, game {}", games);
        println!("{}", counts.join(","));
        games += 1;
    }
    eprintln!("replayed {} games, all scores agree", games);
}
