// Heredox page. All words and art come from the skin; all rules from engine.js.
(function () {
  const H = self.Heredox, AI = self.HeredoxAI, S = self.HeredoxSkin, W = S.words;
  const $ = id => document.getElementById(id);
  const stage = $('stage'), boardEl = $('board'), inner = $('inner'), overlay = $('overlay'), body = $('panel-body');
  const MIDS = [[50, 0], [100, 50], [50, 100], [0, 50]];
  const SYM = { C: 1, V: 2 };

  let L = { W: 0, H: 0, ox: 0, oy: 0, s: 0, bx: 0, by: 0, B: 0, tx: 0, ty: 0, portrait: false };
  let game = null, mode = '1p', level = 'abbot', you = 'C';
  let pending = null;   // {rot} while a human holds the tile
  let floatEl = null, cur = { x: 0, y: 0 }, drag = null, gen = 0, closable = false;

  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const isHuman = sym => mode === '2p' || sym === you;

  // --- drawing -----------------------------------------------------------
  function symbolSVG(sym) {
    const s = S.symbols[sym];
    return `<svg viewBox="-1.2 -1.2 2.4 2.4" style="color:${s.color}">${s.art}</svg>`;
  }
  function faceSVG(e) {
    let g = '';
    for (let d = 0; d < 4; d++) {
      if (!e[d]) continue;
      const s = S.symbols[e[d]];
      g += `<g transform="translate(${MIDS[d][0]},${MIDS[d][1]}) scale(31)" style="color:${s.color}">${s.art}</g>`;
    }
    return `<svg viewBox="0 0 100 100">${g}</svg>`;
  }
  function tileEl(e, cls) {
    const el = document.createElement('div');
    el.className = 'tile ' + (cls || '');
    el.innerHTML = `<div class="face">${faceSVG(e)}</div>`;
    return el;
  }

  // --- layout ------------------------------------------------------------
  function layout() {
    const vw = window.innerWidth, vh = window.innerHeight;
    if (window.matchMedia('(pointer: coarse)').matches) {
      // phones and tablets: fill the screen, always landscape
      L.portrait = vh > vw;
      L.W = Math.max(vw, vh); L.H = Math.min(vw, vh);
      L.ox = 0; L.oy = 0;
    } else {
      // desktop: never turn the page; a fixed-shape table centred in the window
      L.portrait = false;
      L.H = Math.min(vh, 820, vw / 1.6);
      L.W = Math.min(vw, L.H * 1.7);
      L.ox = (vw - L.W) / 2; L.oy = (vh - L.H) / 2;
    }
    stage.classList.toggle('portrait', L.portrait);
    stage.style.left = L.ox + 'px'; stage.style.top = L.oy + 'px';
    stage.style.width = L.W + 'px'; stage.style.height = L.H + 'px';
    stage.style.fontSize = Math.max(10, L.H / 26) + 'px';
    const side = Math.max(L.H * 0.5, Math.min(L.W - L.H * 1.08, L.H * 0.74));
    $('side').style.width = side + 'px';
    L.B = Math.min(L.H, L.W - side);
    L.s = L.B / 5;
    L.bx = (L.W - side - L.B) / 2; L.by = (L.H - L.B) / 2;
    boardEl.style.left = L.bx + 'px'; boardEl.style.top = L.by + 'px';
    boardEl.style.width = boardEl.style.height = L.B + 'px';
    stage.style.setProperty('--s', L.s + 'px');
    // shrink the side panel's text until every button fits
    const sideEl = $('side');
    let f = Math.max(10, L.H / 26);
    const names = [...sideEl.querySelectorAll('.nm')];
    while ((sideEl.scrollHeight > L.H + 0.5 || names.some(n => n.scrollWidth > n.clientWidth)) && f > 6) {
      f -= 0.5;
      stage.style.fontSize = f + 'px';
    }
    centre();
  }
  function centre() {
    let mx = 0, my = 0;
    if (game && game.board.size) {
      const [x0, x1, y0, y1] = game.bounds();
      mx = (x0 + x1) / 2; my = (y0 + y1) / 2;
    }
    L.tx = L.B / 2 - (mx + 0.5) * L.s; L.ty = L.B / 2 - (my + 0.5) * L.s;
    inner.style.transform = `translate(${L.tx}px, ${L.ty}px)`;
  }
  const cellCentre = (x, y) => ({ x: L.bx + L.tx + (x + 0.5) * L.s, y: L.by + L.ty + (y + 0.5) * L.s });
  const cellAt = p => [Math.floor((p.x - L.bx - L.tx) / L.s), Math.floor((p.y - L.by - L.ty) / L.s)];
  function trayCentre() {
    let el = $('tray'), x = el.offsetWidth / 2, y = el.offsetHeight / 2;
    while (el && el !== stage) { x += el.offsetLeft; y += el.offsetTop; el = el.offsetParent; }
    return { x, y };
  }
  function toStage(ev) {
    return L.portrait ? { x: ev.clientY, y: window.innerWidth - ev.clientX } : { x: ev.clientX - L.ox, y: ev.clientY - L.oy };
  }

  function renderBoard() {
    inner.querySelectorAll('.tile, .slot').forEach(n => n.remove());
    if (!game) return;
    for (const t of game.board.values()) {
      const el = tileEl(t.e);
      el.style.left = t.x * L.s + 'px'; el.style.top = t.y * L.s + 'px';
      inner.appendChild(el);
    }
    if (pending) {
      for (const [x, y] of game.legalCells()) {
        const el = document.createElement('div');
        el.className = 'slot'; el.dataset.c = x + ',' + y;
        el.innerHTML = '<i></i>';
        el.style.left = x * L.s + 'px'; el.style.top = y * L.s + 'px';
        inner.appendChild(el);
      }
    }
  }
  function renderSide(bump) {
    const sc = game ? game.score() : [0, 0];
    ['C', 'V'].forEach((p, i) => {
      const row = $('row-' + p), pts = row.querySelector('.pts');
      row.querySelector('.nm').textContent = W.names[p];
      const who = row.querySelector('.who');
      if (mode === '1p' && game && p !== you) { who.innerHTML = S.hats[level]; who.classList.add('hat'); }
      else { who.textContent = mode === '1p' && game ? W.you : ''; who.classList.remove('hat'); }
      if (pts.textContent !== String(sc[i])) {
        pts.textContent = sc[i];
        if (bump) { pts.classList.remove('bump'); void pts.offsetWidth; pts.classList.add('bump'); }
      }
      row.classList.toggle('turn', !!game && !game.over && game.toMove === p);
    });
    const left = game ? 16 - game.turn - (floatEl ? 1 : 0) : 16;
    $('left').textContent = W.tilesLeft(Math.max(0, left));
    let st = '';
    if (game && !game.over) {
      const p = game.toMove;
      st = mode === '2p' ? W.toMove(W.names[p]) : p === you ? W.yourMove : W.thinking(W.names[p]);
    }
    $('status').textContent = st;
    $('b-hint').disabled = !pending;
  }

  // --- the tile in hand --------------------------------------------------
  function setFloat(p) {
    cur = p;
    floatEl.style.left = p.x + 'px'; floatEl.style.top = p.y + 'px';
  }
  function setRot(q) {
    floatEl.dataset.rot = q;
    floatEl.firstChild.style.transform = `rotate(${q * 90}deg)`;
  }
  function drawTile(mine) {
    floatEl = tileEl(H.TILES[game.tile], 'float arrive' + (mine ? ' mine' : ''));
    floatEl.style.transition = 'none';
    stage.appendChild(floatEl);
    setFloat(trayCentre());
    setRot(0);
    void floatEl.offsetWidth;
    floatEl.style.transition = '';
    if (mine) {
      floatEl.addEventListener('pointerdown', onDown);
      floatEl.addEventListener('pointermove', onMove);
      floatEl.addEventListener('pointerup', onUp);
      floatEl.addEventListener('pointercancel', onUp);
    }
  }
  function markTarget(c) {
    inner.querySelectorAll('.slot').forEach(n => n.classList.toggle('target', !!c && n.dataset.c === c[0] + ',' + c[1]));
  }
  function onDown(ev) {
    if (!pending) return;
    ev.preventDefault();
    const p = toStage(ev);
    drag = { sx: p.x, sy: p.y, ox: cur.x, oy: cur.y, moved: false };
    floatEl.setPointerCapture(ev.pointerId);
  }
  function onMove(ev) {
    if (!drag) return;
    const p = toStage(ev);
    if (!drag.moved && Math.hypot(p.x - drag.sx, p.y - drag.sy) > 8) {
      drag.moved = true;
      floatEl.classList.add('dragging'); stage.classList.add('dragging');
    }
    if (drag.moved) {
      setFloat({ x: drag.ox + p.x - drag.sx, y: drag.oy + p.y - drag.sy });
      const c = cellAt(cur);
      markTarget(game.isLegalCell(c[0], c[1]) ? c : null);
    }
  }
  function onUp() {
    if (!drag || !pending) { drag = null; return; }
    const d = drag;
    drag = null;
    floatEl.classList.remove('dragging'); stage.classList.remove('dragging');
    if (!d.moved) {            // a tap turns the tile a quarter-turn clockwise
      pending.rot++;
      setRot(pending.rot);
      return;
    }
    const c = cellAt(cur);
    if (game.isLegalCell(c[0], c[1])) {      // dropping on a legal cell ends the turn
      commit(c[0], c[1], ((pending.rot % 4) + 4) % 4);
      const g = gen;
      setTimeout(() => { if (g === gen) nextTurn(); }, 600);
    } else {
      setFloat({ x: d.ox, y: d.oy });
      markTarget(null);
      renderSide();
    }
  }

  // --- turns -------------------------------------------------------------
  function commit(x, y, r) {
    const e = H.rotate(H.TILES[game.tile], r);
    const joins = game.joinsAt(x, y, e);
    game.play(x, y, r);
    if (floatEl) { floatEl.remove(); floatEl = null; }
    pending = null;
    renderBoard();
    for (const j of joins) {
      const el = document.createElement('div');
      el.className = 'glow';
      const c = S.symbols[j.sym].color;
      el.style.background = `radial-gradient(circle, ${c} 0%, ${c}88 30%, transparent 70%)`;
      el.style.left = (j.x + 0.5 + H.DIRS[j.d][0] * 0.5) * L.s + 'px';
      el.style.top = (j.y + 0.5 + H.DIRS[j.d][1] * 0.5) * L.s + 'px';
      inner.appendChild(el);
      setTimeout(() => el.remove(), 1200);
    }
    centre();
    renderSide(true);
  }

  async function nextTurn() {
    const g = gen;
    if (game.over) {
      renderSide();
      await sleep(1300);
      if (g === gen) showEnd();
      return;
    }
    const mine = isHuman(game.toMove);
    drawTile(mine);
    if (mine) {
      pending = { rot: 0 };
      renderBoard();
      renderSide();
      return;
    }
    renderSide();
    await sleep(900);
    if (g !== gen) return;
    const [x, y, r] = AI.chooseMove(game, level);
    for (let q = 1; q <= r; q++) {          // turn it in the tray, one quarter at a time
      setRot(q);
      await sleep(330);
      if (g !== gen) return;
    }
    await sleep(250);
    if (g !== gen) return;
    floatEl.style.transition = 'left .8s ease, top .8s ease';
    setFloat(cellCentre(x, y));
    await sleep(1050);
    if (g !== gen) return;
    commit(x, y, r);
    await sleep(900);
    if (g === gen) nextTurn();
  }

  function hint() {
    if (!pending) return;
    stage.querySelectorAll('.ghost').forEach(n => n.remove());
    const [x, y, r] = AI.chooseMove(game, 'abbot');
    const el = tileEl(H.rotate(H.TILES[game.tile], r), 'float ghost');
    const p = cellCentre(x, y);
    el.style.left = p.x + 'px'; el.style.top = p.y + 'px';
    stage.appendChild(el);
    setTimeout(() => el.remove(), 2700);
  }

  function newGame(m, lv) {
    gen++;
    mode = m; if (lv) level = lv;
    stage.querySelectorAll('.float, .glow').forEach(n => n.remove());
    floatEl = null; pending = null; drag = null;
    game = new H.Game(H.shuffledDeck(Math.random), Math.random() < 0.5 ? 'C' : 'V');
    you = Math.random() < 0.5 ? 'C' : 'V';
    closePanel(true);
    centre(); renderBoard(); renderSide();
    const g = gen;
    setTimeout(() => { if (g === gen) nextTurn(); }, 500);
  }

  // --- panels ------------------------------------------------------------
  function openPanel(html, canClose, cls) {
    $('panel').className = cls || '';
    body.innerHTML = html;
    closable = canClose;
    $('x').hidden = !canClose;
    overlay.classList.toggle('solid', cls === 'menu' && !game);
    overlay.hidden = false;
  }
  function closePanel(force) {
    if (force || closable) overlay.hidden = true;
  }
  function showMenu() {
    const cardBtn = (act, pic, label, lv) => `<button class="card" data-act="${act}"${lv ? ` data-level="${lv}"` : ''}>${S.portraits[pic]}<span>${label}</span></button>`;
    const lv = Object.keys(W.levels).map(k => cardBtn('1p', k, W.levels[k], k)).join('');
    openPanel(`<h2>${W.title}</h2><div class="cards">
      <div class="group">${lv}</div>
      <div class="group">${cardBtn('2p', 'two', W.twoPlayers)}${cardBtn('rules', 'book', W.howToPlay)}</div>
      </div>`, !!game && !game.over, 'menu');
  }
  function showRules() {
    openPanel(`<h2>${W.howToPlay}</h2>` + W.rules.map(p => `<p>${p}</p>`).join(''), true);
  }
  function showEnd() {
    const w = game.winner(), sc = game.score();
    const head = w === null ? W.draw : mode === '2p' ? W.wins(W.names[w]) : w === you ? W.youWin : W.youLose;
    openPanel(`<h2>${head}</h2><p>${W.names.C} ${sc[0]} · ${W.names.V} ${sc[1]}</p>
      <div><button class="big" data-act="again">${W.playAgain}</button> <button class="big" data-act="menu">${W.menu}</button></div>`, true);
  }
  body.addEventListener('click', ev => {
    const b = ev.target.closest('button');
    if (!b) return;
    const act = b.dataset.act;
    if (act === '1p') newGame('1p', b.dataset.level);
    else if (act === '2p') newGame('2p');
    else if (act === 'again') newGame(mode);
    else if (act === 'menu') showMenu();
    else if (act === 'rules') { const back = !game; showRules(); if (back) rulesFromMenu = true; }
  });
  let rulesFromMenu = false;
  function dismiss() {
    if (rulesFromMenu) { rulesFromMenu = false; showMenu(); return; }
    closePanel();
  }
  $('x').addEventListener('click', dismiss);
  overlay.addEventListener('pointerdown', ev => { if (ev.target === overlay && (closable || rulesFromMenu)) dismiss(); });

  // --- start -------------------------------------------------------------
  $('t-title').textContent = W.title;
  $('b-hint').textContent = W.hint;
  $('b-rules').textContent = W.howToPlay;
  $('b-menu').textContent = W.menu;
  $('row-C').querySelector('.ico').innerHTML = symbolSVG(1);
  $('row-V').querySelector('.ico').innerHTML = symbolSVG(2);
  $('b-hint').addEventListener('click', hint);
  $('b-rules').addEventListener('click', showRules);
  $('b-menu').addEventListener('click', showMenu);
  document.addEventListener('contextmenu', ev => ev.preventDefault());
  window.addEventListener('resize', () => {
    layout(); renderBoard();
    if (floatEl) {
      floatEl.style.transition = 'none';
      setFloat(trayCentre());
      void floatEl.offsetWidth; floatEl.style.transition = '';
    }
  });
  renderSide(); layout(); showMenu();
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(() => { layout(); renderBoard(); });
  self.__heredox = { get game() { return game; }, get you() { return you; }, cellCentre, trayCentre, get L() { return L; } };
})();
