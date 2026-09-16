---
paths:
  - "docs/design/**"
  - "src/styles/**"
---

# Design tokens (Material 3, since 2026-09-16)

- `docs/design/m3/theme.json` is the single source of truth for every visual value (`docs/11-material3.md` section 3). A value is allowed there only if it is (a) a Material 3 system token copied from `material-components/material-web` `tokens/versions/v0_192`, (b) a value from an m3.material.io component spec page cited in a sibling `source` key, or (c) an app layout size on the 4 px grid or a reference to a `--md-sys-*` token, with a `<name>_note` saying why.
- The color scheme is never typed: `scripts/gen-m3-tokens.mjs` generates every `--md-sys-color-*` role from `color.seed` and `color.variant` with `@material/material-color-utilities`. To change the palette, change the seed or the variant and regenerate. Dark values are generated the same way; never derive dark from light by hand.
- Calendar and event chip colors come from `color.custom_colors` through the same generator (`src/styles/palette.ts`). Do not add per-color overrides in CSS.
- After editing `theme.json`: `node scripts/gen-m3-tokens.mjs && node scripts/check-tokens.mjs`, and commit the JSON together with the generated `src/styles/tokens.css`, `typescale.css`, `layout.ts`, `motion.ts`, `palette.ts`. Never edit a generated file by hand; `scripts/check-m3.sh` regenerates and compares.
- `docs/design/m3/mockup.html` is the decision mockup (option 3 is the chosen one). It is a reference, not code: nothing is copied from it into `src/`.
- `docs/design/google/` (before M5: `docs/design/measurements/`, `tokens.json`, `token-spec.json`) is the historical record of calendar.google.com measured on 2026-09-14. It rules nothing. Do not add to it and do not read it to decide a value.
- Before the transition removes it, `src/styles/legacy.css` and `src/styles/measured.css` exist only so unmigrated components keep compiling. Never add a rule or a variable to them.
