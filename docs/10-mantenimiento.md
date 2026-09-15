# 10 — Estado al cierre de la implementación y guía de mantenimiento

Escrito el 2026-09-15, al terminar el plan de `08`. Desde acá el proyecto está en modo mantenimiento: el usuario usa la app instalada, reporta lo que ve y quien implementa lo corrige. Este documento dice qué hay, qué falta, qué se sabe que no está perfecto y cómo trabajar sin molestar al usuario.

## 1. Estado

- Fases F0 a F8 cerradas con sus gates (`bash scripts/check-phase.sh 0..8`). Último commit del plan: `a8094e3` ("F8: phase closed"), en `origin/main`.
- Paquete: `src-tauri/target/release/bundle/deb/Unified Google Calendar_0.1.0_amd64.deb` (11.5 MB). El usuario lo instaló el 2026-09-15.
- Cuentas en la app del usuario: dos cuentas Google corporativas de Greelow y un calendario iCal de solo lectura (RappiCard, publicado por un Apps Script a un gist secreto; la URL vive cifrada en `tokens.bin`). El Gmail personal queda para más adelante, por decisión del usuario.
- Verificación al cierre: `cargo clippy -D warnings`, 117 tests de Rust, `npm run lint`, `npm run typecheck`, 19 tests de vitest, `check-tokens.mjs`, `report.mjs --gate 4` y `--gate 7` sin diferencias de layout fuera de las tolerancias justificadas en `docs/design/measurements/<componente>.md`.
- Medición: 20 componentes de calendar.google.com en claro y oscuro, más la paleta de 24 colores de calendario y 11 de evento (`docs/design/measurements/`, `docs/design/tokens.json`).
- Memoria en release sobre Xvfb (render por software): 246.6 MB abierta, 91.7 MB el proceso Rust con la ventana oculta. Ver `99`, entrada de F8-T2.

## 2. Pendiente del lado del usuario

1. Abrir la app en su escritorio y recorrer semana, día, mes, agenda, popup de evento, creación rápida, formulario completo, recurrencia y alcance de edición. Reportar cada diferencia con Google o cada error.
2. Responder el diálogo de Online Accounts la primera vez (`09` sección D).
3. Con la app en vista semana, correr `bash scripts/measure-ram.sh` y pegar la tabla en `99`, sección "Mediciones de RAM por fase", con la nota "sesión real con GPU".
4. Opcional: Tailscale Funnel y la URL pública en Settings (`09` sección B). Sin eso la app hace polling cada 60 s.
5. Opcional: borrar el calendario de prueba "UGC Fixtures" con `cargo run --example seed_fixtures -- shlomo.serber@greelow.com --delete`. Tiene 25 eventos de referencia más 11 eventos "Color N". Si se borra, no se puede volver a medir chips contra Google sin recrearlo (`--seed`).
6. Ajustes de la cuenta Greelow que la medición cambió y siguen así: formato 24 h, semana desde lunes, zona secundaria America/Mexico_City, apariencia "Device default", colores "Modern", densidad "Responsive to your screen", panel lateral oculto. Las casillas de calendarios del sidebar ya están todas marcadas otra vez. El usuario los puede cambiar cuando quiera, salvo si va a remedir.

## 3. Limitaciones conocidas

| Tema | Detalle | Dónde está registrado |
|---|---|---|
| odiff | El criterio de 0.5 % de píxeles distintos no se cumple: WebKitGTK y Chrome rasterizan el texto distinto. El layout (posición y tamaño de cada nodo) sí coincide. | `99`, "Criterio de aceptación visual" |
| RAM | 246.6 MB abierta contra 150 MB de objetivo; 91.7 MB Rust oculto contra 60 MB. Medido en Xvfb con llvmpipe. Falta la cifra real con GPU. | `99`, F8-T2 |
| Hover | Los estados hover de chips y filas no están medidos ni implementados. | `99`, Fase 7 |
| Deduplicación | Google oculta copias de un mismo evento invitado a varias cuentas de forma distinta a la app en el calendario de prueba. | `docs/design/measurements/week.md` |
| Settings | El diálogo de Settings reutiliza tokens de los diálogos medidos; la página de ajustes de Google no se midió. | `src/app/SettingsDialog.tsx` |
| Scrollbars | Se dejan las barras overlay de GTK sin estilo; estilizar `::-webkit-scrollbar` fuerza barras clásicas en WebKitGTK. | `docs/design/measurements/scrollbars.md` |

## 4. Cómo se trabaja desde ahora

- Cada reporte del usuario se trata como una tarea: reproducir, corregir, test, checks, commit. Los commits usan `fix: <resumen>`, `docs: <resumen>` o `chore: <resumen>` en inglés; la convención `F<fase>-T<tarea>` terminó con el plan.
- Las reglas de `CLAUDE.md` siguen valiendo enteras: sin medidas inventadas, desvíos en `99`, sin dependencias nuevas sin registro, sin funciones de versión 2, sin `sudo`, sin tocar calendarios reales fuera de "UGC Fixtures".
- Antes de cada commit: `cargo clippy -D warnings`, `cargo test`, `npm run lint && npm run typecheck && npm test`, `node scripts/check-tokens.mjs`. Si el cambio toca UI medida, además `node scripts/measure/report.mjs --gate 7` con dumps frescos de la app (sección 6).
- Cambios de comportamiento acordados con el usuario que contradigan `01` a `07` van primero a `99` y luego al código.

## 5. Reproducir y depurar sin tocar el escritorio del usuario

Reglas fijas, pedidas por el usuario:

- Nunca lanzar la app en su display. Si él no tiene que hacer nada en la app, no quiere verla. Los arranques van a un Xvfb propio.
- Nunca compilar con la app de desarrollo abierta y siempre con `CARGO_BUILD_JOBS=2` (o `-j 2`). La máquina tiene unos 14 GB y ya hubo un cierre de terminal por falta de memoria.
- Matar procesos por PID con `pgrep -f` y el truco del corchete, nunca con `pkill -f`: `pkill -f` coincide con la propia línea de comandos del shell y lo mata (pasó dos veces).

```bash
export PATH="$HOME/.cargo/bin:$PATH"

# Binario de desarrollo (una vez por cambio en Rust)
CARGO_BUILD_JOBS=2 cargo build --manifest-path src-tauri/Cargo.toml

# Display privado y app en modo desarrollo (Vite en 5173 + binario debug)
Xvfb :150 -screen 0 1440x900x24 -nolisten tcp &
(npx vite --port 5173 --strictPort &)
DISPLAY=:150 src-tauri/target/debug/unified-google-calendar &

# Captura de pantalla y logs
DISPLAY=:150 scrot -o /tmp/claude-1000/shot.png
tail -n 100 ~/.local/share/unified-google-calendar/logs/app.$(date +%F).log

# Cierre limpio
kill $(pgrep -f "^src-tauri/target/debug/unified-google-calenda[r]") \
     $(pgrep -f "^node .*/vit[e]$") \
     $(pgrep -f "^Xvfb :15[0]")
```

`scripts/measure/app-capture.mjs` hace todo eso solo (Xvfb, Vite, binario, acciones y volcados) y es la forma preferida cuando lo que se depura es layout. Las acciones disponibles (`view`, `date`, `calendars`, `scroll`, `click`, `click_at`, `type`, `key`, `hover`) están en `scripts/measure/app-components.mjs`; el endpoint `POST http://127.0.0.1:8080/dev/measure` solo existe en builds de desarrollo.

Datos del usuario que se pueden leer para diagnosticar, nunca modificar a mano ni volcar en logs o commits:

| Ruta | Uso al depurar |
|---|---|
| `~/.local/share/unified-google-calendar/logs/app.<AAAA-MM-DD>.log` | Primer lugar donde mirar. Rotación diaria, 7 archivos. |
| `~/.local/share/unified-google-calendar/data.db` | Consultar con `sqlite3` en modo solo lectura (`-readonly`). Las tablas están en `03`. |
| `~/.local/share/unified-google-calendar/tokens.bin` | Cifrado. Contiene tokens OAuth y la URL secreta del iCal. No abrir. |
| `~/.config/unified-google-calendar/oauth.json` | Client id y secret del usuario. No leer al repo ni mostrar. |

La app instalada (`/usr/bin/unified-google-calendar`) y el binario de desarrollo comparten esas rutas por defecto, y las dos usan `tauri-plugin-single-instance` y el puerto 8080 del webhook. Mientras el usuario tenga la app abierta, la instancia de depuración arranca con directorios propios para no pisar sus datos ni su puerto:

```bash
XDG_CONFIG_HOME=/tmp/claude-1000/ugc-debug/config XDG_DATA_HOME=/tmp/claude-1000/ugc-debug/data \
  DISPLAY=:150 dbus-run-session -- src-tauri/target/debug/unified-google-calendar &
```

`dbus-run-session` hace falta porque `tauri-plugin-single-instance` toma el nombre `com.greelow.unifiedgooglecalendar` en el bus de sesión: sin bus propio, el segundo proceso solo muestra la ventana de la instancia instalada y termina. Con bus propio esa instancia no ve GNOME (notificaciones, Online Accounts, EDS); para depurar eso, el usuario cierra su app primero.

Esa instancia empieza vacía: copiar `oauth.json` al `XDG_CONFIG_HOME` de prueba solo si hace falta una cuenta Google, y agregar entonces la cuenta de Greelow con el calendario "UGC Fixtures" desde la propia app (el navegador del login se abre en `:150`; `scripts/measure/session.mjs` explica cómo se hizo la primera vez). Para reproducir un bug con los datos reales del usuario, él cierra antes la app desde la bandeja y avisa; nunca lanzarla con sus datos mientras la instalada corre.

## 6. Volver a medir contra Google

El perfil `~/.chrome-measure` sigue con sesión iniciada en la cuenta de Greelow. Pasos, todos sin el usuario:

```bash
node scripts/measure/reference-state.mjs on        # deja visible solo "UGC Fixtures" en Google
node scripts/measure/capture.mjs <componente[:estado]>   # docs/design/measurements/<comp>-<light|dark>.json/.png
node scripts/measure/extract-tokens.mjs            # token-spec.json + dumps -> tokens.json
node scripts/gen-tokens.mjs && node scripts/gen-measured-css.mjs
node scripts/measure/app-capture.mjs <componente>  # dumps de la app en Xvfb :150
node scripts/measure/report.mjs --md --gate 7      # comparación; tolerancias en measurements/<comp>.md
node scripts/measure/reference-state.mjs off       # vuelve a marcar todos los calendarios del usuario
```

Si la sesión de Chrome caducó, `scripts/measure/session.mjs` lo detecta; el usuario tiene que iniciar sesión otra vez en ese perfil (`09` sección F). Es la única parte que lo necesita.

## 7. Reinstalar tras un cambio

```bash
TAURI_LINUX_AYATANA_APPINDICATOR=1 CARGO_BUILD_JOBS=2 npm run tauri build -- --bundles deb
bash scripts/bump-version.sh <x.y.z>   # solo si cambia la versión
```

El `sudo apt install "./src-tauri/target/release/bundle/deb/..."` lo corre el usuario. El build de release tarda varios minutos y usa `lto` con `codegen-units = 1`; no lanzarlo con otra compilación en curso.
