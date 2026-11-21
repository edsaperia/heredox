# Heredox — rules and rulings

Two players. One is Castian (C, the rounded cross), the other Veridican (V, the
spiked cross).

## Tiles

16 square tiles. Each edge carries half a Castian symbol, half a Veridican
symbol, or nothing. Edges are listed top, right, bottom, left.

| Type | Count | Edges      | Distinct rotations |
|------|-------|------------|--------------------|
| A    | 3     | C, V, –, – | 4 |
| B    | 3     | C, –, –, V | 4 |
| D    | 2     | C, –, V, – | 4 |
| E    | 4     | C, V, V, C | 4 |
| F    | 4     | V, C, V, C | 2 |

Source: `all heredox tiles vector.ai`, confirmed by Edward (2026-11-21).

## Play

1. Shuffle the tiles face down in a stack.
2. Veridican always moves first (Edward, 2026-11-21 14:34, for lore reasons).
   Which player is Veridican is chosen at random.
3. The first player places the top tile.
4. Players then alternate. On your turn, draw the top tile and place it, in any
   rotation, sharing an edge with a tile already down. Tiles are never flipped.
5. The finished layout must be a 4 × 4 square. The boundary floats: it is fixed
   only once the layout spans four cells in a direction. A placement that would
   make the layout wider or taller than four is illegal.
6. There is no matching requirement. Any tile may go next to any other.

## Scoring

A symbol is complete when two halves of the same kind meet across a shared
edge. Mixed joins, halves against a blank edge and halves on the outer edge
score nothing for anyone.

When all 16 tiles are down, the player with more complete copies of their own
symbol wins. Only the final count matters; margin does not. If the counts are
equal, the player who moved first wins: in effect the first player starts
half a symbol ahead. There are no draws.

## Rulings (Edward, 2026-11-21)

- Rotation is free on every placement; no flipping.
- The 4 × 4 boundary floats.
- Equal counts go to the player who moved first (changed 13:53; before that
  draws existed and counted as a loss for both computer players).
- The main game is Two tiles (changed 14:30; see below). The one-tile game
  described under Play is kept in the engines but is off the menu for now.

## Variants

- **Face-up stack:** the next tile is visible (only the next one, not the
  whole order).
- **Two tiles** (the main game; Edward, 2026-11-21): there are always two tiles
  to pick from. The first player draws two. On your turn you turn and place
  whichever you like; the other remains for your opponent, and a new tile is
  drawn to join it. When the stack is empty the last tile is the one left
  over. In the engines this is `variant = "two"`; a move is (x, y, r, which),
  where `which` 0 is the tile left by the previous player and 1 the new one.

## Engine conventions

- Symbols: 0 blank, 1 Castian, 2 Veridican.
- Edge order: 0 top, 1 right, 2 bottom, 3 left. y increases downwards.
- Rotation r turns the tile r quarter-turns clockwise.
- The first tile is at (0, 0). A move is (x, y, r).
- Legal moves are listed in order of y, then x, then r, with r limited to the
  tile's distinct rotations.
- A transcript is JSON: `{"deck": "…16 letters…", "first": "C"|"V",
  "moves": [[x, y, r], …], "score": [c, v]}`.

## Lore (Edward, 2026-11-21)

- Veridican is the old doctrine and Castian the new one: approximately a
  reformation. The game is a reminder of a historical schism.
- A completed symbol shows which doctrine has captured a particular diocese.
- A heredox (heresy / orthodox) is a diocese that remains split: a Castian
  half against a Veridican half.
- Veridican always moves first.
