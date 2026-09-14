# 99 — Registro de decisiones y desvíos

Cada entrada: fecha, quién decidió (usuario, modelo de diseño, modelo de implementación), qué se decidió, por qué, y qué documento afecta. Las decisiones de diseño previas a la implementación están en los documentos 01 a 07. Acá van las que surgen después.

Formato:

```
## AAAA-MM-DD — Título corto
- Quién: usuario | implementación
- Fase/tarea: F4-T3
- Decisión: ...
- Motivo: ...
- Afecta: docs/03-modelo-de-datos.md sección 4
```

## 2026-09-14 — Cierre de la fase de diseño
- Quién: usuario y modelo de diseño
- Decisión: la implementación la hace un modelo distinto siguiendo `docs/` y el harness de `.claude/`. No se escribió código de producto en la fase de diseño.
- Motivo: costo.
- Afecta: todo.

## 2026-09-14 — Tokens en archivo cifrado propio, no en GNOME Keyring
- Quién: usuario
- Decisión: `tokens.bin` con AES-256-GCM y clave derivada de machine-id y uid.
- Motivo: menos dependencias del sistema. El usuario aceptó el límite de que un proceso del mismo usuario puede derivar la clave.
- Afecta: `02-arquitectura.md` sección 7.

## 2026-09-14 — Fuentes propietarias de Google en runtime
- Quién: modelo de diseño, pendiente de confirmación del usuario en la fase 4
- Decisión: Google Sans Flex y Roboto empaquetadas. Google Sans Text solo si la medición la muestra como fuente efectiva, cargada desde fonts.googleapis.com.
- Motivo: no redistribuir fuentes propietarias. App personal no distribuida.
- Afecta: `04-fidelidad-visual.md` sección 6.

## 2026-09-14 — Fase 0: entorno y nombre del archivo de log
- Quién: implementación
- Fase/tarea: F0-T1, F0-T3
- Decisión: (1) Rust se instaló con `rustup` en modo `--no-modify-path` y perfil `minimal` más `clippy` y `rustfmt`; el `PATH` se exporta por sesión con `export PATH="$HOME/.cargo/bin:$PATH"`. Versión instalada: rustc 1.98.1. (2) El paquete `python3-fonttools` de Ubuntu 25.04 no instala el binario `pyftsubset`; se usa el equivalente `python3 -m fontTools.subset`. (3) El log con rotación diaria de `tracing-appender` se llama `logs/app.<AAAA-MM-DD>.log`, no `logs/app.log`, porque el appender rotativo siempre agrega la fecha al nombre. Se conservan 7 archivos.
- Motivo: (1) evitar tocar `~/.bashrc` del usuario. (2) el binario está en el paquete `fonttools`, no en `python3-fonttools`. (3) limitación de la librería; el criterio de "7 archivos, rotación diaria" se cumple.
- Afecta: `docs/07-empaquetado.md` sección 1, `docs/02-arquitectura.md` sección 9, `docs/08` F0-T3.

## 2026-09-14 — Versiones concretas del stack
- Quién: implementación
- Fase/tarea: F0-T2
- Decisión: tauri 2.11, tauri-build 2.6, tauri-plugin-single-instance 2.4, tauri-plugin-opener 2.5, rusqlite 0.40, reqwest 0.12 (rustls), axum 0.8, rrule 0.13, chrono 0.4, chrono-tz 0.10, notify-rust 4.18 (backend zbus), aes-gcm 0.10 + hkdf 0.12 + sha2 0.10 (línea `digest` 0.10, compatibles entre sí), zbus 5, tokio 1.53, uuid 1, rand 0.9, base64 0.22, url 2, `subtle` 2 para la comparación en tiempo constante del token del webhook. Frontend: React 18.3.1, TypeScript 5.9.3, Vite 6.4.3, Vitest 3.2.7, ESLint 9, `@tauri-apps/api` 2.11.1, date-fns 4.4.0, `@date-fns/tz` 1.5.0. Dev: wiremock 0.6, tempfile 3, http-body-util 0.1.
- Motivo: `02` sección 1 da versiones mínimas; estas son las últimas estables de cada línea al 2026-09-14. `subtle` y `http-body-util` son utilidades de test/seguridad sin superficie propia; se listan acá para cumplir la regla de dependencias.
- Afecta: `docs/02-arquitectura.md` sección 1.

## 2026-09-14 — Identificación de eventos en comandos de escritura y movimiento
- Quién: implementación
- Fase/tarea: F1-T5
- Decisión: `update_event`, `delete_event` y `move_event_account` reciben `occurrence_id` (que codifica cuenta, calendario y evento) en lugar de un `event_id` suelto; `move_event_account` recibe además `target_account_id` y `target_calendar_id`. `get_view` acepta el parámetro `tz` del contrato pero no lo usa: la posición de los eventos de día completo la resuelve la UI con la zona principal.
- Motivo: `events.id` solo es único dentro de `(account_id, calendar_id)`; el `occurrence_id` ya lleva las tres claves y evita ambigüedad. Los calendarios de destino se identifican por par cuenta+calendario por la misma razón.
- Afecta: `docs/02-arquitectura.md` sección 5 (tabla de comandos).

## 2026-09-14 — Fase 2 cerrada con verificaciones manuales pendientes
- Quién: implementación
- Fase/tarea: F2-T2, F2-T4
- Decisión: la fase se cierra con todos los criterios automáticos cumplidos (58 tests unitarios, 8 de cliente Google y 5 de sync con wiremock). Las pruebas manuales que requieren cuentas reales (alta de la cuenta Greelow y del Gmail personal, resultado del admin de Workspace, comparación de la semana actual contra calendar.google.com) quedan agrupadas en la lista de verificación manual que se entrega al usuario al final de la implementación, porque el usuario pidió avanzar sin interrupciones. La medición de RAM se hace con el build de release en F8-T2.
- Motivo: instrucción del usuario de completar todo el proyecto de corrido.
- Afecta: `docs/08-plan-de-implementacion.md` sección 11 (orden del cierre de fase).

## 2026-09-14 — Inyección de dependencias en el motor de sync
- Quién: implementación
- Fase/tarea: F2-T4
- Decisión: `sync::SyncCtx` agrupa `DbHandle`, `google::Client`, una fuente de tokens (`TokenSource`) y un emisor de eventos (`SyncEvents`). Las funciones `full_sync_calendar` e `incremental_calendar` reciben `&SyncCtx`; las firmas con `&AppHandle` de `docs/08` sección 12 existen como wrappers (`full_sync_calendar_app`, `incremental_calendar_app`, `sync_all`). En producción el contexto usa OAuth y `app.emit`; en tests usa un token fijo y un registrador en memoria.
- Motivo: probar el motor con wiremock sin construir una app Tauri.
- Afecta: `docs/08-plan-de-implementacion.md` sección 12.

## 2026-09-14 — Fase 3: detalles de implementación de las escrituras contra Google
- Quién: implementación
- Fase/tarea: F3-T1 a F3-T4
- Decisión: (1) Antes de usar el id de instancia construido (`master_YYYYMMDDTHHMMSSZ`), la app hace `events.get` sobre él; si responde 404 usa `events.instances?originalStart=` para obtener el id real. Con esto el fallback de `docs/03` sección 4 se aplica sin intento de escritura fallido. (2) Al mover entre cuentas distintas se importa solo el master; las excepciones del master no viajan (el `import` de Google no las acepta) y quedan como diferencia conocida de la versión 1. (3) Al editar "all" la app desplaza la serie por el mismo delta que el usuario aplicó a la instancia (misma regla que en local) y borra las excepciones locales si cambió la hora, igual que Google. (4) El cuerpo del RSVP toma el array `attendees` de `raw` del master cuando la ocurrencia no tiene excepción propia. (5) `get_settings`/`set_settings` se implementaron ya en esta fase porque `holidays_account` los necesita; las pruebas manuales de la fase quedan en la lista final de verificación.
- Motivo: casos no cubiertos literalmente por los documentos.
- Afecta: `docs/03-modelo-de-datos.md` secciones 4 y 7.

## 2026-09-14 — Orden de fases: 5 y 6 antes de cerrar la 4
- Quién: implementación
- Fase/tarea: F4-T1
- Decisión: las fases 5 (push y sincronización continua) y 6 (integración con GNOME) se implementan antes de F4-T1, F4-T3, F4-T4 y F4-T5. F4-T2 (fuentes e íconos) se hace en cuanto termina la 6. La fase 4 se cierra cuando el usuario entregue las mediciones de los componentes 1 a 10.
- Motivo: F4-T1 requiere que el usuario mida calendar.google.com con su sesión de Chrome; el modelo no puede iniciar sesión en su cuenta. Las fases 5 y 6 no dependen de ningún token de diseño. El usuario pidió avanzar sin interrupciones.
- Afecta: `docs/08-plan-de-implementacion.md` (regla "No se empieza una fase sin cerrar la anterior").

## 2026-09-14 — Fase 5: detalles del webhook y del listener de suspensión
- Quién: implementación
- Fase/tarea: F5-T1 a F5-T3
- Decisión: (1) El límite de cuerpo de 64 KB se aplica con `DefaultBodyLimit` de axum y el handler nunca lee el cuerpo. (2) El rate limit es una tabla en memoria por IP (`X-Forwarded-For` de Tailscale, o el peer) con ventana fija de 60 s. (3) Un `watch` que responde 400 o cuya `reason` menciona webhook/push/unauthorized/domain apaga `push_enabled` y guarda el mensaje en `settings.push_error`. (4) El listener de `PrepareForSleep` usa el bus de sistema con `zbus`; si no hay bus, solo se registra en el log y queda el polling. (5) El indicador de estado de la barra superior se implementa con la UI de la fase 4 (F4-T3). Las pruebas manuales (crear evento en la web y verlo en menos de 5 s, suspender y despertar) quedan en la lista final.
- Motivo: los documentos no fijan el mecanismo concreto de estos puntos.
- Afecta: `docs/05-sincronizacion.md` secciones 3.1, 3.3 y 4.

## 2026-09-14 — Fase 6: detalles de notificaciones, bandeja y espejo EDS
- Quién: implementación
- Fase/tarea: F6-T1 a F6-T4
- Decisión: (1) `sha1` 0.10 se agregó como dependencia para el UID `ugc-<sha1>` de las fuentes EDS (misma familia RustCrypto que `sha2`). (2) El UID de cada VEVENT es `ugc-<occurrence_id con | reemplazado por ->` porque se espejan ocurrencias y el `event_id` de un master se repite; `docs/06` 4.4 nombraba `ugc-<account_id>-<event_id>`. (3) El diff del espejo compara UID, DTSTART, DTEND, SUMMARY, LOCATION y LAST-MODIFIED del texto generado, ignorando DTSTAMP y SEQUENCE que EDS reescribe. (4) Tras `CreateSources` la app espera hasta 5 s a que el registro anuncie la fuente y reintenta `OpenCalendar` hasta 6 s, porque la fábrica de calendarios la ve de forma asíncrona (comprobado en vivo). (5) Recordatorios: un recordatorio perdido por hasta 10 minutos (suspensión) igual se muestra; los de día completo cuentan desde la medianoche local de la zona principal. (6) La extensión `ubuntu-appindicators@ubuntu.com` está instalada pero NO habilitada en la máquina al 2026-09-14 (`gsettings get org.gnome.shell enabled-extensions` no la incluye); sin ella no aparece el ícono de bandeja. El usuario debe habilitarla: `gnome-extensions enable ubuntu-appindicators@ubuntu.com`. (7) Verificado en vivo: webhook en 127.0.0.1:8080 (`/healthz` 200, POST inválido 404), listener de `login1`, fuente `ugc-local` con un VEVENT sin VALARM (`scripts/check-eds.sh` OK), y una notificación de recordatorio a 1 minuto registrada en `fired_reminders`. El diálogo de Online Accounts se muestra desde la UI (F7-T6) con los comandos `goa_status` y `goa_disable_calendars`.
- Motivo: comportamiento observado de EDS y GNOME en la máquina del usuario.
- Afecta: `docs/06-integracion-gnome.md` secciones 1, 2 y 4.4; `docs/01-requisitos.md` sección 10 (estado de la extensión).

## 2026-09-14 — Build de release y nombre del .deb
- Quién: implementación
- Fase/tarea: F8-T1 (prueba anticipada)
- Decisión: `TAURI_LINUX_AYATANA_APPINDICATOR=1 npm run tauri build -- --bundles deb` produce `src-tauri/target/release/bundle/deb/Unified Google Calendar_0.1.0_amd64.deb` (Tauri usa `productName` con espacios en el nombre del archivo, no `unified-google-calendar_0.1.0_amd64.deb` como dice `docs/07` sección 3). El paquete declara `Depends: libayatana-appindicator3-1, evolution-data-server, libwebkit2gtk-4.1-0, libgtk-3-0`, instala el binario en `/usr/bin/unified-google-calendar`, las fuentes en `/usr/lib/Unified Google Calendar/resources/fonts/`, los íconos en hicolor y el `.desktop` de la plantilla propia. Se hizo esta prueba antes de la fase 4 para detectar problemas de empaquetado temprano; la fase 8 se cierra al final con la UI terminada.
- Motivo: comportamiento del bundler de Tauri 2.11.
- Afecta: `docs/07-empaquetado.md` secciones 3 y 4 (nombre del archivo).

## 2026-09-14 — `removeUnusedCommands` exige declarar los comandos propios
- Quién: implementación
- Fase/tarea: F2-T2 (prueba manual)
- Decisión: con `removeUnusedCommands: true`, Tauri recorta los comandos según el ACL, no según el frontend. Sin un manifiesto de app, `generate_handler!` elimina todos los comandos propios y la UI recibe "Command X not found". Por eso `src-tauri/build.rs` declara la lista de comandos con `AppManifest::commands`, Tauri genera `src-tauri/permissions/autogenerated/` (`allow-<comando>`), y `capabilities/default.json` lista cada `allow-*`. La lista de `build.rs`, la de `commands::handler()` y la de la capability deben mantenerse iguales; `permissions/autogenerated/` se commitea porque `docs/02` sección 7 pide que la capability enumere los comandos propios.
- Motivo: comportamiento de tauri-macros 2.6 / tauri-utils 2.9 (`filter_unused_commands`).
- Afecta: `docs/02-arquitectura.md` sección 7, `docs/07-empaquetado.md` sección 2.

## 2026-09-14 — Ícono de la app
- Quién: usuario
- Fase/tarea: F0-T2 (revisión)
- Decisión: el ícono sigue el estilo del de `claude_code_display_plugin`: cuadrado naranja `#D97757` con esquinas redondeadas y un glifo blanco con borde negro, en este caso `calendar-multiple` de Material Design Icons (Apache-2.0). Fuente en `src-tauri/icons/app-icon.svg`; PNG generados con `tauri icon`.
- Motivo: pedido del usuario. Sigue sin usar ningún logo de Google (`docs/04` sección 9).
- Afecta: `docs/04-fidelidad-visual.md` sección 9.

## 2026-09-14 — Calendarios iCal por URL (dirección secreta), adelantados de la versión 2
- Quién: usuario
- Fase/tarea: F3 (extensión)
- Decisión: la app acepta calendarios `.ics` por URL, solo lectura. Motivo concreto: el admin de RappiCard bloquea toda app OAuth de terceros (`Error 400: access_not_configured`), incluida GNOME Online Accounts; la "dirección secreta en formato iCal" de calendar.google.com es la única vía. Modelo: una cuenta con `kind = 'ical'` por URL (id uuid, `display_name` elegido por el usuario) y un calendario `access_role = 'reader'`. La URL es un secreto y se guarda en `tokens.bin` (campo `ical_urls`), nunca en SQLite ni en logs. Sync: descarga y parseo en cada ciclo del poll de respaldo (60 s sin push, 10 min con push), con `If-None-Match`/hash del cuerpo para no reprocesar; parser propio de iCalendar (`sync/ics.rs`) que cubre VEVENT, RRULE/EXDATE/RDATE, RECURRENCE-ID, TZID IANA, día completo, STATUS, ATTENDEE y ORGANIZER. Sin escritura, sin RSVP, sin mover eventos desde ese calendario, sin canales push. El resto (vista, popup, conflictos, recordatorios, espejo EDS, bandeja) funciona igual que para Google.
- Motivo: sin esto la cuenta de RappiCard no puede verse en ninguna app; el usuario prefiere tenerla aquí aunque sea de solo lectura.
- Afecta: `docs/01-requisitos.md` R3.4 y sección 4 (versión 2); `docs/03-modelo-de-datos.md` sección 1 (`accounts.kind` admite `ical`); `docs/02-arquitectura.md` sección 5 (comando `add_ical_calendar`).

## Mediciones de RAM por fase

| Fase | Fecha | Proceso Rust PSS | WebKitWebProcess PSS | Total | Nota |
|---|---|---|---|---|---|
| | | | | | |
