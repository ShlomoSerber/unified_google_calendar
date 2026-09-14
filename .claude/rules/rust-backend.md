---
paths:
  - "src-tauri/**"
---

# Rust backend rules

- Every module has a header comment pointing to the section of `docs/02-arquitectura.md` or `docs/05-sincronizacion.md` it implements.
- Database access only through `db::call`. Never hold a `rusqlite::Connection` in Tauri state or in a `Mutex`.
- Google API calls only in `google/`. `sync/` and `commands/` never build URLs or JSON for Google.
- `events.list` always uses exactly `singleEvents=false&showDeleted=true&maxResults=2500` plus `pageToken` or `syncToken`. Never add `timeMin`, `timeMax`, `updatedMin`, `orderBy` or `q`. See `docs/05` section 2.2.
- `events.insert` and `events.update` always send `conferenceDataVersion=1`. `update` builds the body from `events.raw`.
- Tokens: read and written only in `auth/token_store.rs`. No token appears in `tracing` output, in `AppError` messages, or in fixtures.
- Errors: return `AppError`; convert to `String` only at the `#[tauri::command]` boundary with `user_message()`.
- Long-running work goes through `tauri::async_runtime::spawn`. Blocking work through `spawn_blocking`.
- Every D-Bus name, interface and method for Evolution Data Server or GNOME Online Accounts must match `docs/06-integracion-gnome.md` section 4.3. Do not guess suffixes; discover them with `ListNames`.
- Tests for network code use `wiremock` with the JSON fixtures in `src-tauri/tests/fixtures/google/`. Do not call the real API in tests.
- `cargo clippy -- -D warnings` must pass before every commit.
