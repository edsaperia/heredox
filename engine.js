// Heredox rules engine. Mirrors py/heredox.py; see RULES.md for conventions.
(function (root) {
  const BLANK = 0, C = 1, V = 2;
  const TYPES = ['A', 'B', 'D', 'E', 'F'];
  const TILES = {
    A: [C, V, BLANK, BLANK],
    B: [C, BLANK, BLANK, V],
    D: [C, BLANK, V, BLANK],
    E: [C, V, V, C],
    F: [V, C, V, C],
  };
  const COUNTS = { A: 3, B: 3, D: 2, E: 4, F: 4 };
  const DISTINCT_ROTATIONS = { A: 4, B: 4, D: 4, E: 4, F: 2 };
  const FULL_DECK = 'AAABBBDDEEEEFFFF';
  const CHOICES = { single: 1, two: 2, three: 3 };   // tiles to choose from each turn
  const DIRS = [[0, -1], [1, 0], [0, 1], [-1, 0]]; // top, right, bottom, left

  function rotate(e, r) {
    return [0, 1, 2, 3].map(i => e[(i - r + 8) % 4]);
  }
  const k = (x, y) => x + ',' + y;

  function shuffledDeck(rand) {
    const d = FULL_DECK.split('');
    for (let i = d.length - 1; i > 0; i--) {
      const j = Math.floor(rand() * (i + 1));
      [d[i], d[j]] = [d[j], d[i]];
    }
    return d.join('');
  }

  class Game {
    // variant 'single': draw one tile, place it.
    // variants 'two' and 'three': that many tiles to choose from. The first
    // player draws them; each turn you place one, the rest stay for your
    // opponent, and one more is drawn while the stack lasts. Tiles left for
    // you come first in hand, the newly drawn one last.
    constructor(deck, first, variant) {
      if (deck.split('').sort().join('') !== FULL_DECK) throw new Error('deck must be the 16 tiles');
      this.deck = deck;
      this.first = first || 'C';
      this.variant = variant || 'single';
      this.board = new Map(); // "x,y" -> {x, y, e, type, r}
      this.moves = [];
      this.choice = CHOICES[this.variant];
      if (!this.choice) throw new Error('unknown variant');
      this.hand = deck.slice(0, this.choice).split('');
      this.pos = this.choice;
    }
    get stackLeft() { return 16 - this.pos; }
    get turn() { return this.moves.length; }
    get over() { return this.turn === 16; }
    get tile() { return this.hand[0]; }
    get toMove() {
      const other = this.first === 'C' ? 'V' : 'C';
      return this.turn % 2 === 0 ? this.first : other;
    }
    at(x, y) { return this.board.get(k(x, y)); }
    bounds() {
      let x0 = 99, x1 = -99, y0 = 99, y1 = -99;
      for (const t of this.board.values()) {
        x0 = Math.min(x0, t.x); x1 = Math.max(x1, t.x);
        y0 = Math.min(y0, t.y); y1 = Math.max(y1, t.y);
      }
      return [x0, x1, y0, y1];
    }
    legalCells() {
      if (this.board.size === 0) return [[0, 0]];
      const [x0, x1, y0, y1] = this.bounds();
      const seen = new Set(), cells = [];
      for (const t of this.board.values()) {
        for (const [dx, dy] of DIRS) {
          const x = t.x + dx, y = t.y + dy, key = k(x, y);
          if (this.board.has(key) || seen.has(key)) continue;
          if (Math.max(x1, x) - Math.min(x0, x) > 3) continue;
          if (Math.max(y1, y) - Math.min(y0, y) > 3) continue;
          seen.add(key);
          cells.push([x, y]);
        }
      }
      cells.sort((a, b) => a[1] - b[1] || a[0] - b[0]);
      return cells;
    }
    isLegalCell(x, y) { return this.legalCells().some(c => c[0] === x && c[1] === y); }
    legalMoves() {
      if (this.over) return [];
      const cells = this.legalCells(), out = [];
      if (this.variant === 'single') {
        const n = DISTINCT_ROTATIONS[this.tile];
        for (const [x, y] of cells) for (let r = 0; r < n; r++) out.push([x, y, r]);
        return out;
      }
      this.hand.forEach((t, w) => {
        for (const [x, y] of cells) for (let r = 0; r < DISTINCT_ROTATIONS[t]; r++) out.push([x, y, r, w]);
      });
      return out;
    }
    play(x, y, r, which) {
      which = which || 0;
      if (this.over) throw new Error('game over');
      if (!this.isLegalCell(x, y) || r < 0 || r > 3 || which >= this.hand.length) throw new Error('illegal move');
      const type = this.hand[which];
      this.board.set(k(x, y), { x, y, e: rotate(TILES[type], r), type, r });
      if (this.variant === 'single') { this.moves.push([x, y, r]); this.hand = []; }
      else { this.moves.push([x, y, r, which]); this.hand = this.hand.filter((_, i) => i !== which); }
      if (this.pos < 16 && this.moves.length < 16) this.hand.push(this.deck[this.pos++]);
    }
    // Joins made by a tile with edges e at (x, y): [{x, y, d, sym}] for matches.
    joinsAt(x, y, e) {
      const out = [];
      for (let d = 0; d < 4; d++) {
        const n = this.at(x + DIRS[d][0], y + DIRS[d][1]);
        if (n && e[d] !== BLANK && e[d] === n.e[(d + 2) % 4]) out.push({ x, y, d, sym: e[d] });
      }
      return out;
    }
    score() {
      const s = [0, 0, 0];
      for (const t of this.board.values()) {
        for (const d of [1, 2]) {
          const n = this.at(t.x + DIRS[d][0], t.y + DIRS[d][1]);
          if (n && t.e[d] !== BLANK && t.e[d] === n.e[(d + 2) % 4]) s[t.e[d]]++;
        }
      }
      return [s[C], s[V]];
    }
    // Joins where a Castian half meets a Veridican half: dioceses left split.
    splits() {
      let n = 0;
      for (const t of this.board.values()) {
        for (const d of [1, 2]) {
          const o = this.at(t.x + DIRS[d][0], t.y + DIRS[d][1]);
          if (o && t.e[d] !== BLANK && o.e[(d + 2) % 4] !== BLANK && t.e[d] !== o.e[(d + 2) % 4]) n++;
        }
      }
      return n;
    }
    // Equal counts go to the player who moved first.
    winner() {
      const [c, v] = this.score();
      return c > v ? 'C' : v > c ? 'V' : this.first;
    }
    transcript() {
      return { deck: this.deck, first: this.first, variant: this.variant, moves: this.moves.map(m => m.slice()), score: this.score() };
    }
  }

  const api = { BLANK, C, V, TYPES, TILES, COUNTS, DISTINCT_ROTATIONS, FULL_DECK, CHOICES, DIRS, rotate, shuffledDeck, Game };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.Heredox = api;
})(typeof self !== 'undefined' ? self : this);
