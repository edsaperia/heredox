// Checks ai.js's network evaluation against Rust, on positions dumped with
// their values by `heredox-train dumpnet`. Each position is also checked with
// the two symbols exchanged, which must give the same answer.
// usage: node test/net.js <net.json> <positions.jsonl>
const fs = require('fs');
const AI = require('../ai.js');
AI.setNet(JSON.parse(fs.readFileSync(process.argv[2], 'utf8')));
const sw = v => (v === 1 ? 2 : v === 2 ? 1 : 0), ab = t => (t === 0 ? 1 : t === 1 ? 0 : t);
let worst = 0, n = 0;
for (const line of fs.readFileSync(process.argv[3], 'utf8').split('\n')) {
  if (!line.trim()) continue;
  const p = JSON.parse(line);
  const placed = p.cells.filter(c => c).length, moverFirst = placed % 2 === 0;
  const a = { mask: p.mask, cells: p.cells, counts: p.counts, diff: p.diff, mover: 1, first: moverFirst ? 1 : 2 };
  const c = p.counts.slice(); [c[0], c[1]] = [c[1], c[0]];
  const b = { mask: p.mask, cells: p.cells.map(e => e && e.map(sw)), counts: c, diff: -p.diff, mover: 2, first: moverFirst ? 2 : 1 };
  const h = p.h === 9 ? -1 : p.h;
  worst = Math.max(worst, Math.abs(AI.netValue(a, h) - p.v), Math.abs(AI.netValue(b, h < 0 ? -1 : ab(h)) - p.v));
  n++;
}
console.log(`${n} positions, net value against Rust: largest disagreement ${worst.toExponential(2)}`);
if (worst > 2e-4) process.exit(1);
