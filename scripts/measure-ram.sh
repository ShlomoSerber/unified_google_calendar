#!/usr/bin/env bash
# Prints PSS (proportional set size) of the running app and its WebKit helper
# processes, read from /proc/<pid>/smaps_rollup. Run with the app open.
# Usage: bash scripts/measure-ram.sh [binary-name]
set -euo pipefail
BIN="${1:-unified-google-calendar}"

pss_kb() { awk '/^Pss:/ {print $2}' "/proc/$1/smaps_rollup" 2>/dev/null || echo 0; }

# pgrep -x cannot match names longer than 15 chars, so match the full command line
mapfile -t MAIN < <(pgrep -f -- "(^|/)${BIN}(\s|$)" || true)
if [ ${#MAIN[@]} -eq 0 ]; then
  echo "no process named $BIN is running (in dev the binary may be under src-tauri/target/debug/)"; exit 1
fi

printf '%-28s %8s %10s\n' process pid pss_mb
total=0
for pid in "${MAIN[@]}"; do
  kb=$(pss_kb "$pid"); total=$((total + kb))
  printf '%-28s %8s %10.1f\n' "$BIN" "$pid" "$(echo "$kb/1024" | bc -l)"
  # WebKit helpers are children of the main process
  for child in $(pgrep -P "$pid" || true); do
    name=$(cat "/proc/$child/comm" 2>/dev/null || echo '?')
    ckb=$(pss_kb "$child"); total=$((total + ckb))
    printf '%-28s %8s %10.1f\n' "$name" "$child" "$(echo "$ckb/1024" | bc -l)"
    for grand in $(pgrep -P "$child" || true); do
      gname=$(cat "/proc/$grand/comm" 2>/dev/null || echo '?')
      gkb=$(pss_kb "$grand"); total=$((total + gkb))
      printf '%-28s %8s %10.1f\n' "  $gname" "$grand" "$(echo "$gkb/1024" | bc -l)"
    done
  done
done
printf '%-28s %8s %10.1f\n' TOTAL '' "$(echo "$total/1024" | bc -l)"
echo "target: < 150 MB total with the window open (docs/02-arquitectura.md section 6)"
