// Two in hand: check the memoised solver against plain search on small
// endgames, then play whole games at each level.
const H = require('../engine.js');
const AI = require('../ai.js');
let seed = 4242;
const rand = () => (seed = (seed * 1664525 + 1013904223) % 4294967296) / 4294967296;

// plain search with no memo, written separately from Solver2
function brute(g) {
  if (g.over) { const w = g.winner(); return w === 'C' ? [1, 0] : w === 'V' ? [0, 1] : [0, 0]; }
  // the mover's hand is known; what they hand over is known; the next draw is the next deck tile,
  // so average over every order of the remaining stack by averaging over which stack tile comes next
  const me = g.toMove === 'C' ? 0 : 1;
  let best = null;
  for (const m of g.legalMoves()) {
    // enumerate distinct next draws from the stack (g.pos .. 15), weighting by count
    const stack = g.deck.slice(g.pos).split('');
    let v;
    const base = g.deck.slice(0, g.pos);
    // the tile at deck[g.pos-1] is already in hand, so permuting the stack is safe
    if (stack.length <= 1 || g.hand.length < 2 && stack.length === 0) {
      const c = clone(g); c.play(...m); v = brute(c);
    } else {
      // after this move the opponent draws deck[g.pos]; average over which stack tile that is
      v = [0, 0];
      const seen = {};
      stack.forEach(t => seen[t] = (seen[t] || 0) + 1);
      for (const t of Object.keys(seen)) {
        const rest = stack.slice(); rest.splice(rest.indexOf(t), 1);
        const c = clone(g); c.deck = base + t + rest.join(''); c.play(...m);
        const r = brute(c);
        v[0] += seen[t] / stack.length * r[0]; v[1] += seen[t] / stack.length * r[1];
      }
    }
    if (best === null || v[me] > best[me] + 1e-12 || (Math.abs(v[me] - best[me]) <= 1e-12 && v[1 - me] < best[1 - me] - 1e-12)) best = v;
  }
  return best;
}
function clone(g) {
  const c = new H.Game(H.FULL_DECK, g.first, g.variant);
  c.deck = g.deck; c.board = new Map(g.board); c.moves = g.moves.slice(); c.hand = g.hand.slice(); c.pos = g.pos;
  return c;
}
let worst = 0, n = 0;
for (let i = 0; i < 40; i++) {
  const g = new H.Game(H.shuffledDeck(rand), 'C', 'two');
  while (g.stackLeft > 2) { const ms = g.legalMoves(); g.play(...ms[Math.floor(rand() * ms.length)]); }
  const s = AI.fromGame(g), hand = g.hand.map(t => H.TYPES.indexOf(t));
  const a = new AI.Solver2().best(s, hand).value, b = brute(g);
  worst = Math.max(worst, Math.abs(a[0] - b[0]), Math.abs(a[1] - b[1]));
  n++;
}
console.log(`${n} two-in-hand endgames against plain search: largest disagreement ${worst.toExponential(2)}`);
if (worst > 1e-9) process.exit(1);

for (const [a, b] of [['novice', 'novice'], ['deacon', 'novice'], ['abbot', 'novice'], ['abbot', 'deacon']]) {
  const r = [0, 0, 0], seat = [0, 0, 0];
  let slow = 0;
  for (let i = 0; i < 100; i++) {
    const aSym = i % 2 ? 'C' : 'V';
    const g = new H.Game(H.shuffledDeck(rand), rand() < 0.5 ? 'C' : 'V', 'two');
    while (!g.over) {
      const t1 = Date.now();
      g.play(...AI.chooseMove(g, g.toMove === aSym ? a : b, rand));
      slow = Math.max(slow, Date.now() - t1);
    }
    const w = g.winner();
    r[w === null ? 1 : w === aSym ? 0 : 2]++;
    seat[w === null ? 1 : w === g.first ? 0 : 2]++;
  }
  console.log(`${a} v ${b}: ${r[0]} wins, ${r[1]} draws, ${r[2]} losses; first seat won ${seat[0]}, second ${seat[2]}; slowest move ${slow} ms`);
}
