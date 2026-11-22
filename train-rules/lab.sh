#!/bin/sh
# Heredox lab: runs queued jobs one at a time, so runs can be started, stopped
# and read from the chat. Start it once and leave the window open:
#     wsl sh lab.sh
# A job is a file queue/<name>.job holding one line: a program in this folder
# (heredox-exp or heredox-train) and its arguments. THREADS stands for the
# number of cores. Create a file called "stop" to end the current job.
cd "$(dirname "$0")" || exit 1
mkdir -p queue done
chmod +x ./heredox-exp ./heredox-train 2>/dev/null
N=$(nproc 2>/dev/null || echo 4)
echo "$(date '+%F %T') lab started on $N threads; waiting for jobs. Leave this window open; Ctrl+C quits." | tee -a lab.log
while :; do
  JOB=$(ls queue/*.job 2>/dev/null | head -n 1)
  if [ -z "$JOB" ]; then
    echo "idle $(date '+%F %T')" > status
    sleep 5
    continue
  fi
  NAME=$(basename "$JOB")
  LINE=$(head -n 1 "$JOB" | tr -d '\r' | sed "s/THREADS/$N/g")
  set -- $LINE
  PROG=$1
  shift
  case "$PROG" in
    heredox-exp|heredox-train) ;;
    *)
      echo "$(date '+%F %T') skipped $NAME: unknown program '$PROG'" | tee -a lab.log
      mv "$JOB" done/
      continue
      ;;
  esac
  echo "$(date '+%F %T') started $NAME: $PROG $*" | tee -a lab.log
  echo "running $NAME ($PROG $*) since $(date '+%F %T')" > status
  ./"$PROG" "$@" 2>&1 | tee -a lab.out &
  while [ -n "$(jobs -r 2>/dev/null)" ] || pgrep -x "$PROG" >/dev/null 2>&1; do
    if [ -f stop ]; then
      pkill -x "$PROG"
      rm -f stop
      echo "$(date '+%F %T') stopped $NAME on request" | tee -a lab.log
    fi
    sleep 5
  done
  wait
  echo "$(date '+%F %T') finished $NAME" | tee -a lab.log
  mv "$JOB" done/
done
