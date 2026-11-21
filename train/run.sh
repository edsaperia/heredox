#!/bin/sh
# Trains the two-tile Heredox player. Plain sh, static binary: nothing to install.
#   sh run.sh            8 hours on every core
#   sh run.sh 12         12 hours
#   sh run.sh 12 6       12 hours on 6 threads
# It carries on from net.txt, saves as it goes, and can be stopped and
# restarted at any time. When it finishes, send back net.json.
HOURS=${1:-8}
THREADS=${2:-$(nproc 2>/dev/null || echo 4)}
cd "$(dirname "$0")" || exit 1
chmod +x ./heredox-train 2>/dev/null
exec ./heredox-train train "$THREADS" "$HOURS"
