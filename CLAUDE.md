# Unified Google Calendar

Desktop app for Ubuntu that shows several Google accounts plus local personal events in one calendar. Tauri 2 + Rust backend, React + TypeScript frontend, SQLite, Google Calendar API v3, push notifications through Tailscale Funnel, mirror into GNOME's calendar panel via Evolution Data Server.

The design phase is finished and every architectural decision is written in `docs/`. The implementation plan of `docs/08-plan-de-implementacion.md` is complete (F0–F8 closed on 2026-09-14); the `.deb` 0.2.0 (Material 3) is the current delivery.

**State on 2026-09-16: the UI is Material 3.** The transition from the pixel replica of calendar.google.com was decided (`docs/99-decisiones.md`, entry 2026-09-16), specified in `docs/11-material3.md`, planned in `docs/12-plan-m3.md` and executed the same day (commits `M0-T1` to `M6`, `.deb` 0.2.0). Everything the user reports now is maintenance: read `docs/10-mantenimiento.md` first.

## Read before doing anything

1. `docs/00-indice.md` — what each document is for and the reading rules.
2. `docs/01-requisitos.md` — scope. Anything listed as "versión 2" is out of scope, even if it looks easy.
3. `docs/02-arquitectura.md` — stack, modules, IPC contract, RAM budget, security.
4. `docs/11-material3.md` — the visual system (tokens, fonts, `@material/web` in React 19, component mapping, CSS rules). `docs/04-fidelidad-visual.md` is historical.
5. `docs/10-mantenimiento.md` — current state, known limitations, how to run and debug the app off the user's display.
6. For maintenance of already delivered work: the task in `docs/08` the bug belongs to, or the task in `docs/12-plan-m3.md` for anything visual.

Then read the documents the task lists. Do not start coding a task without having read them in this session.

## Non-negotiable rules

- **No invented UI values.** Every size, color, font, radius, shadow and duration in the frontend is a `var(--md-sys-*)`, `var(--ugc-*)` or `<md-*>` attribute that comes from `docs/design/m3/theme.json` through `node scripts/gen-m3-tokens.mjs`. A value enters `theme.json` only if it is a Material 3 system token, a value from a cited m3.material.io spec page, or an app layout size on the 4 px grid with a `<name>_note`. Never type a pixel value or a color in a component stylesheet.
- **No decision changes without a record.** If you must deviate from `docs/02`, `03`, `05`, `06`, `07` or `11`, write the entry in `docs/99-decisiones.md` first, then change the code.
- **No new dependencies** beyond those listed in `docs/02-arquitectura.md` section 1, `docs/08` and `docs/11` section 2 without an entry in `docs/99-decisiones.md` stating why.
- **No product code for version-2 features.** Drag and drop, search, keyboard shortcuts, Google Tasks, .ics, autostart, auto-update: do not implement. Focus rings and menu keyboard navigation that `@material/web` brings are not "keyboard shortcuts".
- **Never run `sudo`, `apt`, `tailscale`, or anything that changes system state.** Ask the user to run it and paste the output. Never modify `~/.config/goa-1.0` or Evolution sources that are not `ugc-*`, except through the app flows described in `docs/06`.
- **Never act on the user's real Google calendars.** Verify with the local calendar (account `local`) or the debug instance's own data directory. Never send invitations to real people in tests.
- **Tokens and secrets** never reach the webview, logs, fixtures, or commits. `~/.config/unified-google-calendar/oauth.json` is user-owned and is not read into the repo.
- **IPC types are mirrored by hand** in `src-tauri/src/commands/types.rs` and `src/types/ipc.ts`. Every change updates both and regenerates fixtures with `cargo test`.
- **Code and comments in English. Docs in Spanish.** Commit messages in English. Plan tasks used `F<phase>-T<task>: short summary`; the M3 transition uses `M<phase>-T<task>: short summary`; maintenance work uses `fix:`, `docs:` or `chore:` followed by a short summary.
- **Never launch the app on the user's display** and never compile while an app instance is open. Run it on a private Xvfb (`docs/10-mantenimiento.md` section 5), build with `CARGO_BUILD_JOBS=2`, and kill processes by PID (`pgrep -f` with the bracket trick), never with `pkill -f`.
- **Do not leave dead bytes on disk.** A cache that speeds up the next build stays (`src-tauri/target/` for the current toolchain and `Cargo.lock`, `node_modules`, Vite's cache). Files that will never be read again go: after a `rustup update` or a `Cargo.lock` change run `cargo clean --manifest-path src-tauri/Cargo.toml --profile dev` once the new build has succeeded, remove build artifacts of removed crates, examples or tests, and never leave scratch files, screenshots, measurements or logs in the repo: they belong in the scratchpad or in `docs/design/` when they are deliverables. Before deleting, confirm no app instance or build is running and report what was removed and how much space it freed.

## Commands

```bash
npm run tauri dev                         # run the app in dev mode (Vite + Rust); never on the user's display
TAURI_LINUX_AYATANA_APPINDICATOR=1 CARGO_BUILD_JOBS=2 npm run tauri build -- --bundles deb   # build the .deb
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml   # use CARGO_BUILD_JOBS=2 and close the dev app first: parallel rustc of tauri crates exhausts RAM
npm run lint && npm run typecheck && npm test
node scripts/gen-m3-tokens.mjs            # docs/design/m3/theme.json -> src/styles/{tokens.css,typescale.css,layout.ts,motion.ts,palette.ts}
node scripts/check-tokens.mjs             # fails if CSS uses a literal value or an undefined token
bash scripts/check-m3.sh                  # gate of the M3 transition (docs/12 section 7); run it after any UI change
bash scripts/measure-ram.sh               # PSS of the running app processes
bash scripts/check-eds.sh                 # verifies the Evolution Data Server mirror
bash scripts/bump-version.sh x.y.z        # same version in package.json, tauri.conf.json, Cargo.toml
```

## Working protocol per task

1. Announce the task id and restate its acceptance criteria in one or two lines.
2. Read the documents the task names.
3. Implement. Keep changes inside the modules the task names.
4. Write the tests the acceptance criteria ask for. Run the full check commands above.
5. If the task changes what the user sees, run the app on Xvfb, take screenshots in light and dark, look at them, and fix before committing.
6. If manual verification with the user is required, say exactly what they must do and what output you need back.
7. Commit with the task id.
8. Report: what was done, what the verification printed, what is left, and which step of `docs/09-setup-usuario.md` the user must do next if any.

Close a phase of `docs/08` only through the `close-phase` skill. The M3 transition closed through `docs/12` section 7 on 2026-09-16.

## Conventions

- Rust: `rustfmt` defaults, `clippy -D warnings`, `thiserror` for `AppError`, `tracing` for logs, no `unwrap()` outside tests, `async` only where I/O happens, database access only through `db::call`.
- TypeScript: `strict: true`, no `any`, functional components, one component per file, CSS in plain `.css` files next to the component using only `var(--token)` values, no CSS-in-JS. UI controls are `@material/web` custom elements used from JSX as `docs/11` section 5 says (properties for non-string values, lowercase `on<event>` listeners); the app's own interactive elements carry `<md-ripple>` and `<md-focus-ring>`.
- Tests: Rust unit tests next to the code, integration tests in `src-tauri/tests/` with `wiremock` fixtures under `src-tauri/tests/fixtures/`. Frontend tests with `vitest`; `src/m3/register.ts` is mocked in `vitest.setup.ts`, so tests never depend on the inside of a `<md-*>`.
- Dates: UTC integers in Rust and SQLite. IANA zone names as strings. The frontend formats with `date-fns` and never expands recurrences.
- Errors shown to the user are full sentences in English that say what happened and what to do.

## Repository map

See `docs/02-arquitectura.md` section 4 for the intended tree (`src/m3/` and `src/components/MonthGrid.tsx` were added by the transition). Research reports with sources for every API claim are in `docs/research/`. Design tokens and the decision mockup live in `docs/design/m3/`; `docs/design/google/` keeps the historical measurements of calendar.google.com.
