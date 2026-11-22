// Checks ai.js's exact solver for the three-tile game against values from
// the Rust solver (rs: `exp dump 120 three`).
const fs = require('fs');
const AI = require('../ai.js');
let worst = 0, n = 0;
const t0 = Date.now();
for (const line of fs.readFileSync(__dirname + '/three_positions.jsonl', 'utf8').split('\n')) {
  if (!line.trim()) continue;
  const p = JSON.parse(line);
  const s = { mask: p.mask, cells: p.cells, counts: p.counts, diff: p.diff, mover: 1, first: p.first ? 1 : 2, choice: 3, ox: 0, oy: 0 };
  const v = new AI.Solver2().solve(s, p.left);
  worst = Math.max(worst, Math.abs(v[0] - p.v), Math.abs(v[1] - (1 - p.v)));
  n++;
}
console.log(`${n} three-tile positions against Rust: largest disagreement ${worst.toExponential(2)} (${Date.now() - t0} ms)`);
if (worst > 1e-9) process.exit(1);
