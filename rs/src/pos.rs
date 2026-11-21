//! Positions as the solvers see them: always from the point of view of the
//! player to move, whose symbol is written as 1.
use crate::{DISTINCT_ROTATIONS, TILES};
use std::hash::Hasher;

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

pub fn pack(e: [u8; 4]) -> u8 {
    e[0] | e[1] << 2 | e[2] << 4 | e[3] << 6
}
pub fn edge(c: u8, d: usize) -> u8 {
    (c >> (2 * d)) & 3
}
pub fn swap_sym(c: u8) -> u8 {
    // 1 <-> 2 in every 2-bit field, 0 stays 0
    let lo = c & 0x55;
    let hi = (c >> 1) & 0x55;
    hi | lo << 1
}

impl State {
    pub fn start() -> State {
        State { mask: 0, cells: [0; 16], counts: crate::COUNTS, diff: 0 }
    }
    pub fn remaining(&self) -> u8 {
        self.counts.iter().sum()
    }
    pub fn size(&self) -> (i32, i32) {
        let (mut w, mut h) = (0, 0);
        for i in 0..16 {
            if self.mask >> i & 1 == 1 {
                w = w.max(i as i32 % 4 + 1);
                h = h.max(i as i32 / 4 + 1);
            }
        }
        (w, h)
    }
    pub fn filled(&self, x: i32, y: i32) -> bool {
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
        let e = crate::rotate(TILES[t], r);
        let mut diff = self.diff as i32;
        for d in 0..4 {
            let (dx, dy) = crate::DIRS[d];
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

    pub fn normalise(&mut self) {
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
    pub fn rot90(&self) -> State {
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
    pub fn mirror(&self) -> State {
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

    pub fn raw_key(&self) -> (u64, u32) {
        let (w, h) = self.size();
        let mut edges: u64 = 0;
        let mut n = 0;
        for i in 0..16i32 {
            if self.mask >> i & 1 == 0 {
                continue;
            }
            let (x, y) = (i % 4, i / 4);
            for d in 0..4 {
                let (dx, dy) = crate::DIRS[d];
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

