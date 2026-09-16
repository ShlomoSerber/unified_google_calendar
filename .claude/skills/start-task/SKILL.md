---
name: start-task
description: Begin a task from docs/08-plan-de-implementacion.md. Use at the start of every implementation task (F<phase>-T<n>) to load the right documents and restate acceptance criteria before writing code.
---

# Start a task

Input: a task id like `F2-T4`. If none is given, find the first task in `docs/08-plan-de-implementacion.md` whose acceptance criteria are not yet met, checking `git log --oneline` for closed task ids.

Steps:

1. Read `docs/08-plan-de-implementacion.md` and locate the task. Copy its "Docs", "Pasos" and "Aceptación" lines.
2. Confirm the previous task in the same phase is committed (`git log --oneline | grep '<previous id>'`). If not, stop and tell the user.
3. Read every document the task lists under "Docs", plus `docs/02-arquitectura.md` section 4 (tree) and section 5 (IPC) if the task touches commands or the frontend.
4. If the task touches the frontend, read `docs/11-material3.md` (the visual system since 2026-09-16): every value comes from `docs/design/m3/theme.json`; add a missing layout token there, on the 4 px grid, with a note.
5. Write a short plan to the user: task id, acceptance criteria in your own words, files you will create or modify, tests you will write, anything you need from the user (see `docs/08` section 10).
6. Implement. Follow `CLAUDE.md` and the rules in `.claude/rules/`.
7. Run: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml && npm run lint && npm run typecheck && npm test`.
8. Verify each acceptance criterion explicitly and quote the evidence (test names, command output).
9. Commit: `git add -A && git commit -m "<task id>: <summary>"`.
10. Report to the user in the format of `CLAUDE.md` "Working protocol per task" step 7.

Never mark a task done with a failing check or an unverified criterion. If a criterion needs the user (manual test, sudo, real account), say so and stop there.
