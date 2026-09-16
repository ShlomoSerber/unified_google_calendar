# Unified Google Calendar

Desktop app for Ubuntu (GNOME, Wayland) that shows several Google accounts plus local events in
one calendar with a Material 3 UI (`@material/web`). Tauri 2 + Rust backend, React + TypeScript
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

Visual system (`docs/11-material3.md`): every colour, size, type role, shape, motion and elevation
value comes from `docs/design/m3/theme.json`; `node scripts/gen-m3-tokens.mjs` turns it into
`src/styles/{tokens.css,typescale.css,layout.ts,motion.ts,palette.ts}` and `node scripts/check-tokens.mjs`
fails on any literal value in a component stylesheet. `bash scripts/check-m3.sh` is the gate of the transition.

## Install

```bash
sudo apt install "./src-tauri/target/release/bundle/deb/Unified Google Calendar_0.2.0_amd64.deb"
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
- `docs/design/m3/theme.json` Material 3 tokens → `src/styles/` generated files (`node scripts/gen-m3-tokens.mjs`); `docs/design/google/` is the historical record of the previous Google-replica UI.
- `scripts/` phase gates, the M3 gate, RAM and EDS checks.
