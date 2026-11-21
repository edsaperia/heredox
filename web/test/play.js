// Scripted play-through in a headless browser: plays a whole one-player game
// by dragging and tapping, and saves screenshots.
const { chromium } = require('/opt/npm-tools/node_modules/playwright');
const path = require('path');
(async () => {
  const out = process.argv[2] || '/tmp/shots';
  require('fs').mkdirSync(out, { recursive: true });
  const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', args: ['--no-sandbox'] });
  const desktop = !!process.env.DESKTOP;
  const page = await browser.newPage(desktop ? { viewport: { width: 1440, height: 900 } } : { viewport: { width: 844, height: 390 }, hasTouch: true, isMobile: true });
  const off = () => page.evaluate(() => ({ x: self.__heredox.L.ox, y: self.__heredox.L.oy }));
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  await page.goto('file://' + path.resolve(__dirname, '../index.html'));
  await page.waitForTimeout(800);
  await page.screenshot({ path: out + '/01-menu.png' });
  await page.click('button[data-level="abbot"]');
  let shot = 2, myMoves = 0;
  for (let i = 0; i < 400; i++) {
    const st = await page.evaluate(() => {
      const h = self.__heredox, g = h.game;
      return { over: g.over, turn: g.turn, mine: g.toMove === h.you, float: !!document.querySelector('.float.mine'), 
        cells: g.legalCells(), tray: (() => { const f = [...document.querySelectorAll('.float.mine')].pop(); return f ? { x: parseFloat(f.style.left), y: parseFloat(f.style.top) } : null; })(),
        panel: !document.getElementById('overlay').hidden };
    });
    if (st.over && st.panel) break;
    if (st.mine && st.float) {
      const c = st.cells[myMoves % st.cells.length];
      const to = await page.evaluate(c => self.__heredox.cellCentre(c[0], c[1]), c);
      const o = await off(); to.x += o.x; to.y += o.y; st.tray.x += o.x; st.tray.y += o.y;
      if (myMoves === 2) { await page.click('#b-hint'); await page.waitForTimeout(600); await page.screenshot({ path: out + '/0' + (shot++) + '-hint.png' }); }
      await page.mouse.move(st.tray.x, st.tray.y);
      await page.mouse.down(); await page.mouse.up();          // tap: rotate
      await page.waitForTimeout(300);
      await page.mouse.move(st.tray.x, st.tray.y);
      await page.mouse.down();
      await page.mouse.move((st.tray.x + to.x) / 2, (st.tray.y + to.y) / 2, { steps: 4 });
      if (myMoves === 1) await page.screenshot({ path: out + '/0' + (shot++) + '-dragging.png' });
      await page.mouse.move(to.x, to.y, { steps: 4 });
      await page.mouse.up();
      await page.waitForTimeout(700);
      if (myMoves === 1) await page.screenshot({ path: out + '/0' + (shot++) + '-placed.png' });
      myMoves++;
      if (myMoves === 5) { await page.waitForTimeout(700); await page.screenshot({ path: out + '/0' + (shot++) + '-mid.png' }); }
    }
    await page.waitForTimeout(250);
  }
  await page.waitForTimeout(500);
  await page.screenshot({ path: out + '/0' + (shot++) + '-end.png' });
  const fin = await page.evaluate(() => ({ over: self.__heredox.game.over, score: self.__heredox.game.score(), tiles: document.querySelectorAll('#inner .tile').length }));
  await page.click('#x');
  await page.screenshot({ path: out + '/0' + (shot++) + '-final-board.png' });
  // portrait: the stage should turn to landscape
  await page.setViewportSize(desktop ? { width: 700, height: 1000 } : { width: 500, height: 900 });
  await page.waitForTimeout(400);
  await page.screenshot({ path: out + '/0' + (shot++) + '-portrait.png' });
  console.log(JSON.stringify({ fin, myMoves, errors }));
  await browser.close();
  if (!fin.over || fin.tiles !== 16 || errors.length) process.exit(1);
})();
