# Heredox

A two-player tile game by Edward Saperia. Rules and rulings are in `RULES.md`.

- `py/` reference engine and tests (`cd py && python3 -m unittest`)
- `rs/` Rust port, transcript replayer and exact endgame solver
- `web/` the playable page: one template (`index.html`, `ui.js`), one rules
  engine (`engine.js`), computer players (`ai.js`) and a skin folder holding
  all art and words (`skins/classic/`)

Checks: `node web/test/replay.js transcripts.jsonl` replays Python games
through the JavaScript engine; `node web/test/solver.js` checks the JavaScript
solver against values from the Rust solver; `node web/test/play.js` plays a
whole game in a headless browser.
