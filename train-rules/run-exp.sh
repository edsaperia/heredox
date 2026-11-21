#!/bin/sh
# Compares three versions of the two-tile game by training a fresh player on
# each and playing it against itself:
#   none    today's rules
#   reward  split a diocese and your opponent's next turn has no choice of tile
#   cost    split a diocese and your own next turn has no choice of tile
#
#   sh run-exp.sh              2 hours per version (6 hours in all), every core
#   sh run-exp.sh 3            3 hours per version
#   sh run-exp.sh 3 8          3 hours per version on 8 threads
#   sh run-exp.sh 2 0 castian  as the first, but level games go to Castian
#
# Results are appended to exp.log as it goes; the lines starting FINAL are
# the ones that matter. Send back exp.log.
HOURS=${1:-2}
THREADS=${2:-0}
[ "$THREADS" = "0" ] && THREADS=$(nproc 2>/dev/null || echo 4)
cd "$(dirname "$0")" || exit 1
chmod +x ./heredox-exp 2>/dev/null
MINUTES=$(awk "BEGIN { print $HOURS * 60 }")
for RULE in none reward cost; do
  ./heredox-exp "$RULE" "$MINUTES" "$THREADS" "${GAMES:-10000}" $3 || exit 1
done
echo "done: send back exp.log"
