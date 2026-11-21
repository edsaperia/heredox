// Checks ai.js's exact solver against values computed by the Rust solver,
// then plays whole games at each level to make sure every move is legal.
const fs = require('fs');
const H = require('../engine.js');
const AI = require('../ai.js');
let worst = 0, n = 0;
const t0 = Date.now();
for (const line of fs.readFileSync(__dirname + '/solver_positions.jsonl', 'utf8').split('\n')) {
  if (!line.trim()) continue;
  const p = JSON.parse(line);
  const s = { mask: p.mask, cells: p.cells, counts: p.counts, diff: p.diff, mover: 1, first: p.counts.reduce((a, b) => a + b, 0) % 2 === 0 ? 1 : 2, ox: 0, oy: 0 };
  const v = new AI.Solver().solve(s);
  worst = Math.max(worst, Math.abs(v[0] - p.pm), Math.abs(v[1] - p.po));
  n++;
}
console.log(`${n} positions against Rust: largest disagreement ${worst.toExponential(2)} (${Date.now() - t0} ms)`);
if (worst > 1e-9) process.exit(1);

let seed = 99;
const rand = () => (seed = (seed * 1664525 + 1013904223) % 4294967296) / 4294967296;
const res = {};
for (const [a, b] of [['novice', 'novice'], ['deacon', 'novice'], ['abbot', 'novice'], ['abbot', 'deacon']]) {
  const r = [0, 0, 0];
  let slow = 0;
  for (let i = 0; i < 60; i++) {
    const aSym = i % 2 ? 'C' : 'V';
    const g = new H.Game(H.shuffledDeck(rand), rand() < 0.5 ? 'C' : 'V');
    while (!g.over) {
      const t1 = Date.now();
      g.play(...AI.chooseMove(g, g.toMove === aSym ? a : b, rand));
      slow = Math.max(slow, Date.now() - t1);
    }
    const w = g.winner();
    r[w === aSym ? 0 : 2]++;
  }
  console.log(`${a} v ${b}: ${r[0]} wins, ${r[1]} draws, ${r[2]} losses; slowest move ${slow} ms`);
}
