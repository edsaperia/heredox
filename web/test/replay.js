// Replays the Python transcripts through engine.js and prints the number of
// legal moves at each turn, for diffing against the other engines.
const fs = require('fs');
const H = require('../engine.js');
let n = 0;
for (const line of fs.readFileSync(process.argv[2], 'utf8').split('\n')) {
  if (!line.trim()) continue;
  const t = JSON.parse(line), g = new H.Game(t.deck, t.first), counts = [];
  for (const [x, y, r] of t.moves) {
    const legal = g.legalMoves();
    counts.push(legal.length);
    if (!legal.some(m => m[0] === x && m[1] === y && m[2] === r)) throw new Error('move not legal in game ' + n);
    g.play(x, y, r);
  }
  const s = g.score();
  if (s[0] !== t.score[0] || s[1] !== t.score[1]) throw new Error('score mismatch in game ' + n);
  console.log(counts.join(','));
  n++;
}
console.error('replayed ' + n + ' games, all scores agree');
