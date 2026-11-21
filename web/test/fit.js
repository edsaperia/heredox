// The side panel must fit at phone sizes, even with a tall font.
const { chromium } = require('/opt/npm-tools/node_modules/playwright');
const path = require('path');
(async () => {
  const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', args: ['--no-sandbox'] });
  let bad = 0;
  for (const [w, h] of [[393, 780], [780, 393], [667, 375], [375, 600], [932, 430], [1280, 720], [568, 320]]) {
    const page = await browser.newPage({ viewport: { width: w, height: h } });
    await page.addInitScript(() => document.addEventListener('DOMContentLoaded', () => {
      const st = document.createElement('style'); st.textContent = '#side, #side * { line-height: 1.7 !important; }'; document.head.appendChild(st);
    }));
    await page.goto('file://' + path.resolve(__dirname, '../index.html'));
    await page.waitForTimeout(300);
    await page.evaluate(() => window.dispatchEvent(new Event('resize')));
    await page.waitForTimeout(200);
    const r = await page.evaluate(() => { const s = document.getElementById('side'), m = document.getElementById('b-menu'); return { sh: s.scrollHeight, ch: s.clientHeight, menuBottom: m.offsetTop + m.offsetHeight + m.offsetParent.offsetTop }; });
    const ok = r.sh <= r.ch + 1;
    if (!ok) bad++;
    console.log(w + 'x' + h, ok ? 'fits' : 'OVERFLOWS', JSON.stringify(r));
    if (w === 393) await page.screenshot({ path: '/tmp/shots/fit-portrait.png' });
    await page.close();
  }
  await browser.close();
  process.exit(bad ? 1 : 0);
})();
