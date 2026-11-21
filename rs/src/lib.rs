//! Heredox engine. Mirrors py/heredox.py; see RULES.md for conventions.
use std::collections::HashMap;

pub mod pos;
pub mod two;

pub const BLANK: u8 = 0;
pub const C: u8 = 1;
pub const V: u8 = 2;

pub const TYPES: [char; 5] = ['A', 'B', 'D', 'E', 'F'];
pub const TILES: [[u8; 4]; 5] = [
    [C, V, BLANK, BLANK],
    [C, BLANK, BLANK, V],
    [C, BLANK, V, BLANK],
    [C, V, V, C],
    [V, C, V, C],
];
pub const COUNTS: [u8; 5] = [3, 3, 2, 4, 4];
pub const DISTINCT_ROTATIONS: [u8; 5] = [4, 4, 4, 4, 2];
pub const DIRS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

pub fn type_index(c: char) -> usize {
    TYPES.iter().position(|&t| t == c).expect("unknown tile type")
}

/// Turn a tile r quarter-turns clockwise.
pub fn rotate(e: [u8; 4], r: u8) -> [u8; 4] {
    let mut out = [0u8; 4];
    for i in 0..4 {
        out[i] = e[(i + 4 - (r as usize % 4)) % 4];
    }
    out
}

pub struct Game {
    pub deck: Vec<usize>,
    pub first: u8, // C or V
    pub board: HashMap<(i32, i32), [u8; 4]>,
    pub moves: Vec<(i32, i32, u8)>,
}

impl Game {
    pub fn new(deck: &str, first: u8) -> Game {
        let deck: Vec<usize> = deck.chars().map(type_index).collect();
        assert_eq!(deck.len(), 16);
        Game { deck, first, board: HashMap::new(), moves: Vec::new() }
    }
    pub fn turn(&self) -> usize {
        self.moves.len()
    }
    pub fn over(&self) -> bool {
        self.turn() == 16
    }
    pub fn tile(&self) -> usize {
        self.deck[self.turn()]
    }
    pub fn to_move(&self) -> u8 {
        if self.turn() % 2 == 0 { self.first } else { 3 - self.first }
    }
    pub fn bounds(&self) -> (i32, i32, i32, i32) {
        let xs = self.board.keys().map(|k| k.0);
        let ys = self.board.keys().map(|k| k.1);
        (xs.clone().min().unwrap(), xs.max().unwrap(), ys.clone().min().unwrap(), ys.max().unwrap())
    }
    pub fn legal_cells(&self) -> Vec<(i32, i32)> {
        if self.board.is_empty() {
            return vec![(0, 0)];
        }
        let (x0, x1, y0, y1) = self.bounds();
        let mut cells: Vec<(i32, i32)> = Vec::new();
        for (&(x, y), _) in self.board.iter() {
            for (dx, dy) in DIRS {
                let c = (x + dx, y + dy);
                if self.board.contains_key(&c) || cells.contains(&c) {
                    continue;
                }
                if x1.max(c.0) - x0.min(c.0) > 3 || y1.max(c.1) - y0.min(c.1) > 3 {
                    continue;
                }
                cells.push(c);
            }
        }
        cells.sort_by_key(|c| (c.1, c.0));
        cells
    }
    pub fn legal_moves(&self) -> Vec<(i32, i32, u8)> {
        if self.over() {
            return vec![];
        }
        let n = DISTINCT_ROTATIONS[self.tile()];
        let mut out = Vec::new();
        for (x, y) in self.legal_cells() {
            for r in 0..n {
                out.push((x, y, r));
            }
        }
        out
    }
    pub fn play(&mut self, x: i32, y: i32, r: u8) -> Result<(), String> {
        if self.over() {
            return Err("game over".into());
        }
        if !self.legal_cells().contains(&(x, y)) || r > 3 {
            return Err(format!("illegal move {} {} {}", x, y, r));
        }
        self.board.insert((x, y), rotate(TILES[self.tile()], r));
        self.moves.push((x, y, r));
        Ok(())
    }
    /// (castian, veridican) complete symbols so far.
    pub fn score(&self) -> (u32, u32) {
        let mut s = [0u32; 3];
        for (&(x, y), e) in self.board.iter() {
            for d in [1usize, 2] {
                let (dx, dy) = DIRS[d];
                if let Some(n) = self.board.get(&(x + dx, y + dy)) {
                    if e[d] != BLANK && e[d] == n[(d + 2) % 4] {
                        s[e[d] as usize] += 1;
                    }
                }
            }
        }
        (s[1], s[2])
    }
}
