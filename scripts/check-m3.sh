#!/usr/bin/env bash
# Gate of the Material 3 transition (docs/12-plan-m3.md section 7). Passes only when every task
# of the plan is done: the M3 pipeline is the sole source of visual values, the Google-replica
# tooling is gone, and the usual checks are green.
# Usage: bash scripts/check-m3.sh
set -uo pipefail
cd "$(dirname "$0")/.."
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
fail=0
step() { printf '\n== %s\n' "$1"; }
run() { if "$@"; then echo "ok: $*"; else echo "FAIL: $*"; fail=1; fi; }
need_file() { if [ -e "$1" ]; then echo "ok: $1"; else echo "FAIL: missing $1"; fail=1; fi; }
must_not_exist() { if [ -e "$1" ]; then echo "FAIL: $1 must be deleted (docs/99, 2026-09-16)"; fail=1; else echo "ok: gone $1"; fi; }
must_not_grep() { if grep -rqE "$1" $2 2>/dev/null; then echo "FAIL: '$1' still present in $2"; grep -rnE "$1" $2 | head -5; fail=1; else echo "ok: no '$1' in $2"; fi; }

step "generated tokens are current"
tmp="$(mktemp -d)"
run node scripts/gen-m3-tokens.mjs >/dev/null
UGC_TOKENS_OUT="$tmp" node scripts/gen-m3-tokens.mjs >/dev/null
for f in tokens.css typescale.css layout.ts motion.ts palette.ts; do
  if cmp -s "$tmp/$f" "src/styles/$f"; then echo "ok: src/styles/$f matches theme.json"; else echo "FAIL: src/styles/$f differs from the generator output"; fail=1; fi
done
rm -rf "$tmp"
run node scripts/check-tokens.mjs

step "Google-replica tooling removed"
must_not_exist src/styles/measured.css
must_not_exist src/styles/legacy.css
must_not_exist docs/design/tokens.json
must_not_exist docs/design/token-spec.json
must_not_exist docs/design/measurements
must_not_exist scripts/measure
must_not_exist scripts/gen-measured-css.mjs
must_not_exist scripts/gen-tokens.mjs
must_not_exist src/dev
must_not_exist src/event/QuickCreate.tsx
must_not_exist src/event/QuickCreate.css
must_not_exist src-tauri/src/commands/dev.rs
must_not_exist .claude/skills/measure-component
must_not_exist src-tauri/resources/fonts/GoogleMaterialIcons-subset.woff2
need_file docs/design/google/README.md
need_file src-tauri/resources/fonts/MaterialSymbolsOutlined-subset.woff2
need_file docs/design/m3/icons.txt

step "no runtime dependency on Google's fonts"
must_not_grep "fonts.googleapis.com|fonts.gstatic.com" "index.html src-tauri/tauri.conf.json src/styles/fonts.css"
must_not_grep "Google Sans Text" "src"

step "M3 is the only visual system"
grep -q '"@material/web"' package.json && echo "ok: @material/web in package.json" || { echo "FAIL: @material/web missing in package.json"; fail=1; }
grep -q '"react": "19' package.json && echo "ok: React 19" || { echo "FAIL: React 19 required"; fail=1; }
must_not_grep "installRipples|installTooltips|data-tooltip=|ugc-state" "src"
must_not_grep "var\\(--(color|font|layout|event|calendar|motion|ff|ep|qc)-" "src"
must_not_grep "measured\\.css|dev/measure|dev_dump|/dev/measure" "src src-tauri/src index.html"
need_file src/m3/register.ts
need_file src/m3/jsx.d.ts

step "common checks"
run cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
run cargo test --manifest-path src-tauri/Cargo.toml
run npm run --silent lint
run npm run --silent typecheck
run npm test --silent -- --run

step "release"
v="$(node -p "require('./package.json').version")"
if [ "$v" = "0.2.0" ]; then echo "ok: version 0.2.0"; else echo "FAIL: version is $v, expected 0.2.0 (scripts/bump-version.sh 0.2.0)"; fail=1; fi
ls src-tauri/target/release/bundle/deb/*_0.2.0_*.deb >/dev/null 2>&1 && echo "ok: .deb 0.2.0 present" || { echo "FAIL: no 0.2.0 .deb in src-tauri/target/release/bundle/deb"; fail=1; }
grep -q "| M6 |" docs/99-decisiones.md && echo "ok: RAM row for M6" || { echo "FAIL: add the M6 RAM row to docs/99-decisiones.md"; fail=1; }

step "result"
if [ $fail -eq 0 ]; then echo "M3 TRANSITION GATE PASSED"; else echo "M3 TRANSITION GATE FAILED"; fi
exit $fail
