# Unified Google Calendar

Desktop app for Ubuntu (GNOME, Wayland) that shows several Google accounts plus local events in
one calendar with the Google Calendar web UI. Tauri 2 + Rust backend, React + TypeScript
frontend, SQLite, Google Calendar API v3 with push notifications through Tailscale Funnel, and a
mirror into GNOME's calendar panel through Evolution Data Server.

The design, data model, sync rules and implementation plan live in `docs/` (Spanish). Start with
`docs/00-indice.md`.

## Build

```bash
# system packages (once): docs/07-empaquetado.md section 1
npm ci
export PATH="$HOME/.cargo/bin:$PATH"
TAURI_LINUX_AYATANA_APPINDICATOR=1 npm run tauri build -- --bundles deb
ls src-tauri/target/release/bundle/deb/
```

Development: `npm run tauri dev`. Checks: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`,
`cargo test --manifest-path src-tauri/Cargo.toml`, `npm run lint && npm run typecheck && npm test`.
Phase gates: `bash scripts/check-phase.sh <N>`.

Pixel fidelity (`docs/04-fidelidad-visual.md`): `node scripts/measure/capture.mjs` dumps
calendar.google.com components with the `~/.chrome-measure` profile, `scripts/measure/extract-tokens.mjs`
+ `scripts/gen-tokens.mjs` + `scripts/gen-measured-css.mjs` turn the dumps into `docs/design/tokens.json`
and the generated stylesheets, `scripts/measure/app-capture.mjs` dumps the app on a private Xvfb
display and `scripts/measure/report.mjs [--gate 4|7]` compares both sides. Results per component
are in `docs/design/measurements/<component>.md`. Motion (`docs/04` section 10): `node scripts/measure/animations.mjs`
records every Web Animation calendar.google.com runs per scenario into `docs/design/measurements/animations-<theme>.json`;
the derived tokens live in `tokens.json` under `component.motion`.

## Install

```bash
sudo apt install "./src-tauri/target/release/bundle/deb/Unified Google Calendar_0.1.0_amd64.deb"
```

Before the first run create `~/.config/unified-google-calendar/oauth.json` with your Google Cloud
OAuth client (`docs/09-setup-usuario.md` section A). Push notifications need Tailscale Funnel
(section B); without it the app polls every 60 s. The tray icon needs the
`ubuntu-appindicators@ubuntu.com` GNOME extension enabled.

## Uninstall

```bash
sudo apt remove unified-google-calendar
bash scripts/uninstall-data.sh   # removes config, database, tokens, logs and the EDS mirror sources
```

## Layout

- `src-tauri/` Rust backend: `auth`, `google`, `sync`, `db`, `recurrence`, `commands`, `reminders`, `tray`, `webhook`, `eds`.
- `src/` React frontend: `app`, `views`, `event`, `components`, `ipc`, `state`, `styles`, `types`.
- `docs/design/tokens.json` measured design tokens → `src/styles/tokens.css` (`node scripts/gen-tokens.mjs`).
- `scripts/` phase gates, RAM and EDS checks, measurement tools.
