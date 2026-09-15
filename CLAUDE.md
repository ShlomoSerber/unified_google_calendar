# Unified Google Calendar

Desktop app for Ubuntu that shows several Google accounts plus local personal events in a pixel-perfect replica of the Google Calendar web UI. Tauri 2 + Rust backend, React + TypeScript frontend, SQLite, Google Calendar API v3, push notifications through Tailscale Funnel, mirror into GNOME's calendar panel via Evolution Data Server.

The design phase is finished. Every architectural decision is already made and written in `docs/`. The implementation plan of `docs/08-plan-de-implementacion.md` is complete (phases F0–F8 closed on 2026-09-14) and the `.deb` is installed on the user's machine. The project is now in maintenance: the user reports what they see in the app and you fix it. Read `docs/10-mantenimiento.md` first: it holds the state, the known limitations, and the debugging procedure that keeps the app off the user's display.

## Read before doing anything

1. `docs/00-indice.md` — what each document is for and the reading rules.
2. `docs/01-requisitos.md` — scope. Anything listed as "versión 2" is out of scope, even if it looks easy.
3. `docs/02-arquitectura.md` — stack, modules, IPC contract, RAM budget, security.
4. `docs/10-mantenimiento.md` — current state, known limitations, how to reproduce and debug.
5. `docs/08-plan-de-implementacion.md` — the tasks already delivered, their acceptance criteria, and the documents each one names; reread the task a bug belongs to before touching it.

Then read the documents the task lists. Do not start coding a task without having read them in this session. Use the `start-task` skill.

## Non-negotiable rules

- **No invented UI measurements.** Every size, color, font, radius, shadow in the frontend comes from `docs/design/tokens.json`, which is filled only from real measurements of calendar.google.com following `docs/04-fidelidad-visual.md`. If a token you need is missing or `null`, stop and run the `measure-component` skill with the user. Never type a pixel value directly in CSS.
- **No decision changes without a record.** If you must deviate from `docs/02`, `03`, `05`, `06` or `07`, write the entry in `docs/99-decisiones.md` first, then change the code.
- **No new dependencies** beyond those listed in `docs/02-arquitectura.md` section 1 and `docs/08` without an entry in `docs/99-decisiones.md` stating why.
- **No product code for version-2 features.** Drag and drop, search, keyboard shortcuts, Google Tasks, .ics, autostart, auto-update: do not implement.
- **Never run `sudo`, `apt`, `tailscale`, or anything that changes system state.** Ask the user to run it and paste the output. Never modify `~/.config/goa-1.0` or Evolution sources that are not `ugc-*`, except through the app flows described in `docs/06`.
- **Never act on the user's real Google calendars** except the test calendar the user designates for manual verification. Never send invitations to real people in tests.
- **Tokens and secrets** never reach the webview, logs, fixtures, or commits. `~/.config/unified-google-calendar/oauth.json` is user-owned and is not read into the repo.
- **IPC types are mirrored by hand** in `src-tauri/src/commands/types.rs` and `src/types/ipc.ts`. Every change updates both and regenerates fixtures with `cargo test`.
- **Code and comments in English. Docs in Spanish.** Commit messages in English. Plan tasks used `F<phase>-T<task>: short summary`; maintenance work uses `fix:`, `docs:` or `chore:` followed by a short summary.
- **Never launch the app on the user's display** and never compile while an app instance is open. Run it on a private Xvfb (`docs/10-mantenimiento.md` section 5), build with `CARGO_BUILD_JOBS=2`, and kill processes by PID (`pgrep -f` with the bracket trick), never with `pkill -f`.

## Commands

```bash
npm run tauri dev                         # run the app in dev mode (Vite + Rust)
TAURI_LINUX_AYATANA_APPINDICATOR=1 npm run tauri build -- --bundles deb   # build the .deb
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml   # use CARGO_BUILD_JOBS=2 and close the dev app first: parallel rustc of tauri crates exhausts RAM
npm run lint && npm run typecheck && npm test
node scripts/gen-tokens.mjs               # docs/design/tokens.json -> src/styles/tokens.css
node scripts/check-tokens.mjs             # fails if CSS uses a null or missing token
node scripts/measure/diff-layout.mjs a.json b.json   # layout diff between Google and the app
bash scripts/measure-ram.sh               # PSS of the running app processes
bash scripts/check-eds.sh                 # verifies the Evolution Data Server mirror
bash scripts/check-phase.sh <N>           # gate to close phase N
```

## Working protocol per task

1. Announce the task id and restate its acceptance criteria in one or two lines.
2. Read the documents the task names.
3. Implement. Keep changes inside the modules the task names.
4. Write the tests the acceptance criteria ask for. Run the full check commands above.
5. If manual verification with the user is required, say exactly what they must do and what output you need back.
6. Commit with the task id.
7. Report: what was done, what the verification printed, what is left, and which step of `docs/09-setup-usuario.md` the user must do next if any.

Close a phase only through the `close-phase` skill.

## Conventions

- Rust: `rustfmt` defaults, `clippy -D warnings`, `thiserror` for `AppError`, `tracing` for logs, no `unwrap()` outside tests, `async` only where I/O happens, database access only through `db::call`.
- TypeScript: `strict: true`, no `any`, functional components, one component per file, CSS in plain `.css` files next to the component using only `var(--token)` values, no CSS-in-JS, no UI libraries.
- Tests: Rust unit tests next to the code, integration tests in `src-tauri/tests/` with `wiremock` fixtures under `src-tauri/tests/fixtures/`. Frontend tests with `vitest`.
- Dates: UTC integers in Rust and SQLite. IANA zone names as strings. The frontend formats with `date-fns` and never expands recurrences.
- Errors shown to the user are full sentences in English that say what happened and what to do.

## Repository map

See `docs/02-arquitectura.md` section 4 for the intended tree. Research reports with sources for every API claim are in `docs/research/`. Measurements and design tokens live in `docs/design/`.
