"""Heredox reference engine. Plain and slow on purpose; see RULES.md."""
import json
import random

BLANK, C, V = 0, 1, 2

TILES = {
    "A": (C, V, BLANK, BLANK),
    "B": (C, BLANK, BLANK, V),
    "D": (C, BLANK, V, BLANK),
    "E": (C, V, V, C),
    "F": (V, C, V, C),
}
COUNTS = {"A": 3, "B": 3, "D": 2, "E": 4, "F": 4}
DISTINCT_ROTATIONS = {"A": 4, "B": 4, "D": 4, "E": 4, "F": 2}
FULL_DECK = "".join(t * n for t, n in COUNTS.items())

# top, right, bottom, left; y increases downwards
DIRS = ((0, -1), (1, 0), (0, 1), (-1, 0))


def rotate(edges, r):
    """Turn a tile r quarter-turns clockwise."""
    return tuple(edges[(i - r) % 4] for i in range(4))


def shuffled_deck(rng):
    deck = list(FULL_DECK)
    rng.shuffle(deck)
    return "".join(deck)


class Game:
    """variant "single": draw one tile, place it.
    variant "two": two in hand. The first player draws two; each turn you
    place one of your tiles and hand the other to your opponent, who then
    draws one more. hand[0] is the tile you were handed, hand[1] the one you
    drew. A move is (x, y, r, which) with which indexing the hand."""

    def __init__(self, deck, first="C", variant="single"):
        assert sorted(deck) == sorted(FULL_DECK), "deck must be the 16 tiles"
        assert first in ("C", "V")
        assert variant in ("single", "two")
        self.deck = deck
        self.first = first
        self.variant = variant
        self.board = {}  # (x, y) -> edges as placed
        self.moves = []
        if variant == "two":
            self.hand = [deck[0], deck[1]]
            self.pos = 2
        else:
            self.hand = [deck[0]]
            self.pos = 1

    # -- state ------------------------------------------------------------
    @property
    def turn(self):
        return len(self.moves)

    @property
    def over(self):
        return self.turn == 16

    @property
    def tile(self):
        """Type letter of the tile to be placed now (single variant)."""
        return self.hand[0]

    @property
    def to_move(self):
        """Symbol ('C' or 'V') of the player to move."""
        other = "V" if self.first == "C" else "C"
        return self.first if self.turn % 2 == 0 else other

    def bounds(self):
        xs = [x for x, _ in self.board]
        ys = [y for _, y in self.board]
        return min(xs), max(xs), min(ys), max(ys)

    # -- moves ------------------------------------------------------------
    def legal_cells(self):
        if not self.board:
            return [(0, 0)]
        x0, x1, y0, y1 = self.bounds()
        cells = set()
        for (x, y) in self.board:
            for dx, dy in DIRS:
                c = (x + dx, y + dy)
                if c in self.board:
                    continue
                if max(x1, c[0]) - min(x0, c[0]) > 3:
                    continue
                if max(y1, c[1]) - min(y0, c[1]) > 3:
                    continue
                cells.add(c)
        return sorted(cells, key=lambda c: (c[1], c[0]))

    def legal_moves(self):
        if self.over:
            return []
        cells = self.legal_cells()
        if self.variant == "single":
            n = DISTINCT_ROTATIONS[self.tile]
            return [(x, y, r) for (x, y) in cells for r in range(n)]
        return [(x, y, r, w) for w, t in enumerate(self.hand)
                for (x, y) in cells for r in range(DISTINCT_ROTATIONS[t])]

    def play(self, x, y, r, which=0):
        assert not self.over
        assert (x, y) in self.legal_cells(), "illegal cell"
        assert 0 <= r < 4 and 0 <= which < len(self.hand)
        self.board[(x, y)] = rotate(TILES[self.hand[which]], r)
        if self.variant == "single":
            self.moves.append((x, y, r))
            self.hand = []
        else:
            self.moves.append((x, y, r, which))
            self.hand = [t for i, t in enumerate(self.hand) if i != which]
        if self.pos < 16 and len(self.moves) < 16:
            self.hand.append(self.deck[self.pos])
            self.pos += 1

    # -- scoring ----------------------------------------------------------
    def score(self):
        """(castian, veridican) complete symbols on the board so far."""
        s = [0, 0, 0]
        for (x, y), e in self.board.items():
            for d in (1, 2):  # right and bottom, so each join counts once
                dx, dy = DIRS[d]
                n = self.board.get((x + dx, y + dy))
                if n is not None and e[d] != BLANK and e[d] == n[(d + 2) % 4]:
                    s[e[d]] += 1
        return s[C], s[V]

    def winner(self):
        """'C', 'V' or None for a draw. Only meaningful when over."""
        c, v = self.score()
        return "C" if c > v else "V" if v > c else None

    # -- transcripts ------------------------------------------------------
    def transcript(self):
        return {"deck": self.deck, "first": self.first, "variant": self.variant,
                "moves": [list(m) for m in self.moves],
                "score": list(self.score())}


def replay(t):
    g = Game(t["deck"], t["first"], t.get("variant", "single"))
    for m in t["moves"]:
        g.play(*m)
    return g


def random_game(rng, variant="single"):
    g = Game(shuffled_deck(rng), rng.choice("CV"), variant)
    while not g.over:
        g.play(*rng.choice(g.legal_moves()))
    return g


if __name__ == "__main__":
    import sys
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 1000
    rng = random.Random(int(sys.argv[2]) if len(sys.argv) > 2 else 1)
    variant = sys.argv[3] if len(sys.argv) > 3 else "single"
    for _ in range(n):
        print(json.dumps(random_game(rng, variant).transcript(), separators=(",", ":")))
