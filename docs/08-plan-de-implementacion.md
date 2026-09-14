# 08 — Plan de implementación

Nueve fases, en orden. Cada fase tiene tareas numeradas `F<fase>-T<n>`. Una tarea se cierra cuando cumple **todos** sus criterios de aceptación y el comando de verificación pasa. Una fase se cierra con el procedimiento de la sección 11. No se empieza una fase sin cerrar la anterior.

Documentos que aplican siempre: `CLAUDE.md`, `01-requisitos.md`, `02-arquitectura.md`. Cada tarea lista los demás.

Convención de commits: `F4-T3: week grid hour rows` en inglés, un commit por tarea como mínimo.

## Fase 0 — Bootstrap del repositorio

Objetivo: proyecto Tauri 2 + React + TypeScript que compila, corre y pasa lint, con la estructura de `02-arquitectura.md` sección 4.

### F0-T1 Dependencias del sistema
- Docs: `07-empaquetado.md` sección 1.
- Pasos: pedir al usuario que corra los comandos de apt y rustup. El modelo no ejecuta `sudo`.
- Aceptación: `rustc --version` ≥ 1.85, `pkg-config --modversion webkit2gtk-4.1` responde, `pyftsubset --help` responde.

### F0-T2 Scaffold
- Pasos: `npm create tauri-app@latest . -- --template react-ts --manager npm` en el directorio del proyecto, o equivalente manual. Reorganizar a la estructura de `02` sección 4. Agregar en `Cargo.toml` las dependencias de `02` sección 1 con versiones fijadas. Configurar `tauri.conf.json` según `07` sección 2. Agregar `eslint`, `prettier`, `typescript` strict, `vitest`. Agregar `[workspace.lints]` con `clippy::all` como `warn` y CI local `cargo clippy -- -D warnings`.
- Aceptación: `npm run tauri dev` abre una ventana con texto "Unified Google Calendar". `cargo clippy -- -D warnings`, `cargo test`, `npm run lint`, `npm run typecheck` pasan. `scripts/check-phase.sh 0` pasa.

### F0-T3 Esqueleto de módulos
- Pasos: crear cada módulo de `02` sección 4 con `mod.rs` vacío y un `// Ver docs/02-arquitectura.md sección N`. `error.rs` con `AppError` completo según `02` sección 9. `config.rs` con las rutas de `07` sección 4. Logging con `tracing` y `tracing-appender` a `logs/app.log`.
- Aceptación: compila. Al arrancar, `logs/app.log` recibe una línea `app started`.

## Fase 1 — Base local: SQLite, recurrencias, comandos de lectura

Objetivo: la app almacena y expande eventos sin hablar con Google. Todo probado con tests de Rust.

### F1-T1 Esquema y worker
- Docs: `03-modelo-de-datos.md` secciones 1 y 10, `02` sección 8.
- Pasos: `db/schema/0001_init.sql` con el esquema literal de `03`. `db/migrations.rs` con `user_version`. `db/worker.rs` con hilo dedicado y `db::call`. La migración inserta la cuenta `local` y el calendario `local-personal` con color `#f4511e` y `default_reminders=[{"method":"popup","minutes":10}]`.
- Aceptación: test que abre una DB temporal, migra, y verifica `user_version=1` y la existencia de la cuenta local. Test de que migrar dos veces no falla.

### F1-T2 Expansión de recurrencias
- Docs: `03` sección 3.
- Pasos: `recurrence/expand.rs` con `expand_master(conn, account_id, calendar_id, event_id, window) -> Result<usize>` y `materialize_simple(...)`. Usar crate `rrule`. Manejar `EXDATE`, `RDATE`, excepciones con `original_start_ts`, excepciones canceladas, all-day.
- Aceptación: tests con al menos estos casos, comparando las fechas resultantes contra listas escritas a mano en el test: diario 5 veces; semanal lunes y miércoles con `UNTIL`; mensual el día 31 en meses cortos; anual; cada 2 semanas con una `EXDATE`; con `RDATE` extra; una excepción movida; una excepción cancelada; evento all-day de 2 días; evento semanal en `America/Argentina/Buenos_Aires` que cruza un cambio de hora en `America/Mexico_City` sin cambiar la hora local. Tope de 5000 instancias respetado.

### F1-T3 Escritura local y scopes
- Docs: `03` secciones 4 y 8.
- Pasos: `db/queries/events.rs` con `upsert_event`, `delete_event`, `get_event`. `recurrence/edit_scope.rs` con `apply_edit(scope, ...)` para `this`, `following`, `all` sobre la tabla local, incluyendo el recálculo de `UNTIL` y `COUNT`.
- Aceptación: tests para cada scope de edición y borrado sobre un master semanal de 10 instancias, verificando la tabla `occurrences` resultante.

### F1-T4 Comandos de lectura
- Docs: `02` sección 5, `03` secciones 5, 6 y 9.
- Pasos: `commands/types.rs` con todos los tipos de `02` sección 5. `commands/view.rs` con `get_view` y `get_event`, incluyendo deduplicación por `ical_uid` y conflictos. `src/types/ipc.ts` espejo. Test de Rust que serializa cada tipo a `src/types/fixtures/<Tipo>.json`.
- Aceptación: test de `get_view` con tres cuentas de prueba, dos copias del mismo `ical_uid`, un conflicto real y un evento `transparent` que no cuenta. `npm run typecheck` pasa con los fixtures importados en un test de vitest.

### F1-T5 Comandos de escritura local
- Pasos: `create_event`, `update_event`, `delete_event`, `move_event_account` solo para local a local. Validación de `EventDraft`: fin mayor que inicio, título opcional, recurrencia válida.
- Aceptación: tests de comandos con DB temporal.

## Fase 2 — OAuth y lectura desde Google

Objetivo: agregar una cuenta real y ver sus calendarios y eventos en SQLite.

### F2-T1 Almacén de tokens cifrado
- Docs: `02` sección 7.
- Pasos: `auth/token_store.rs` con `load()`, `save()`. HKDF-SHA256 sobre `/etc/machine-id` + uid + sal. AES-256-GCM, nonce nuevo por escritura, archivo con permisos 0600.
- Aceptación: test de ida y vuelta; test de que un archivo alterado en un byte falla la autenticación de GCM.

### F2-T2 OAuth PKCE con loopback
- Docs: `05-sincronizacion.md` sección 1.
- Pasos: `auth/pkce.rs`, `auth/oauth.rs` con `add_account()`. Lee `oauth.json`. Listener en `127.0.0.1:0`. Abre navegador con opener. Timeout 5 minutos. Refresco con mutex por cuenta. Manejo de `invalid_grant` y `admin_policy_enforced`.
- Aceptación: test del listener con un cliente HTTP falso que llama al redirect con `state` correcto e incorrecto. Prueba manual con el usuario: la cuenta de Greelow y el Gmail personal quedan en `accounts`. Registrar el resultado de la cuenta Workspace en `99-decisiones.md`.

### F2-T3 Cliente de Calendar API
- Docs: `05` sección 2, `docs/research/google-calendar-api.md`.
- Pasos: `google/client.rs` con `reqwest` rustls, inyección de bearer, backoff en 403/429/5xx. `google/types.rs` con structs `serde` de `CalendarListEntry`, `Event`, `Channel`, `Colors`. `google/calendar_list.rs`, `google/events.rs` con `list_page`. Tests con `wiremock` y respuestas JSON guardadas en `src-tauri/tests/fixtures/google/`.
- Aceptación: tests de paginación, de `nextSyncToken`, de `410`, de backoff en `429`.

### F2-T4 Full e incremental
- Docs: `05` sección 2, `03` sección 2.
- Pasos: `sync/full.rs`, `sync/incremental.rs`, `sync/engine.rs` con mutex por calendario, `SyncTick`, `sync_state` por cuenta, `sync_log`. Parámetros fijos de `events.list`. Mapeo completo Google → `events` incluyendo `raw`. Re-expansión al tocar masters.
- Aceptación: tests con wiremock: full de dos páginas, incremental con un cancelado y una excepción nueva, `410` que dispara wipe y full. Prueba manual: los eventos de la semana actual del usuario aparecen en `occurrences` y coinciden con lo que muestra calendar.google.com.

### F2-T5 Paleta y colores
- Docs: `04-fidelidad-visual.md` sección 7, `03` sección 2.
- Pasos: `google/colors.rs` con la tabla moderna por `colorId`. Resolución de color por evento: `color_id` mapeado, si no el `color_bg` del calendario. `get_colors`.
- Aceptación: test de mapeo de los 11 ids.

## Fase 3 — Escrituras contra Google

Objetivo: crear, editar, borrar, responder y mover eventos en cuentas reales.

### F3-T1 Insert, update, delete
- Docs: `05` sección 2.3, `03` sección 4.
- Pasos: `google/events.rs` con `insert`, `update`, `patch`, `delete`, `instances`, `move`, `import`. `commands/events.rs` enruta local o Google según `account_id`. `update` construye desde `raw`. `conferenceDataVersion=1` siempre. `sendUpdates` según regla.
- Aceptación: tests con wiremock que verifican query params y cuerpo enviado. Prueba manual en un calendario secundario de prueba del usuario: crear, editar título, borrar, verificar en la web de Google.

### F3-T2 Scopes de recurrencia en Google
- Docs: `03` sección 4.
- Pasos: `this` con id de instancia; `following` con `UNTIL` en el master y `insert` del nuevo master; `all` sobre el master. Fallback a `events.instances` con `originalStart` si el id construido devuelve 404.
- Aceptación: prueba manual con un evento semanal de prueba en los tres scopes, verificando en la web. Tests con wiremock del cuerpo enviado.

### F3-T3 Invitados, RSVP y Meet
- Pasos: `rsvp` con `patch` de `attendees` completo. `create_event` con `add_meet=true` genera `createRequest`. Polling de `pending` hasta 3 veces.
- Aceptación: prueba manual: crear evento con Meet e invitado de prueba, aceptar desde la app, verificar en la web.

### F3-T4 Mover entre cuentas y feriados
- Docs: `03` sección 7, `09-setup-usuario.md` sección E.
- Pasos: los cuatro caminos de `move_event_account`. Suscripción a `en.ar#holiday@group.v.calendar.google.com` en la primera cuenta Gmail, con `settings.holidays_account`.
- Aceptación: prueba manual local → Google, Google → local, Google → Google entre cuentas. Los feriados aparecen en `calendars` con `access_role=reader`.

## Fase 4 — UI base pixel a pixel

Objetivo: barra superior, barra lateral, vista semana con chips, tema claro y oscuro, idénticos a Google.

### F4-T1 Medición de componentes 1 a 10
- Docs: `04` completo, `09` sección F.
- Pasos: el usuario prepara el perfil de Chrome y los eventos de prueba. El modelo pide al usuario que ejecute `dumpRegion` y las capturas por componente, o lo hace con el usuario presente. Guardar todo en `docs/design/measurements/`. Completar `docs/design/tokens.json`. Correr `node scripts/gen-tokens.mjs`.
- Aceptación: `node scripts/check-tokens.mjs` pasa. Existen JSON y PNG claro y oscuro para los componentes 1 a 10. Se registró en `99-decisiones.md` qué fuente resultó efectiva para cada rol de texto.

### F4-T2 Fuentes e íconos
- Docs: `04` sección 6.
- Pasos: descargar Google Sans Flex y Roboto de `google/fonts` a `src-tauri/resources/fonts/` junto con `OFL.txt`. Subset de Material Symbols con los íconos de `docs/design/icons.txt`. `src/styles/fonts.css` con `@font-face`. `font-synthesis: none`.
- Aceptación: en el webview, `document.fonts.check('500 14px "Google Sans Flex"')` es `true`. Un ícono `check` se renderiza como ligadura.

### F4-T3 Shell: barra superior y barra lateral
- Pasos: `src/app/` con TopBar, Sidebar, CreateButton, MiniCalendar, CalendarList agrupado por cuenta con avatar y etiqueta de tipo, ViewSelector. Estado de UI en zustand: fecha, vista, tema. `src/ipc/` con wrappers tipados. Zustand se agrega a `package.json`.
- Aceptación: `diff-layout` 0 diferencias y `odiff` < 0.5% para componentes 2, 3, 4, 5 en ambos temas.

### F4-T4 Vista semana
- Pasos: `src/views/week/` con cabecera, fila all-day, gutter con zona primaria y secundaria, grilla, línea de ahora, chips con layout de solapamiento replicando el de Google medido en el componente 10. `get_view` conectado. Escucha de `calendar:updated`.
- Aceptación: `diff-layout` 0 y `odiff` < 0.5% para componentes 6, 7, 8, 9, 10 en ambos temas, con los eventos de prueba.

### F4-T5 Tema del sistema
- Pasos: `prefers-color-scheme` mapeado a tokens `light` y `dark`. Sin selector manual en la versión 1.
- Aceptación: al cambiar el tema de GNOME, la app cambia sin reiniciar.

## Fase 5 — Push y sincronización continua

### F5-T1 Servidor webhook
- Docs: `05` sección 3.
- Pasos: `webhook/server.rs` con axum en `127.0.0.1:8080`, rutas de `05` 3.1, validación en tiempo constante, límite de cuerpo, rate limit, `SyncTick` al canal.
- Aceptación: tests de las cuatro rutas y de cada rama de validación con `axum::test` o `tower::ServiceExt`.

### F5-T2 Canales
- Pasos: `google/channels.rs` con `watch_events`, `watch_calendar_list`, `stop`. `sync/push.rs` con creación, renovación cada hora y stop al quitar cuenta. `push_enabled` y `public_base_url` en settings con `Test`.
- Aceptación: tests con wiremock. Prueba manual con el usuario tras completar `09` sección B: crear un evento en calendar.google.com y verlo en la app en menos de 5 segundos. Registrar en `99-decisiones.md` si Google exigió verificación de dominio.

### F5-T3 Red de seguridad
- Docs: `05` sección 4.
- Pasos: `sync/poll.rs` con intervalo según `push_enabled`. `sync/sleep.rs` con `zbus` sobre `login1`. `sync_now`. Indicador de estado en la barra superior.
- Aceptación: suspender y despertar la máquina produce una línea `wake` en `sync_log`. Con `push_enabled=false` el incremental corre cada 60 s.

## Fase 6 — Integración con GNOME

### F6-T1 Notificaciones
- Docs: `06-integracion-gnome.md` secciones 1 y 3, `03` sección 8.
- Pasos: `reminders/scheduler.rs`, `reminders/notify.rs` con `notify-rust`, acciones `open`, `join`, `snooze`. `fired_reminders` persistido.
- Aceptación: un evento de prueba con recordatorio a 1 minuto dispara una notificación de GNOME con botón Join que abre el navegador. Reiniciar la app no la repite.

### F6-T2 Bandeja y cerrar a segundo plano
- Docs: `06` secciones 2 y 3.
- Pasos: `tray.rs` con menú y `set_title`. `on_window_event` y `ExitRequested`. `single-instance`.
- Aceptación: cerrar la ventana deja el proceso vivo y el ícono en la barra. "Open" la muestra. Lanzar la app de nuevo no crea otro proceso. El título de bandeja muestra el próximo evento.

### F6-T3 Espejo en Evolution Data Server
- Docs: `06` sección 4.
- Pasos: `eds/dbus.rs` con descubrimiento de buses, `GetManagedObjects`, `CreateSources`, `Write`, `Remove`, `OpenCalendar`, `Open`, `GetObjectList`, `CreateObjects`, `ModifyObjects`, `RemoveObjects`, `Close`. `eds/mirror.rs` con generación de VEVENT en UTC sin VALARM, diff por `LAST-MODIFIED`, debounce de 5 s, ventana -30/+180 días.
- Aceptación: `scripts/check-eds.sh` pasa. El panel de GNOME muestra los eventos de la app. Quitar una cuenta borra sus fuentes.

### F6-T4 Diálogo de Online Accounts
- Docs: `06` sección 4.6.
- Pasos: detectar cuentas Google en GOA por D-Bus, diálogo en el primer arranque, `CalendarDisabled=true` con consentimiento.
- Aceptación: tras aceptar, `~/.config/goa-1.0/accounts.conf` tiene `CalendarEnabled=false` en las cuentas Google y el panel no duplica eventos.

## Fase 7 — UI completa

### F7-T1 Medición de componentes 11 a 20
- Igual que F4-T1 para los componentes restantes.

### F7-T2 Vistas día, mes y agenda
- Aceptación: `diff-layout` 0 y `odiff` < 0.5% en componentes 11, 12, 13.

### F7-T3 Popup de detalle
- Docs: `03` sección 9.
- Pasos: `src/event/EventPopup` con todas las filas de `get_event`, botones editar, borrar, RSVP, Join, `also_in`, aviso de conflicto. Posicionamiento por JS según `04` sección 8.
- Aceptación: componente 14 en sus cuatro estados.

### F7-T4 Quick create y formulario completo
- Aceptación: componentes 15 y 16. Crear desde ambos guarda en la cuenta elegida.

### F7-T5 Recurrencia y scope
- Aceptación: componentes 18 y 19. Editar una instancia pregunta scope y aplica según `03` sección 4.

### F7-T6 Settings y bienvenida
- Docs: `07` sección 5.
- Pasos: pantalla de bienvenida sin `oauth.json`, alta de primera cuenta, Settings con zona secundaria, cuenta por defecto, push URL y Test, cuenta de feriados, lista de cuentas con "Sign in again" y "Remove".
- Aceptación: primer arranque en una máquina limpia llega a la vista semana siguiendo `07` sección 5.

### F7-T7 Selector de vista, scrollbars y hover
- Aceptación: componentes 17 y 20. Estados hover de chips y filas.

## Fase 8 — Empaquetado y cierre

### F8-T1 `.deb`
- Docs: `07`.
- Aceptación: `TAURI_LINUX_AYATANA_APPINDICATOR=1 npm run tauri build -- --bundles deb` produce el paquete. `sudo apt install ./...deb` instala. La app aparece en el launcher con su ícono propio y arranca.

### F8-T2 RAM
- Docs: `02` sección 6.
- Pasos: `scripts/measure-ram.sh` con la app abierta en vista semana con 3 cuentas, y con la ventana cerrada. Anotar en `99-decisiones.md`. Si supera 150 MB total en reposo, aplicar las medidas de `02` sección 6 en orden y volver a medir.
- Aceptación: tabla de `99` completa.

### F8-T3 Cierre
- Pasos: `scripts/uninstall-data.sh`. README.md corto en inglés con build e instalación apuntando a `docs/`. Revisar que `99-decisiones.md` tiene todo desvío.
- Aceptación: `scripts/check-phase.sh 8` pasa.

## 9. Fuera de alcance

Todo lo listado en `01-requisitos.md` como versión 2. Si una tarea parece requerirlo, se anota en `99-decisiones.md` y se sigue sin implementarlo.

## 10. Cuándo preguntar al usuario

- Cualquier comando con `sudo`.
- Cualquier acción sobre sus cuentas reales de Google que no sea en el calendario de prueba acordado.
- Cambios a `~/.config/goa-1.0` o a fuentes EDS que no sean `ugc-*`.
- Cuando una medición de Google no coincide con lo que dice un documento.
- Cuando la verificación de dominio o el admin de Workspace bloquean algo.

## 11. Procedimiento de cierre de fase

1. `scripts/check-phase.sh <N>` pasa.
2. `cargo clippy -- -D warnings`, `cargo test`, `npm run lint`, `npm run typecheck`, `npm test` pasan.
3. `scripts/measure-ram.sh` ejecutado si la fase cambia la UI o el sync. Resultado en `99`.
4. Todo desvío respecto de `docs/` anotado en `99`.
5. Commit `F<N>: phase closed`.
6. Mensaje al usuario con: qué se hizo, qué quedó pendiente, qué tiene que hacer él para la fase siguiente según `09`.

## 12. Contratos de módulos en Rust

Firmas que la implementación respeta. Los tipos de datos están en `commands/types.rs` y en `google/types.rs`.

```rust
// db
pub async fn call<R: Send + 'static>(f: impl FnOnce(&mut rusqlite::Connection) -> Result<R, AppError> + Send + 'static) -> Result<R, AppError>;
pub fn migrate(conn: &mut Connection) -> Result<(), AppError>;

// recurrence
pub struct Window { pub from_ts: i64, pub to_ts: i64 }
pub fn expand_master(conn: &mut Connection, account_id: &str, calendar_id: &str, event_id: &str, window: Window) -> Result<usize, AppError>;
pub fn materialize_simple(conn: &mut Connection, account_id: &str, calendar_id: &str, event_id: &str, window: Window) -> Result<(), AppError>;
pub enum EditScope { This, Following, All }

// auth
pub async fn add_account(app: &AppHandle) -> Result<AccountInfo, AppError>;
pub async fn access_token(account_id: &str) -> Result<String, AppError>;   // refresca si hace falta

// google
pub struct Client { /* reqwest::Client, base url configurable para tests */ }
pub async fn calendar_list_page(&self, token: &str, page_token: Option<&str>, sync_token: Option<&str>) -> Result<CalendarListPage, AppError>;
pub async fn events_page(&self, token: &str, calendar_id: &str, page_token: Option<&str>, sync_token: Option<&str>) -> Result<EventsPage, AppError>;
pub async fn event_insert(&self, token: &str, calendar_id: &str, body: &Event, send_updates: SendUpdates) -> Result<Event, AppError>;
pub async fn event_update(&self, token: &str, calendar_id: &str, event_id: &str, body: &Event, send_updates: SendUpdates) -> Result<Event, AppError>;
pub async fn event_patch(&self, token: &str, calendar_id: &str, event_id: &str, body: &serde_json::Value, send_updates: SendUpdates) -> Result<Event, AppError>;
pub async fn event_delete(&self, token: &str, calendar_id: &str, event_id: &str, send_updates: SendUpdates) -> Result<(), AppError>;
pub async fn event_move(&self, token: &str, calendar_id: &str, event_id: &str, destination: &str) -> Result<Event, AppError>;
pub async fn event_import(&self, token: &str, calendar_id: &str, body: &Event) -> Result<Event, AppError>;
pub async fn watch_events(&self, token: &str, calendar_id: &str, req: &WatchRequest) -> Result<Channel, AppError>;
pub async fn watch_calendar_list(&self, token: &str, req: &WatchRequest) -> Result<Channel, AppError>;
pub async fn channel_stop(&self, token: &str, id: &str, resource_id: &str) -> Result<(), AppError>;

// sync
pub struct SyncTick { pub account_id: String, pub calendar_id: Option<String> }
pub fn start(app: AppHandle) -> tokio::sync::mpsc::Sender<SyncTick>;
pub async fn full_sync_calendar(app: &AppHandle, account_id: &str, calendar_id: &str) -> Result<(), AppError>;
pub async fn incremental_calendar(app: &AppHandle, account_id: &str, calendar_id: &str) -> Result<(), AppError>;
pub async fn sync_all(app: &AppHandle, reason: &str) -> Result<(), AppError>;

// eds
pub async fn mirror_calendars(app: &AppHandle, calendar_keys: &[(String, String)]) -> Result<(), AppError>;
pub async fn remove_account_sources(app: &AppHandle, account_id: &str) -> Result<(), AppError>;

// reminders
pub fn start(app: AppHandle);

// tray
pub fn build(app: &AppHandle) -> Result<(), AppError>;
pub fn set_next_event(app: &AppHandle, label: Option<String>);
```
