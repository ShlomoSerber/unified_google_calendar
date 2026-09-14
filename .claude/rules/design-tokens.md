---
paths:
  - "docs/design/**"
  - "src/styles/**"
---

# Design tokens and measurements

- `docs/design/tokens.json` is the single source of truth for every visual value. A value enters this file only after being measured on calendar.google.com with the procedure in `docs/04-fidelidad-visual.md` section 3.
- Every token has a provenance: the measurement JSON file in `docs/design/measurements/` it was read from. Keep the `"source"` field next to each component group.
- Never overwrite a measured token with a guessed one. If two measurements disagree, record both in `docs/99-decisiones.md` and pick the newer one.
- Dark theme tokens are measured separately with Google Calendar in dark appearance. Never derive dark from light by inverting.
- After editing `tokens.json`, run `node scripts/gen-tokens.mjs` and `node scripts/check-tokens.mjs` and commit both the JSON and the generated CSS.
- Measurement files are named `<component>-<light|dark>.json`, `<component>-<light|dark>.png` for Google and `<component>-<light|dark>-app.json`, `-app.png` for our app. The comparison result goes in `<component>.md` with the `diff-layout` output and the `odiff` percentage.
