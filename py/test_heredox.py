import random
import unittest

from heredox import (BLANK, C, V, TILES, COUNTS, DISTINCT_ROTATIONS, FULL_DECK,
                     Game, rotate, random_game, replay)


class TileSet(unittest.TestCase):
    def test_sixteen_tiles(self):
        self.assertEqual(len(FULL_DECK), 16)

    def test_every_tile_balanced(self):
        for e in TILES.values():
            self.assertEqual(e.count(C), e.count(V))

    def test_24_halves_each(self):
        for s in (C, V):
            self.assertEqual(sum(TILES[t].count(s) * n for t, n in COUNTS.items()), 24)

    def test_distinct_rotations(self):
        for t, e in TILES.items():
            self.assertEqual(len({rotate(e, r) for r in range(4)}), DISTINCT_ROTATIONS[t])

    def test_symbol_swap_maps_set_to_itself(self):
        def canon(e):
            return min(rotate(e, r) for r in range(4))
        swap = {BLANK: BLANK, C: V, V: C}
        before = sorted(canon(TILES[t]) for t in FULL_DECK)
        after = sorted(canon(tuple(swap[s] for s in TILES[t])) for t in FULL_DECK)
        self.assertEqual(before, after)

    def test_rotate_clockwise(self):
        # top goes to right
        self.assertEqual(rotate((C, BLANK, BLANK, BLANK), 1), (BLANK, C, BLANK, BLANK))


class Play(unittest.TestCase):
    def test_first_move_is_origin(self):
        g = Game(FULL_DECK)
        self.assertEqual(g.legal_cells(), [(0, 0)])

    def test_floating_boundary(self):
        g = Game(FULL_DECK)
        g.play(0, 0, 0)
        self.assertEqual(g.legal_cells(), [(0, -1), (-1, 0), (1, 0), (0, 1)])
        g.play(1, 0, 0)
        g.play(2, 0, 0)
        g.play(3, 0, 0)
        cells = g.legal_cells()
        self.assertNotIn((4, 0), cells)   # would be five wide
        self.assertNotIn((-1, 0), cells)
        self.assertIn((0, -1), cells)
        self.assertIn((3, 1), cells)

    def test_illegal_cell_rejected(self):
        g = Game(FULL_DECK)
        g.play(0, 0, 0)
        with self.assertRaises(AssertionError):
            g.play(2, 0, 0)

    def test_scoring(self):
        # deck starts A, A: A is (C, V, -, -)
        g = Game(FULL_DECK)
        g.play(0, 0, 0)            # right edge V
        g.play(1, 0, 2)            # rotated twice: (-, -, C, V): left edge V
        self.assertEqual(g.score(), (0, 1))
        g2 = Game(FULL_DECK)
        g2.play(0, 0, 0)           # right edge V
        g2.play(1, 0, 3)           # rotated 3: (V, -, -, C): left edge C, mixed
        self.assertEqual(g2.score(), (0, 0))

    def test_level_scores_go_to_first_player(self):
        g = Game(FULL_DECK, first="V")
        g.play(0, 0, 0)
        self.assertEqual(g.score(), (0, 0))
        self.assertEqual(g.winner(), "V")

    def test_turn_order(self):
        g = Game(FULL_DECK, first="V")
        self.assertEqual(g.to_move, "V")
        g.play(0, 0, 0)
        self.assertEqual(g.to_move, "C")

    def test_random_games_finish_square(self):
        rng = random.Random(7)
        for _ in range(300):
            g = random_game(rng)
            x0, x1, y0, y1 = g.bounds()
            self.assertEqual((x1 - x0, y1 - y0), (3, 3))
            self.assertEqual(len(g.board), 16)
            c, v = g.score()
            self.assertLessEqual(c, 12)
            self.assertLessEqual(v, 12)
            self.assertEqual(replay(g.transcript()).score(), (c, v))


class TwoInHand(unittest.TestCase):
    def test_hands(self):
        g = Game(FULL_DECK, variant="two")     # deck starts A A A B ...
        self.assertEqual(g.hand, ["A", "A"])
        g.play(0, 0, 0, 1)
        self.assertEqual(g.hand, ["A", "A"])   # handed A, drew the third A
        g.play(1, 0, 0, 0)
        self.assertEqual(g.hand, ["A", "B"])   # handed A, drew B

    def test_last_two_turns(self):
        rng = random.Random(3)
        for _ in range(200):
            g = Game("".join(rng.sample(FULL_DECK, 16)), variant="two")
            sizes = []
            while not g.over:
                sizes.append(len(g.hand))
                g.play(*rng.choice(g.legal_moves()))
            self.assertEqual(sizes, [2] * 15 + [1])   # the last tile is the one you are handed
            self.assertEqual(len(g.board), 16)
            self.assertEqual(g.hand, [])
            self.assertEqual(replay(g.transcript()).score(), g.score())


class ThreeTiles(unittest.TestCase):
    def test_hands(self):
        g = Game(FULL_DECK, variant="three")   # deck starts A A A B B B ...
        self.assertEqual(g.hand, ["A", "A", "A"])
        g.play(0, 0, 0, 2)
        self.assertEqual(g.hand, ["A", "A", "B"])

    def test_sizes(self):
        rng = random.Random(4)
        for _ in range(200):
            g = Game("".join(rng.sample(FULL_DECK, 16)), variant="three")
            sizes = []
            while not g.over:
                sizes.append(len(g.hand))
                g.play(*rng.choice(g.legal_moves()))
            self.assertEqual(sizes, [3] * 14 + [2, 1])
            self.assertEqual(len(g.board), 16)
            self.assertEqual(replay(g.transcript()).score(), g.score())


if __name__ == "__main__":
    unittest.main()
