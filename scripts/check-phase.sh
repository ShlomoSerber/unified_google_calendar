#!/usr/bin/env bash
# Gate to close phase N of docs/08-plan-de-implementacion.md.
# Usage: bash scripts/check-phase.sh <N>
set -uo pipefail
N="${1:-}"; [ -z "$N" ] && { echo "usage: check-phase.sh <phase-number>"; exit 2; }
cd "$(dirname "$0")/.."
fail=0
step() { printf '\n== %s\n' "$1"; }
run() { if "$@"; then echo "ok: $*"; else echo "FAIL: $*"; fail=1; fi; }
need_file() { if [ -e "$1" ]; then echo "ok: $1"; else echo "FAIL: missing $1"; fail=1; fi; }

step "common checks"
need_file src-tauri/Cargo.toml
need_file src-tauri/tauri.conf.json
need_file package.json
run cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
run cargo test --manifest-path src-tauri/Cargo.toml
run npm run --silent lint
run npm run --silent typecheck
run npm test --silent -- --run

step "phase $N specific checks"
case "$N" in
  0) need_file src-tauri/src/error.rs; need_file src-tauri/src/config.rs ;;
  1) need_file src-tauri/src/db/schema/0001_init.sql; need_file src-tauri/src/recurrence/expand.rs
     need_file src-tauri/src/commands/types.rs; need_file src/types/ipc.ts ;;
  2) need_file src-tauri/src/auth/oauth.rs; need_file src-tauri/src/auth/token_store.rs
     need_file src-tauri/src/sync/full.rs; need_file src-tauri/src/sync/incremental.rs
     need_file src-tauri/tests/fixtures/google ;;
  3) need_file src-tauri/src/google/events.rs
     grep -q "conferenceDataVersion" src-tauri/src/google/events.rs && echo "ok: conferenceDataVersion present" || { echo "FAIL: conferenceDataVersion missing in google/events.rs"; fail=1; } ;;
  4|7) echo "phases 4 and 7 (Google-replica UI) were superseded by the Material 3 transition: bash scripts/check-m3.sh" ;;
  5) need_file src-tauri/src/webhook/server.rs; need_file src-tauri/src/sync/push.rs; need_file src-tauri/src/sync/sleep.rs ;;
  6) need_file src-tauri/src/eds/mirror.rs; need_file src-tauri/src/reminders/notify.rs; need_file src-tauri/src/tray.rs
     run bash scripts/check-eds.sh ;;
  8) ls src-tauri/target/release/bundle/deb/*.deb >/dev/null 2>&1 && echo "ok: .deb present" || { echo "FAIL: no .deb in src-tauri/target/release/bundle/deb"; fail=1; }
     need_file README.md; need_file scripts/uninstall-data.sh
     grep -q "| 8 |" docs/99-decisiones.md && echo "ok: RAM row for phase 8" || { echo "FAIL: add the phase 8 RAM row to docs/99-decisiones.md"; fail=1; } ;;
  *) echo "no specific checks for phase $N" ;;
esac

step "result"
if [ $fail -eq 0 ]; then echo "PHASE $N GATE PASSED"; else echo "PHASE $N GATE FAILED"; fi
exit $fail
