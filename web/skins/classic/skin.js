// Classic skin: every word and every piece of art the page shows.
// Symbols here are placeholder drawings until Edward's vector files are in.
(function (root) {
  const ring = '<circle r="0.3" fill="none" stroke="currentColor" stroke-width="0.11"/><circle r="0.09" fill="currentColor"/>';
  const four = arm => [0, 90, 180, 270].map(a => `<path transform="rotate(${a})" d="${arm}"/>`).join('');
  const castianArm = 'M-0.07,-0.36 L-0.07,-0.52 Q-0.33,-0.52 -0.37,-0.86 Q-0.2,-0.99 0,-0.99 Q0.2,-0.99 0.37,-0.86 Q0.33,-0.52 0.07,-0.52 L0.07,-0.36 Z';
  const veridicanArm = 'M0,-1.12 L0.12,-0.74 L0.31,-0.63 L0.1,-0.56 L0.07,-0.36 L-0.07,-0.36 L-0.1,-0.56 L-0.31,-0.63 L-0.12,-0.74 Z';

  root.HeredoxSkin = {
    name: 'classic',
    // unit-size drawings centred on the origin, radius about 1, using currentColor
    symbols: {
      1: { color: '#f8e58c', art: `<g fill="currentColor">${four(castianArm)}${ring}</g>` },
      2: { color: '#5c0b0b', art: `<g fill="currentColor">${four(veridicanArm)}${ring}</g>` },
    },
    words: {
      title: 'Heredox',
      names: { C: 'Castian', V: 'Veridican' },
      levels: { novice: 'Novice', deacon: 'Deacon', abbot: 'Abbot' },
      onePlayer: 'One player',
      twoPlayers: 'Two players',
      howToPlay: 'How to play',
      endTurn: 'End turn',
      hint: 'Hint',
      menu: 'Menu',
      you: 'you',
      yourMove: 'Your move',
      thinking: n => `${n} is moving`,
      toMove: n => `${n} to move`,
      youWin: 'You win',
      youLose: 'You lose',
      wins: n => `${n} wins`,
      draw: 'Draw',
      playAgain: 'Play again',
      tilesLeft: n => (n === 1 ? '1 tile left' : `${n} tiles left`),
      rules: [
        'Players take turns placing tiles. All tiles after the first must be placed adjacent to another tile. The game ends when all tiles are laid, which must be in a four by four square. The player with the most complete copies of their symbol at the end of the game is the winner.',
        'Drag the tile onto the board. Tap it to turn it. Press End turn when you are happy.',
      ],
    },
  };
})(self);
