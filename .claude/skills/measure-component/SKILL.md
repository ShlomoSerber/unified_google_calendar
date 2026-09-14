---
name: measure-component
description: Measure a Google Calendar web component (layout, computed styles, screenshots, light and dark) with the user and turn the result into design tokens. Use whenever a UI task needs a token that is missing or null in docs/design/tokens.json, and for tasks F4-T1 and F7-T1.
---

# Measure a component of Google Calendar

The user drives Chrome; you cannot log into their Google account. Your job is to give exact instructions, receive files, and process them.

Prerequisites: `docs/09-setup-usuario.md` section F done (Chrome profile `~/.chrome-measure`, settings of `docs/04` section 2.2, fixture events from `docs/design/fixture-events.md`).

Steps for one component `<name>` from the table in `docs/04-fidelidad-visual.md` section 4:

1. Tell the user, in one message, exactly this:
   - Open Chrome with the measurement profile at 1440×900, DevTools open, device toolbar in Responsive mode 1440×900, DPR 1.
   - Navigate to the state the component needs (view, day, dialog open) using the fixture events.
   - In Elements, select the root node described in the table so it becomes `$0`.
   - In Console, paste the whole content of `scripts/measure/dumpRegion.js`, press Enter, then run `dumpRegion($0)`. A JSON downloads.
   - Ctrl+Shift+P → "Capture node screenshot". A PNG downloads.
   - Repeat both with Google Calendar in the other theme (Settings → Appearance).
   - Move the four files to `docs/design/measurements/` named `<name>-light.json`, `<name>-light.png`, `<name>-dark.json`, `<name>-dark.png`, and say "done".
   For component 1 (theme tokens) use the custom-properties snippet in `docs/research/pixel-perfect.md` section 2(b) instead of `dumpRegion`, saving `gm3-light.json` and `gm3-dark.json`.
2. When the files exist, read the JSON. For each node with text or a role, extract: font family, size, weight, line height, letter spacing, color, background, borders, radius, padding, margin, shadow, and the rect. Identify which values are shared across nodes (theme tokens) and which are component specific.
3. Add the values to `docs/design/tokens.json` under `component.<name>` with a `"source": "measurements/<name>-light.json"` field, and any new theme colors under `color.light` / `color.dark`. Do not rename existing tokens. Do not round values except to two decimals.
4. Run `node scripts/gen-tokens.mjs && node scripts/check-tokens.mjs`.
5. Implement or adjust the component using only those tokens.
6. Ask the user to run the app (`npm run tauri dev`), open the inspector (Ctrl+Shift+I in dev), select the same root node, run the same `dumpRegion($0)` and node screenshot, in both themes, and save as `<name>-<theme>-app.json` and `.png`.
7. Run `node scripts/measure/diff-layout.mjs docs/design/measurements/<name>-light.json docs/design/measurements/<name>-light-app.json` and the dark pair. Fix every reported difference and repeat step 6 until the script reports 0 differences.
8. Run `npx odiff-bin docs/design/measurements/<name>-light.png docs/design/measurements/<name>-light-app.png docs/design/measurements/<name>-light-diff.png --antialiasing --threshold 0.1` and the dark pair. Record the percentage.
9. Write `docs/design/measurements/<name>.md` with: date, both diff-layout results, both odiff percentages, and any accepted difference with its reason.
10. Commit measurements, tokens, generated CSS and the component together.

A component is finished only when diff-layout is 0 and odiff is below 0.5% in both themes.
