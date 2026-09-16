---
name: m3-transition
description: Execute the full transition of the frontend to Material 3 (docs/12-plan-m3.md) in one session, task by task, until the .deb is built. Use when the user says "hace la transición a M3", "transition to M3", "empezá el plan M3" or names a task M<n>-T<n>.
---

# Transition to Material 3

The decision is taken and recorded (`docs/99-decisiones.md`, 2026-09-16). The design system is
specified in `docs/11-material3.md`; the tasks, their order and acceptance criteria are in
`docs/12-plan-m3.md`. Your job is to execute the plan completely, in order, in this session.

## Before the first task

1. Read, in this order, completely: `CLAUDE.md`, `docs/11-material3.md`, `docs/12-plan-m3.md`,
   `docs/design/m3/theme.json`, the 2026-09-16 entry of `docs/99-decisiones.md`,
   `docs/10-mantenimiento.md` section 5 (how to run the app on a private Xvfb),
   `.claude/rules/frontend.md` and `.claude/rules/design-tokens.md`.
2. Open `docs/design/m3/mockup.html` mentally as the visual target (option 3). If you can drive a
   browser (the `ccdp-display` plugin), load it there and look at it in light and dark.
3. Check the working tree is clean and on `main`, and that no app instance is running
   (`pgrep -f "unified-google-calenda[r]"`). If the user's installed app is open, work anyway:
   builds and Xvfb runs never touch their display, and the debug instance uses its own
   `XDG_*` directories (`docs/10` section 5).
4. Find the first task of `docs/12` without a commit (`git log --oneline | grep 'M<n>-T'`). Start there.
5. Tell the user in two lines which task you start with and that you will run the whole plan
   without stopping unless a criterion needs them (`docs/12` section 8).

## Per task

1. Restate the task id and its acceptance criteria in one or two lines.
2. Implement exactly what the task lists, inside the files it names. Every visual value comes from
   `var(--md-sys-*)`, `var(--ugc-*)` or a `<md-*>` component attribute (`docs/11` section 8). If a
   layout value is missing, add it to `docs/design/m3/theme.json` on the 4 px grid with a
   `<name>_note`, regenerate, and continue. Never type a px or a color in a component stylesheet.
3. Run the checks the task lists. Always at least:
   `node scripts/gen-m3-tokens.mjs && node scripts/check-tokens.mjs && npm run lint && npm run typecheck && npm test`.
   Rust checks (`cargo clippy -D warnings`, `cargo test`, with `CARGO_BUILD_JOBS=2`) only when the
   task touches `src-tauri/`.
4. For every task that changes what the user sees, run the app on Xvfb (`docs/10` section 5) with
   the debug `XDG_*` directories, take a `scrot` screenshot in light and in dark
   (`GTK_THEME=Adwaita:dark` for dark), look at both with the Read tool, and fix what is wrong
   before committing. Kill the processes by PID afterwards.
5. Commit: `git add -A && git commit -m "M<n>-T<m>: <summary>"`. One commit per task minimum.
6. Do not report between tasks unless something blocks you. Keep going.

## Closing

Follow `docs/12` section 7: run `bash scripts/check-m3.sh`, measure RAM on Xvfb, bump the version
to 0.2.0 with `scripts/bump-version.sh`, build the `.deb` with
`TAURI_LINUX_AYATANA_APPINDICATOR=1 CARGO_BUILD_JOBS=2 npm run tauri build -- --bundles deb`,
update `docs/10-mantenimiento.md` section 1 and the RAM table of `docs/99`, commit
`M6: transition closed`, and send the user the closing message of `docs/12` section 7 (what to
install, what changed, what to look at first).

## Rules that still apply

Everything in `CLAUDE.md`: no `sudo`, never the user's display, `CARGO_BUILD_JOBS=2`, kill by PID,
no version-2 features, no touching real calendars, tokens and secrets never in the repo, English in
code and commits, Spanish in docs, no dead bytes left behind.
