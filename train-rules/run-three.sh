#!/bin/sh
# Trains a fresh player on the game with THREE tiles to choose from each turn
# (two are left for the opponent, who draws one more), then plays it against
# itself. Compare the FINAL line with "FINAL None" from the two-tile run.
#
#   sh run-three.sh            40 minutes, every core
#   sh run-three.sh 60         60 minutes
#   sh run-three.sh 40 castian 40 minutes, level games to Castian
#
# Results are appended to exp.log. Send back exp.log.
MINUTES=${1:-40}
cd "$(dirname "$0")" || exit 1
chmod +x ./heredox-exp 2>/dev/null
exec ./heredox-exp none "$MINUTES" "$(nproc 2>/dev/null || echo 4)" 10000 three $2
