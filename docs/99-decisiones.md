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

## Mediciones de RAM por fase

| Fase | Fecha | Proceso Rust PSS | WebKitWebProcess PSS | Total | Nota |
|---|---|---|---|---|---|
| | | | | | |
