// Heredox page. All words and art come from the skin; all rules from engine.js.
(function () {
  const H = self.Heredox, AI = self.HeredoxAI, S = self.HeredoxSkin, W = S.words;
  const $ = id => document.getElementById(id);
  const stage = $('stage'), boardEl = $('board'), inner = $('inner'), overlay = $('overlay'), body = $('panel-body');
  const MIDS = [[50, 0], [100, 50], [50, 100], [0, 50]];
  const SYM = { C: 1, V: 2 };

  let L = { W: 0, H: 0, ox: 0, oy: 0, s: 0, bx: 0, by: 0, B: 0, tx: 0, ty: 0, portrait: false };
  let game = null, mode = '1p', level = 'abbot', you = 'C';
  let pending = false;  // true while a human may move the tiles in hand
  let floats = [], drag = null, gen = 0, closable = false;
  // Two tiles is the main game. The one-tile game still works ('single') but
  // its menu switch is commented out for now.
  let variant = 'two';

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
  // a player's count as shown: the first player starts half a symbol ahead
  function shown(p) {
    if (!game) return '0';
    const n = game.score()[p === 'C' ? 0 : 1];
    return p === game.first ? (n ? n + '\u00bd' : '\u00bd') : String(n);
  }
  function renderSide(bump) {
    ['C', 'V'].forEach((p, i) => {
      const row = $('row-' + p), pts = row.querySelector('.pts');
      row.querySelector('.nm').textContent = W.names[p];
      const who = row.querySelector('.who');
      if (mode === '1p' && game && p !== you) { who.innerHTML = S.hats[level]; who.classList.add('hat'); }
      else { who.textContent = mode === '1p' && game ? W.you : ''; who.classList.remove('hat'); }
      if (pts.textContent !== shown(p)) {
        pts.textContent = shown(p);
        if (bump) { pts.classList.remove('bump'); void pts.offsetWidth; pts.classList.add('bump'); }
      }
      row.classList.toggle('turn', !!game && !game.over && game.toMove === p);
    });
    $('left').textContent = W.tilesLeft(game ? game.stackLeft : 16);
    let st = '';
    if (game && !game.over) {
      const p = game.toMove;
      st = mode === '2p' ? W.toMove(W.names[p]) : p === you ? W.yourMove : W.thinking(W.names[p]);
    }
    $('status').textContent = st;
    $('b-hint').disabled = !pending;
  }

  // --- the tiles in hand ---------------------------------------------------
  // floats[i] shows game.hand[i]: {el, rot, pos, slot}. With two tiles to pick
  // from, the one not placed stays where it is for the next player.
  function slotCentre(slot) {            // slot 0 or 1 of the tray; the single game uses the middle
    const c = trayCentre();
    return game && game.variant === 'two' ? { x: c.x + (slot - 0.5) * L.s * 1.06, y: c.y } : c;
  }
  function setFloat(f, p) {
    f.pos = p;
    f.el.style.left = p.x + 'px'; f.el.style.top = p.y + 'px';
  }
  function setRot(f, q) {
    f.rot = q;
    f.el.firstChild.style.transform = `rotate(${q * 90}deg)`;
  }
  function dealHand(mine) {
    // a tile left by the previous player stays exactly where and how it was
    floats.forEach(f => { f.el.classList.toggle('mine', mine); f.el.classList.remove('arrive'); });
    while (floats.length < game.hand.length) {
      const i = floats.length;
      const slot = floats.some(f => f.slot === 0) ? 1 : 0;
      const f = { el: tileEl(H.TILES[game.hand[i]], 'float arrive' + (mine ? ' mine' : '')), rot: 0, pos: null, slot };
      f.el.style.transition = 'none';
      stage.appendChild(f.el);
      setFloat(f, slotCentre(slot));
      setRot(f, 0);
      void f.el.offsetWidth;
      f.el.style.transition = '';
      f.el.addEventListener('pointerdown', ev => onDown(ev, f));
      f.el.addEventListener('pointermove', onMove);
      f.el.addEventListener('pointerup', onUp);
      f.el.addEventListener('pointercancel', onUp);
      floats.push(f);
    }
  }
  function markTarget(c) {
    inner.querySelectorAll('.slot').forEach(n => n.classList.toggle('target', !!c && n.dataset.c === c[0] + ',' + c[1]));
  }
  function onDown(ev, f) {
    if (!pending) return;
    ev.preventDefault();
    const p = toStage(ev);
    drag = { f, sx: p.x, sy: p.y, ox: f.pos.x, oy: f.pos.y, moved: false };
    f.el.setPointerCapture(ev.pointerId);
  }
  function onMove(ev) {
    if (!drag) return;
    const p = toStage(ev), f = drag.f;
    if (!drag.moved && Math.hypot(p.x - drag.sx, p.y - drag.sy) > 8) {
      drag.moved = true;
      f.el.classList.add('dragging'); stage.classList.add('dragging');
    }
    if (drag.moved) {
      setFloat(f, { x: drag.ox + p.x - drag.sx, y: drag.oy + p.y - drag.sy });
      const c = cellAt(f.pos);
      markTarget(game.isLegalCell(c[0], c[1]) ? c : null);
    }
  }
  function onUp() {
    if (!drag || !pending) { drag = null; return; }
    const d = drag, f = d.f;
    drag = null;
    f.el.classList.remove('dragging'); stage.classList.remove('dragging');
    if (!d.moved) {            // a tap turns the tile a quarter-turn clockwise
      setRot(f, f.rot + 1);
      return;
    }
    const c = cellAt(f.pos);
    if (game.isLegalCell(c[0], c[1])) {      // dropping on a legal cell ends the turn
      commit(c[0], c[1], ((f.rot % 4) + 4) % 4, floats.indexOf(f));
      const g = gen;
      setTimeout(() => { if (g === gen) nextTurn(); }, 600);
    } else {
      setFloat(f, { x: d.ox, y: d.oy });
      markTarget(null);
      renderSide();
    }
  }

  // --- turns -------------------------------------------------------------
  function commit(x, y, r, w) {
    const e = H.rotate(H.TILES[game.hand[w]], r);
    const joins = game.joinsAt(x, y, e);
    game.play(x, y, r, w);
    floats[w].el.remove();
    floats.splice(w, 1);
    if (game.variant === 'single') { floats.forEach(f => f.el.remove()); floats = []; }
    pending = false;
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
    dealHand(mine);
    if (mine) {
      pending = true;
      renderBoard();
      renderSide();
      return;
    }
    renderSide();
    await sleep(900);
    if (g !== gen) return;
    const [x, y, r, w] = AI.chooseMove(game, level);
    const f = floats[w || 0];
    const turns = (((r - f.rot) % 4) + 4) % 4;
    for (let q = 1; q <= turns; q++) {      // turn it in the tray, one quarter at a time
      setRot(f, f.rot + 1);
      await sleep(330);
      if (g !== gen) return;
    }
    await sleep(250);
    if (g !== gen) return;
    f.el.style.transition = 'left .8s ease, top .8s ease';
    f.el.style.zIndex = 8;
    setFloat(f, cellCentre(x, y));
    await sleep(1050);
    if (g !== gen) return;
    commit(x, y, r, w || 0);
    await sleep(900);
    if (g === gen) nextTurn();
  }

  function hint() {
    if (!pending) return;
    stage.querySelectorAll('.ghost').forEach(n => n.remove());
    const [x, y, r, w] = AI.chooseMove(game, 'abbot');
    const el = tileEl(H.rotate(H.TILES[game.hand[w || 0]], r), 'float ghost');
    const p = cellCentre(x, y);
    el.style.left = p.x + 'px'; el.style.top = p.y + 'px';
    stage.appendChild(el);
    setTimeout(() => el.remove(), 2700);
    if (floats.length > 1) {              // show which of the two tiles is meant
      const f = floats[w || 0];
      f.el.classList.remove('hinted'); void f.el.offsetWidth; f.el.classList.add('hinted');
      setTimeout(() => f.el.classList.remove('hinted'), 2700);
    }
  }

  function newGame(m, lv) {
    gen++;
    mode = m; if (lv) level = lv;
    stage.querySelectorAll('.float, .glow').forEach(n => n.remove());
    floats = []; pending = false; drag = null;
    game = new H.Game(H.shuffledDeck(Math.random), Math.random() < 0.5 ? 'C' : 'V', variant);
    stage.classList.toggle('two', variant === 'two');
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
    stage.classList.toggle('landing', !game);
    overlay.hidden = false;
  }
  function closePanel(force) {
    if (force || closable) { overlay.hidden = true; stage.classList.remove('landing'); }
  }
  function showMenu() {
    const cardBtn = (act, pic, label, lv) => `<button class="card" data-act="${act}"${lv ? ` data-level="${lv}"` : ''}>${S.portraits[pic]}<span>${label}</span></button>`;
    const lv = Object.keys(W.levels).map(k => cardBtn('1p', k, W.levels[k], k)).join('');
    openPanel(`<h2>${W.title}</h2><div class="cards">
      <div class="group">${lv}</div>
      <div class="group">${cardBtn('2p', 'two', W.twoPlayers)}${cardBtn('rules', 'book', W.howToPlay)}</div>
      </div>
      `, /* one-tile option, off for now:
      <div class="switch">${Object.keys(W.variants).map(k => `<button data-act="variant" data-v="${k}" class="${k === variant ? 'on' : ''}">${W.variants[k]}</button>`).join('')}</div> */
      !!game && !game.over, 'menu');
  }
  function showRules() {
    const set = H.TYPES.map(t => `<div class="kind"><div class="face">${faceSVG(H.TILES[t])}</div><span>\u00d7${H.COUNTS[t]}</span></div>`).join('');
    openPanel(`<h2>${W.howToPlay}</h2>` + W.rules.concat(variant === 'two' ? [W.rulesTwo] : []).map(p => `<p>${p}</p>`).join('')
      + `<div class="tileset">${set}</div>`, true);
  }
  function showEnd() {
    const w = game.winner();
    const head = mode === '2p' ? W.wins(W.names[w]) : w === you ? W.youWin : W.youLose;
    openPanel(`<h2>${head}</h2><p>${W.names.C} ${shown('C')} · ${W.names.V} ${shown('V')}</p>
      <div><button class="big" data-act="again">${W.playAgain}</button> <button class="big" data-act="menu">${W.menu}</button></div>`, true);
  }
  body.addEventListener('click', ev => {
    const b = ev.target.closest('button');
    if (!b) return;
    const act = b.dataset.act;
    if (act === '1p') newGame('1p', b.dataset.level);
    else if (act === '2p') newGame('2p');
    else if (act === 'again') newGame(mode);
    else if (act === 'variant') { variant = b.dataset.v; showMenu(); }
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
  // iOS Safari zooms on a double tap whatever the page asks for, and tapping a
  // tile twice to turn it twice is a double tap. Cancel the second tap's
  // default action (pointer events, which the tiles use, still arrive).
  let lastTouchEnd = 0;
  document.addEventListener('touchend', ev => {
    const now = Date.now();
    if (now - lastTouchEnd < 400) ev.preventDefault();
    lastTouchEnd = now;
  }, { passive: false });
  document.addEventListener('dblclick', ev => ev.preventDefault());
  ['gesturestart', 'gesturechange'].forEach(t => document.addEventListener(t, ev => ev.preventDefault()));
  window.addEventListener('resize', () => {
    layout(); renderBoard();
    floats.forEach(f => {
      f.el.style.transition = 'none';
      setFloat(f, slotCentre(f.slot));
      void f.el.offsetWidth; f.el.style.transition = '';
    });
  });
  renderSide(); layout(); showMenu();
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(() => { layout(); renderBoard(); });
  self.__heredox = { get game() { return game; }, get you() { return you; }, cellCentre, trayCentre, slotCentre, get L() { return L; } };
})();
