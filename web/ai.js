// Heredox computer players: exact endgame search plus simple lookahead.
// Positions here use real symbols (1 Castian, 2 Veridican); diff is Castian
// minus Veridican. The value of a position is [P(Castian wins), P(Veridican wins)].
// Equal counts are a win for the player who moved first (s.first).
(function (root) {
  const H = typeof module !== 'undefined' && module.exports ? require('./engine.js') : root.Heredox;
  const T = H.TYPES.map(t => H.TILES[t]);
  const ROTS = H.TYPES.map(t => H.DISTINCT_ROTATIONS[t]);
  const DIRS = H.DIRS;
  // ROTATED[t][r] = edges
  const ROTATED = T.map(e => [0, 1, 2, 3].map(r => H.rotate(e, r)));

  // state: {mask, cells: Array(16) of edges|null, counts[5], diff, mover}
  function fromGame(g) {
    const s = { mask: 0, cells: new Array(16).fill(null), counts: [0, 0, 0, 0, 0], diff: 0, mover: g.toMove === 'C' ? 1 : 2, first: g.first === 'C' ? 1 : 2, ox: 0, oy: 0 };
    // single: the tile in hand counts as the next draw; two: the stack only
    for (let i = g.variant === 'two' ? g.pos : g.turn; i < 16; i++) s.counts[H.TYPES.indexOf(g.deck[i])]++;
    if (g.board.size) {
      const [x0, , y0] = g.bounds();
      s.ox = x0; s.oy = y0;
      for (const t of g.board.values()) {
        const j = (t.y - y0) * 4 + (t.x - x0);
        s.mask |= 1 << j;
        s.cells[j] = t.e;
      }
    }
    const [c, v] = g.score();
    s.diff = c - v;
    return s;
  }
  function size(s) {
    let w = 0, h = 0;
    for (let i = 0; i < 16; i++) if (s.mask >> i & 1) { w = Math.max(w, i % 4 + 1); h = Math.max(h, (i >> 2) + 1); }
    return [w, h];
  }
  const filled = (s, x, y) => x >= 0 && x < 4 && y >= 0 && y < 4 && (s.mask >> (y * 4 + x) & 1) === 1;

  function legalCells(s) {
    if (s.mask === 0) return [[0, 0]];
    const [w, h] = size(s), out = [];
    for (let y = -1; y <= Math.min(h, 3); y++) {
      for (let x = -1; x <= Math.min(w, 3); x++) {
        if (filled(s, x, y)) continue;
        if ((x === -1 || x === w) && w === 4) continue;
        if ((y === -1 || y === h) && h === 4) continue;
        if (filled(s, x - 1, y) || filled(s, x + 1, y) || filled(s, x, y - 1) || filled(s, x, y + 1)) out.push([x, y]);
      }
    }
    return out;
  }
  function gain(s, e, x, y) { // change in diff from placing edges e at (x, y)
    let g = 0;
    for (let d = 0; d < 4; d++) {
      if (e[d] === 0) continue;
      const nx = x + DIRS[d][0], ny = y + DIRS[d][1];
      if (filled(s, nx, ny) && s.cells[ny * 4 + nx][(d + 2) % 4] === e[d]) g += e[d] === 1 ? 1 : -1;
    }
    return g;
  }
  function place(s, t, r, x, y) {
    const e = ROTATED[t][r];
    const sx = x === -1 ? 1 : 0, sy = y === -1 ? 1 : 0;
    const n = { mask: 0, cells: new Array(16).fill(null), counts: s.counts.slice(), diff: s.diff + gain(s, e, x, y), mover: 3 - s.mover, first: s.first, ox: s.ox - sx, oy: s.oy - sy };
    if (sx || sy) {
      for (let i = 0; i < 16; i++) if (s.mask >> i & 1) {
        const j = ((i >> 2) + sy) * 4 + i % 4 + sx;
        n.mask |= 1 << j; n.cells[j] = s.cells[i];
      }
    } else { n.mask = s.mask; n.cells = s.cells.slice(); }
    const j = (y + sy) * 4 + x + sx;
    n.mask |= 1 << j; n.cells[j] = e;
    n.counts[t]--;
    return n;
  }
  function key(s) {
    const [w, h] = size(s);
    let edges = '';
    for (let i = 0; i < 16; i++) {
      if (!(s.mask >> i & 1)) continue;
      const x = i % 4, y = i >> 2;
      for (let d = 0; d < 4; d++) {
        const nx = x + DIRS[d][0], ny = y + DIRS[d][1];
        if (filled(s, nx, ny)) continue;
        const open = (DIRS[d][0] === 0 || (nx >= 0 && nx < 4) || w < 4) && (DIRS[d][1] === 0 || (ny >= 0 && ny < 4) || h < 4);
        edges += open ? s.cells[i][d] : 0;
      }
    }
    return s.mask + '|' + edges + '|' + s.counts.join('') + '|' + s.diff + '|' + s.mover + s.first;
  }
  const remaining = s => s.counts[0] + s.counts[1] + s.counts[2] + s.counts[3] + s.counts[4];
  const EPS = 1e-12;
  // is value a better than b for the given mover? own win first, then the other's
  function better(mover, a, b) {
    const m = mover - 1, o = 1 - m;
    return a[m] > b[m] + EPS || (Math.abs(a[m] - b[m]) <= EPS && a[o] < b[o] - EPS);
  }

  class Solver {
    constructor() { this.memo = new Map(); }
    bestForTile(s, t) {
      let best = null, move = null;
      const rots = s.mask === 0 ? 1 : ROTS[t];
      for (const [x, y] of legalCells(s)) {
        for (let r = 0; r < rots; r++) {
          const v = this.solve(place(s, t, r, x, y));
          if (best === null || better(s.mover, v, best)) { best = v; move = [x, y, r]; }
        }
      }
      return { value: best, move };
    }
    solve(s) {
      const rem = remaining(s);
      if (rem === 0) return s.diff > 0 || (s.diff === 0 && s.first === 1) ? [1, 0] : [0, 1];
      const kk = key(s);
      const hit = this.memo.get(kk);
      if (hit) return hit;
      const acc = [0, 0];
      for (let t = 0; t < 5; t++) {
        if (!s.counts[t]) continue;
        const p = s.counts[t] / rem, v = this.bestForTile(s, t).value;
        acc[0] += p * v[0]; acc[1] += p * v[1];
      }
      this.memo.set(kk, acc);
      return acc;
    }
  }

  // --- move choice ------------------------------------------------------
  function pick(list, rand) { return list[Math.floor(rand() * list.length)]; }
  const sign = s => (s.mover === 1 ? 1 : -1);

  // every move with its immediate gain for the mover
  function scoredMoves(s, t) {
    const out = [];
    for (const [x, y] of legalCells(s)) for (let r = 0; r < ROTS[t]; r++) out.push({ x, y, r, g: sign(s) * gain(s, ROTATED[t][r], x, y) });
    return out;
  }
  function greedy(s, t, rand) {
    const ms = scoredMoves(s, t), top = Math.max(...ms.map(m => m.g));
    return pick(ms.filter(m => m.g === top), rand);
  }
  // my gain now, less the opponent's best immediate reply averaged over their draw
  function lookahead(s, t, rand) {
    const ms = scoredMoves(s, t);
    let best = -99, top = [];
    for (const m of ms) {
      const n = place(s, t, m.r, m.x, m.y), rem = remaining(n);
      let reply = 0;
      if (rem) for (let u = 0; u < 5; u++) {
        if (!n.counts[u]) continue;
        reply += n.counts[u] / rem * Math.max(...scoredMoves(n, u).map(q => q.g));
      }
      const val = m.g - reply;
      if (val > best + 1e-9) { best = val; top = [m]; } else if (Math.abs(val - best) <= 1e-9) top.push(m);
    }
    return pick(top, rand);
  }

  // --- two in hand --------------------------------------------------------
  // Here s.counts is the face-down stack only. The mover holds the tile they
  // were handed (h, or -1) and, if the stack is not empty, draws one more.
  function put(s, t, r, x, y) {           // place a hand tile: the stack is untouched
    const n = place(s, t, r, x, y);
    n.counts[t]++;
    return n;
  }
  class Solver2 {
    constructor() { this.memo = new Map(); }
    // best play holding tiles `hand` (type indices, 1 or 2 of them)
    best(s, hand) {
      let best = null, move = null;
      const seen = new Set();
      hand.forEach((t, w) => {
        if (seen.has(t)) return;
        seen.add(t);
        const other = hand.length === 2 ? hand[1 - w] : -1;
        for (const [x, y] of legalCells(s)) {
          for (let r = 0; r < ROTS[t]; r++) {
            const v = this.solve(put(s, t, r, x, y), other);
            if (best === null || better(s.mover, v, best)) { best = v; move = [x, y, r, w]; }
          }
        }
      });
      return { value: best, move };
    }
    // value before the mover draws, holding handed tile h (-1 for none)
    solve(s, h) {
      const rem = remaining(s);
      if (rem === 0 && h < 0) return s.diff > 0 || (s.diff === 0 && s.first === 1) ? [1, 0] : [0, 1];
      const kk = key(s) + '#' + h;
      const hit = this.memo.get(kk);
      if (hit) return hit;
      let acc;
      if (rem === 0) acc = this.best(s, [h]).value;
      else {
        acc = [0, 0];
        for (let d = 0; d < 5; d++) {
          if (!s.counts[d]) continue;
          const p = s.counts[d] / rem;
          const n = Object.assign({}, s, { counts: s.counts.slice() });
          n.counts[d]--;
          const v = this.best(n, h < 0 ? [d] : [h, d]).value;
          acc[0] += p * v[0]; acc[1] += p * v[1];
        }
      }
      this.memo.set(kk, acc);
      return acc;
    }
  }
  function scoredMoves2(s, hand) {
    const out = [];
    hand.forEach((t, w) => {
      for (const [x, y] of legalCells(s)) for (let r = 0; r < ROTS[t]; r++) out.push({ x, y, r, w, t, g: sign(s) * gain(s, ROTATED[t][r], x, y) });
    });
    return out;
  }
  function greedy2(s, hand, rand) {
    const ms = scoredMoves2(s, hand), top = Math.max(...ms.map(m => m.g));
    return pick(ms.filter(m => m.g === top), rand);
  }
  // my gain now, less the opponent's best immediate reply with the tile I hand
  // them plus whatever they draw
  function lookahead2(s, hand, rand) {
    let best = -99, top = [];
    for (const m of scoredMoves2(s, hand)) {
      const n = put(s, m.t, m.r, m.x, m.y), rem = remaining(n);
      const other = hand.length === 2 ? [hand[1 - m.w]] : [];
      let reply = 0;
      if (rem) for (let d = 0; d < 5; d++) {
        if (!n.counts[d]) continue;
        reply += n.counts[d] / rem * Math.max(...scoredMoves2(n, other.concat([d])).map(q => q.g));
      } else if (other.length) reply = Math.max(...scoredMoves2(n, other).map(q => q.g));
      const val = m.g - reply;
      if (val > best + 1e-9) { best = val; top = [m]; } else if (Math.abs(val - best) <= 1e-9) top.push(m);
    }
    return pick(top, rand);
  }

  // exactFrom: tiles left to place (single) / tiles left in the stack (two)
  const LEVELS = {
    novice: { exactFrom: 0, early: greedy, exactFrom2: -1, early2: greedy2 },
    deacon: { exactFrom: 4, early: greedy, exactFrom2: 1, early2: greedy2 },
    abbot: { exactFrom: 6, early: lookahead, exactFrom2: 3, early2: lookahead2 },
  };
  const solver = new Solver(), solver2 = new Solver2();

  // Choose a move in game coordinates for the player to move:
  // [x, y, r] or, with two in hand, [x, y, r, which].
  function chooseMove(game, level, rand) {
    rand = rand || Math.random;
    const L = LEVELS[level] || LEVELS.abbot;
    const s = fromGame(game);
    if (game.variant === 'two') {
      const hand = game.hand.map(t => H.TYPES.indexOf(t));
      let m;
      if (remaining(s) <= L.exactFrom2 && s.mask) {
        if (solver2.memo.size > 400000) solver2.memo.clear();
        const b = solver2.best(s, hand).move;
        m = { x: b[0], y: b[1], r: b[2], w: b[3] };
      } else m = L.early2(s, hand, rand);
      return [m.x + s.ox, m.y + s.oy, m.r, m.w];
    }
    const t = H.TYPES.indexOf(game.tile);
    let m;
    if (remaining(s) <= L.exactFrom) {
      if (solver.memo.size > 400000) solver.memo.clear();
      const b = solver.bestForTile(s, t).move;
      m = { x: b[0], y: b[1], r: b[2] };
    } else m = L.early(s, t, rand);
    return [m.x + s.ox, m.y + s.oy, m.r];
  }

  const api = { Solver, Solver2, fromGame, place, legalCells, key, remaining, chooseMove, LEVELS };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.HeredoxAI = api;
})(typeof self !== 'undefined' ? self : this);
