# 02 — Arquitectura

Fuentes de cada decisión: `docs/research/tauri.md`, `docs/research/google-calendar-api.md`, `docs/research/tailscale-push.md`. Quien implementa no cambia nada de este documento sin registrarlo en `99-decisiones.md`.

## 1. Stack

| Capa | Elección | Versión mínima | Motivo |
|---|---|---|---|
| Shell de escritorio | Tauri 2 | 2.4 | Un solo proceso Rust más el webview del sistema. Menor RAM que Electron para una UI modesta. Empaqueta `.deb` de fábrica. |
| Webview | WebKitGTK 4.1 | 2.50.4 ya instalado | Viene con Ubuntu 25.04. Soporta `:has()`, container queries y `color-mix()`. |
| Backend | Rust estable | 1.85 | Toda la lógica: OAuth, sync, SQLite, recurrencias, notificaciones, bandeja, webhook, espejo a GNOME. |
| Frontend | React 18 + TypeScript 5 + Vite 6 | | UI pura. No contiene lógica de sincronización ni estado de datos más allá de lo visible. |
| Base local | SQLite vía `rusqlite` con feature `bundled`, modo WAL | 0.40 | Sin dependencia de la libsqlite del sistema. Toda consulta pasa por comandos Rust. |
| HTTP cliente | `reqwest` con `rustls-tls` | 0.12 | Sin OpenSSL en runtime. |
| HTTP servidor local | `axum` | 0.8 | Recibe los push de Google en `127.0.0.1:8080`, expuesto por Tailscale Funnel. Comparte el runtime tokio. |
| Recurrencias | crate `rrule` | 0.13 | Expande RRULE/EXDATE/RDATE de eventos de Google y locales. |
| Fechas | `chrono` + `chrono-tz` | | Todo se guarda en UTC más el nombre IANA de la zona. |
| Notificaciones | `notify-rust` con backend `zbus` | 4 | D-Bus `org.freedesktop.Notifications`. Permite botones de acción. `tauri-plugin-notification` no soporta acciones en escritorio. |
| Bandeja | Tauri feature `tray-icon`, libayatana | | Compilar siempre con `TAURI_LINUX_AYATANA_APPINDICATOR=1`. |
| Cifrado de tokens | `aes-gcm` + `hkdf` + `sha2` | | Archivo cifrado propio, decisión del usuario. Ver sección 7. |
| D-Bus | `zbus` | 5 | Espejo a Evolution Data Server y señal de suspensión de `login1`. |
| Plugins Tauri | `tauri-plugin-single-instance`, `tauri-plugin-opener` | 2 | Instancia única. Abrir URLs en el navegador del sistema. |
| Fechas en frontend | `date-fns` 4 + `@date-fns/tz` | | Solo para formatear y navegar. La expansión de recurrencias nunca ocurre en el frontend. |

No se usan: `tauri-plugin-sql`, `tauri-plugin-notification`, `tauri-plugin-stronghold`, `tauri-plugin-oauth`, librerías de componentes de calendario. La UI es propia porque tiene que ser pixel a pixel igual a Google Calendar.

## 2. Procesos y responsabilidades

```
┌──────────────────────────────────────────────────────────────────┐
│ unified-google-calendar (proceso Rust, tokio)                    │
│                                                                  │
│  auth ──► google (cliente API) ◄── sync ◄── webhook (axum :8080) │
│                │                    │                            │
│                ▼                    ▼                            │
│              db (SQLite WAL) ◄── recurrence (expansión)          │
│                │                                                 │
│     ┌──────────┼──────────────┬──────────────┐                   │
│     ▼          ▼              ▼              ▼                   │
│  commands   reminders       tray           eds (D-Bus → EDS)     │
│  (IPC)      (notify-rust)   (set_title)                          │
└─────┬────────────────────────────────────────────────────────────┘
      │ invoke / emit
┌─────▼─────────────────────────┐
│ WebKitWebProcess (React UI)   │
└───────────────────────────────┘
```

- `auth`: flujo OAuth con PKCE y loopback en `127.0.0.1:0`. Guarda y refresca tokens. Nunca expone tokens al webview.
- `google`: cliente tipado de Calendar API v3. Solo este módulo conoce URLs, parámetros y JSON de Google.
- `sync`: orquesta full sync, incremental por sync token, canales push, catch-up al despertar y polling de respaldo. Escribe en `db` y emite `calendar:updated`.
- `webhook`: servidor axum en `127.0.0.1:8080`. Valida canal y token, encola un "tick" para `sync`. Sirve también el archivo de verificación de Search Console.
- `db`: esquema, migraciones y todas las consultas. Un hilo dedicado con canal de mensajes, porque `Connection` no es `Sync`.
- `recurrence`: expande masters en ocurrencias dentro de la ventana de datos. Se ejecuta al escribir un master, nunca al leer.
- `commands`: la única superficie IPC. Devuelve exactamente lo que la UI necesita para una vista.
- `reminders`: planificador que lee las próximas ocurrencias con recordatorio y dispara notificaciones de GNOME con acción "Join" cuando hay Meet.
- `tray`: ícono con menú "Open" y "Quit" y texto con el próximo evento. En GNOME el click izquierdo no emite evento, así que todo va por menú.
- `eds`: espeja calendarios y eventos en Evolution Data Server para el panel de GNOME Shell. Detalle en `06-integracion-gnome.md`.

## 3. Flujos principales

### 3.1 Arranque

1. `tauri-plugin-single-instance` como primer plugin. Si ya hay una instancia, muestra la ventana existente y sale.
2. Abrir SQLite, correr migraciones.
3. Cargar cuentas y tokens desde el archivo cifrado.
4. Levantar `webhook` en `127.0.0.1:8080`.
5. Crear la ventana principal y la bandeja.
6. Lanzar `sync::start(app)`: para cada cuenta, incremental si hay sync token, si no full sync. Luego asegurar canales push vigentes.
7. Lanzar `reminders::start(app)` y `eds::start(app)`.

### 3.2 Lectura de una vista

1. La UI llama `get_view({ from, to, timezone })` con el rango visible más 7 días de margen.
2. Rust consulta la tabla `occurrences` unida con `events`, `calendars` y `accounts`, aplica visibilidad y deduplica por `ical_uid`.
3. Devuelve un `ViewPayload` con ocurrencias ya posicionables: inicio y fin en UTC, `all_day`, color, cuenta, flags de conflicto, resumen de asistentes.
4. La UI no vuelve a pedir hasta que cambia el rango o recibe `calendar:updated` con un rango que interseca el visible.

### 3.3 Escritura

Escritura directa contra Google, sin cola offline. Pasos para crear un evento en una cuenta de Google:

1. La UI llama `create_event(draft)`.
2. `google::events_insert` con `conferenceDataVersion=1` si pidió Meet y `sendUpdates` según haya invitados.
3. Con la respuesta, `db::upsert_event` y `recurrence::expand` si es recurrente.
4. Emitir `calendar:updated` con el rango afectado. Devolver el evento creado a la UI.
5. Si Google responde error, devolver `AppError` con mensaje legible. La UI no cambia nada.

Para eventos locales el paso 2 no existe. Todo lo demás es idéntico.

### 3.4 Push recibido

1. Google hace `POST /gcal/webhook`. `webhook` valida `X-Goog-Channel-ID` contra la tabla `channels` y compara `X-Goog-Channel-Token` en tiempo constante.
2. Responde `200` de inmediato. Encola `SyncTick { account_id, calendar_id }` en un canal `mpsc` con deduplicación.
3. `sync` corre `events.list?syncToken` para ese calendario, aplica cambios, re-expande masters tocados, emite `calendar:updated`.

### 3.5 Suspensión y reconexión

1. `sync` escucha `org.freedesktop.login1.Manager.PrepareForSleep` por `zbus`. Al recibir `false` (despertar), corre incremental en todos los calendarios y revisa vencimiento de canales.
2. Un temporizador de respaldo corre incremental cada 10 minutos. Google avisa que los push no son 100% confiables.
3. Si no hay red, los errores se registran, la UI conserva lo último y el ícono de sync muestra estado de error. No hay reintento agresivo: se espera al siguiente tick o al despertar.

## 4. Árbol de directorios

```
unified_google_calendar/
├── CLAUDE.md
├── .claude/
│   ├── rules/            # reglas por área, ver harness
│   └── skills/           # procedimientos, ver harness
├── docs/                 # esta documentación
├── scripts/              # verificaciones por fase
├── package.json          # frontend + tauri cli
├── vite.config.ts
├── tsconfig.json
├── index.html
├── src/                  # frontend React
│   ├── main.tsx
│   ├── app/              # shell: topbar, sidebar, router de vistas
│   ├── views/            # day, week, month, agenda
│   ├── event/            # popup de detalle, quick create, formulario completo
│   ├── components/       # mini calendar, checkbox de calendario, chips, botones
│   ├── ipc/              # wrappers tipados de invoke y listen
│   ├── state/            # store de UI (zustand), solo estado de presentación
│   ├── styles/
│   │   ├── tokens.css    # generado desde docs/design/tokens.json, no editar a mano
│   │   ├── base.css
│   │   └── fonts.css
│   └── types/            # tipos compartidos, espejo de src-tauri/src/commands/types.rs
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/default.json
│   ├── icons/
│   ├── resources/
│   │   └── fonts/        # fuentes empaquetadas, ver 04
│   └── src/
│       ├── main.rs
│       ├── lib.rs        # builder de Tauri, plugins, setup
│       ├── error.rs      # AppError, conversión a String para IPC
│       ├── config.rs     # rutas, puerto, constantes
│       ├── auth/         # oauth.rs, token_store.rs, pkce.rs
│       ├── google/       # client.rs, events.rs, calendar_list.rs, channels.rs, colors.rs, types.rs
│       ├── sync/         # engine.rs, full.rs, incremental.rs, push.rs, poll.rs, sleep.rs
│       ├── db/           # mod.rs, schema.sql, migrations.rs, queries/*.rs, worker.rs
│       ├── recurrence/   # expand.rs, edit_scope.rs
│       ├── commands/     # view.rs, events.rs, accounts.rs, settings.rs, types.rs
│       ├── reminders/    # scheduler.rs, notify.rs
│       ├── tray.rs
│       ├── webhook/      # server.rs, verify_file.rs
│       └── eds/          # mirror.rs, dbus.rs
└── docs/design/
    ├── tokens.json       # medidas y colores medidos de Google Calendar
    └── measurements/     # dumps JSON crudos por componente
```

## 5. Contrato IPC

Todos los comandos devuelven `Result<T, String>`. El `String` es un mensaje legible en inglés para mostrar en la UI. Los tipos viven en `src-tauri/src/commands/types.rs` con `serde` y se copian a mano a `src/types/ipc.ts`. Un test de Rust serializa cada tipo a JSON de ejemplo y lo compara con `src/types/fixtures/*.json` para detectar desvíos.

Comandos de la versión 1:

| Comando | Entrada | Salida | Notas |
|---|---|---|---|
| `list_accounts` | | `AccountInfo[]` | Incluye estado de sync y error último. |
| `add_account` | | `AccountInfo` | Abre navegador, espera el loopback. Bloquea hasta terminar o timeout de 5 min. |
| `remove_account` | `account_id` | | Borra tokens, canales, calendarios, eventos. Llama `channels.stop`. |
| `list_calendars` | | `CalendarInfo[]` | Agrupados por cuenta, con color, visible, access role. |
| `set_calendar_visible` | `calendar_id, visible` | | Persistido en `calendars.visible`. |
| `get_view` | `from, to, tz` | `ViewPayload` | Ver 3.2. |
| `get_event` | `occurrence_id` | `EventDetail` | Todo lo que muestra el popup. |
| `create_event` | `EventDraft` | `EventDetail` | |
| `update_event` | `occurrence_id, EventDraft, scope` | `EventDetail` | `scope` en `this`, `following`, `all`. |
| `delete_event` | `occurrence_id, scope` | | |
| `rsvp` | `occurrence_id, status, send_updates` | `EventDetail` | |
| `move_event_account` | `event_id, target_calendar_id` | `EventDetail` | Local a Google, Google a local, o Google a Google entre cuentas. |
| `get_colors` | | `ColorPalette` | Paleta moderna de la UI, mapeada por id. |
| `get_settings` / `set_settings` | | `Settings` | Zona secundaria, semana, formato, tema. |
| `sync_now` | | | Fuerza incremental en todo. |
| `open_url` | `url` | | Abre en el navegador del sistema vía opener. |

Eventos emitidos desde Rust:

| Evento | Payload | Cuándo |
|---|---|---|
| `calendar:updated` | `{ from, to, calendar_ids }` | Tras cualquier cambio en `occurrences`. |
| `sync:status` | `{ account_id, state, message, at }` | Cambio de estado de sincronización. |
| `account:changed` | `AccountInfo[]` | Alta, baja o error de auth. |
| `window:show-event` | `occurrence_id` | Click en notificación. |

## 6. Presupuesto de RAM y cómo medirlo

Objetivo: menos de 150 MB con ventana abierta y en reposo, menos de 60 MB del proceso Rust cuando la ventana está oculta. El WebKitWebProcess no se libera al ocultar la ventana, así que la cifra total en bandeja será mayor. Si el usuario lo pide, la versión 2 destruye la ventana al cerrar y la recrea al abrir.

Medidas obligatorias:

- Una sola ventana. Los popups son elementos del DOM, nunca ventanas nuevas.
- Sin feature `devtools` en release.
- `[profile.release]` con `lto = true`, `codegen-units = 1`, `opt-level = "s"`, `panic = "abort"`, `strip = true`.
- `"build": { "removeUnusedCommands": true }` en `tauri.conf.json`.
- Estado de datos en Rust. El frontend guarda solo el `ViewPayload` actual y estado de UI.
- Sin timers en JS. Todo lo periódico vive en tokio.
- Listas virtualizadas en agenda y mes cuando superen 200 filas.

`scripts/measure-ram.sh` imprime PSS del proceso principal, de `WebKitWebProcess` y de `WebKitNetworkProcess` con `smem` o, si no está, con `/proc/<pid>/smaps_rollup`. Se ejecuta al cerrar cada fase y el número queda en `99-decisiones.md`.

## 7. Seguridad

- Tokens en `~/.local/share/unified-google-calendar/tokens.bin`. Formato: `nonce(12) || AES-256-GCM(ciphertext)`, un nonce nuevo en cada escritura. La clave se deriva con HKDF-SHA256 de `/etc/machine-id` más el uid más la sal fija `ugc-token-store-v1`. La clave no se guarda en ningún lado. Límite conocido: cualquier proceso corriendo como el mismo usuario puede derivarla. Protege contra copias del archivo fuera de la máquina y lecturas casuales. El usuario eligió este esquema sobre el llavero de GNOME. Alternativa documentada en `docs/research/tauri.md` sección 6 si cambia de idea.
- `client_id` y `client_secret` del proyecto de Google se leen de `~/.config/unified-google-calendar/oauth.json`, nunca se compilan en el binario ni se commitean.
- Webhook: acepta solo `POST /gcal/webhook` y `GET /google<token>.html`. Cualquier otra ruta responde 404. Límite de cuerpo 64 KB descartado sin leer. Rate limit de 60 requests por minuto por IP.
- `capabilities/default.json` habilita solo `core:default`, `opener:allow-open-url` con scope `https://*` y los comandos propios. Nada de `shell`.
- CSP en `tauri.conf.json`: la de `07-empaquetado.md` sección 2. Las fuentes OFL se empaquetan. Solo las familias propietarias de Google se cargan de `fonts.googleapis.com` si la medición lo exige, ver `04-fidelidad-visual.md` sección 6.

## 8. Concurrencia

- `db::worker`: un hilo con `rusqlite::Connection` y un `std::sync::mpsc` de closures `FnOnce(&mut Connection) -> R`. `db::call(|c| ...)` devuelve un `oneshot` awaitable. Así no hay `Mutex<Connection>` bloqueando tokio.
- `sync::engine`: una tarea por cuenta, con un `tokio::sync::Mutex` por calendario para que push, poll y catch-up nunca se solapen sobre el mismo sync token.
- Escrituras de UI: se ejecutan en el momento, esperan la respuesta de Google y luego escriben en `db`. Si un incremental llega en el medio, gana el orden de `db::worker`, que es secuencial.

## 9. Manejo de errores

`AppError` con variantes `Auth`, `Network`, `Google { status, reason }`, `Db`, `Recurrence`, `NotFound`, `Invalid(String)`. Cada variante tiene `user_message()` en inglés. Los comandos convierten con `map_err(|e| e.user_message())`. Los errores de sync no se muestran como diálogos: van al ícono de estado y al log.

Log con `tracing` a `~/.local/share/unified-google-calendar/logs/app.log`, rotación diaria, 7 archivos. Nunca se loguean tokens ni cuerpos de respuesta completos.

## 10. Lo que la versión 1 no hace

Sin cola offline, sin arrastrar y soltar, sin búsqueda, sin Tasks, sin `.ics`, sin auto arranque, sin auto actualización. Están listados en `01-requisitos.md`. No implementar nada de esto aunque parezca fácil.

## Registro de cambios

- 2026-09-14 — Evento `clock:minute` (Rust → UI, cada minuto) y campo `location` en `ViewOccurrence`; `set_calendar_visible` emite `calendar:updated`: ver `docs/99-decisiones.md` (F4-T3, F4-T4).
- 2026-09-14 — `EventDetail.conference_phone` y `Settings.oauth_configured`; segundo diálogo (`overlay`) en el estado de UI: ver `docs/99-decisiones.md` (Fase 7).
- 2026-09-14 — Cifras de RAM de la fase 8 y `measured.css` con valores literales: ver `docs/99-decisiones.md` (Fase 8).
- 2026-09-16 — Transición a Material 3 (`docs/99-decisiones.md`): sección 1, el frontend pasa a React 19 y suma `@material/web` 2.5.0 (runtime, con `lit` transitiva) y `@material/material-color-utilities` 0.3.0 (solo desarrollo); la frase "La UI es propia porque tiene que ser pixel a pixel igual a Google Calendar" deja de valer. Sección 4: `src/m3/` (registro y tipos de los custom elements), `src/components/MonthGrid.tsx`, `src/styles/{tokens.css,typescale.css}` generados desde `docs/design/m3/theme.json`; `docs/design/tokens.json` y `measurements/` se retiran (histórico en `docs/design/google/`). Sección 7: la CSP pierde `https://fonts.googleapis.com` y `https://fonts.gstatic.com`; ninguna fuente se carga de la red. Sección 6: la cifra de RAM se vuelve a medir al cerrar la transición (fila M6 en `99`). Detalle en `docs/11-material3.md`.
