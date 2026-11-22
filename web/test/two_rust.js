// Checks ai.js's two-tile solver against values from the Rust solver.
const fs = require('fs');
const AI = require('../ai.js');
let worst = 0, n = 0;
for (const line of fs.readFileSync(__dirname + '/two_positions.jsonl', 'utf8').split('\n')) {
  if (!line.trim()) continue;
  const p = JSON.parse(line);
  const placed = p.cells.filter(c => c).length;
  const s = { mask: p.mask, cells: p.cells, counts: p.counts, diff: p.diff, mover: 1, first: placed % 2 === 0 ? 1 : 2, choice: 2, ox: 0, oy: 0 };
  const v = new AI.Solver2().best(s, p.hand).value;
  worst = Math.max(worst, Math.abs(v[0] - p.v), Math.abs(v[1] - (1 - p.v)));
  n++;
}
console.log(`${n} two-tile positions against Rust: largest disagreement ${worst.toExponential(2)}`);
if (worst > 1e-9) process.exit(1);
