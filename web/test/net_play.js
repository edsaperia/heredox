// Abbot with the trained net against Abbot without it, and against Deacon.
const H = require('../engine.js');
global.self = global;
require('../net.js');
const AI = require('../ai.js');
if (!self.HeredoxNetData) { console.log('no trained net in net.js: nothing to test'); process.exit(0); }
AI.setNet(self.HeredoxNetData);
let seed = 7;
const rand = () => (seed = (seed * 1664525 + 1013904223) % 4294967296) / 4294967296;
function match(useNetFor, other, n) {
  let wins = 0, slow = 0;
  for (let i = 0; i < n; i++) {
    const netSym = i % 2 ? 'C' : 'V';
    const g = new H.Game(H.shuffledDeck(rand), 'V', 'three');
    while (!g.over) {
      const mine = g.toMove === netSym;
      AI.setNet(mine ? self.HeredoxNetData : null);
      const t = Date.now();
      g.play(...AI.chooseMove(g, mine ? 'abbot' : other, rand));
      slow = Math.max(slow, Date.now() - t);
    }
    if (g.winner() === netSym) wins++;
  }
  console.log(`Abbot with net v ${other} without: ${wins} of ${n}; slowest move ${slow} ms`);
  return wins;
}
const a = match(true, 'abbot', 100), b = match(true, 'deacon', 100);
AI.setNet(self.HeredoxNetData);
if (a < 70 || b < 70) process.exit(1);
