---
name: close-phase
description: Close an implementation phase of Unified Google Calendar. Use when every task of a phase in docs/08-plan-de-implementacion.md is committed, to run the gate checks, record RAM and deviations, and hand off to the user.
---

# Close a phase

Input: phase number `N`.

1. List the tasks of phase N from `docs/08-plan-de-implementacion.md` and confirm each has a commit (`git log --oneline | grep 'F<N>-T'`). If any is missing, stop.
2. Run `bash scripts/check-phase.sh <N>`. It runs clippy, tests, lint, typecheck and the phase-specific checks. Fix anything that fails; do not skip.
3. If the phase touched UI or sync (phases 2 to 7; the UI phases 4 and 7 were superseded by the Material 3 transition, gated by `bash scripts/check-m3.sh`), ask the user to start the app in week view with their accounts, then run `bash scripts/measure-ram.sh` and paste the table into `docs/99-decisiones.md` under "Mediciones de RAM por fase". If total PSS exceeds 150 MB, apply the measures of `docs/02-arquitectura.md` section 6 in order before closing.
4. Review the diff of the phase (`git diff <first commit of phase>^..HEAD --stat`) for anything that departs from `docs/`. Each departure must have an entry in `docs/99-decisiones.md`. Add missing ones.
5. Commit `F<N>: phase closed`.
6. Send the user a message with: what the phase delivered, the verification output summary, open risks, and the exact section of `docs/09-setup-usuario.md` they must complete before the next phase.
