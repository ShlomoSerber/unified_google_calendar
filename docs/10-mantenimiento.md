# 10 — Estado al cierre de la implementación y guía de mantenimiento

Escrito el 2026-09-15, al terminar el plan de `08`. Desde acá el proyecto está en modo mantenimiento: el usuario usa la app instalada, reporta lo que ve y quien implementa lo corrige. Este documento dice qué hay, qué falta, qué se sabe que no está perfecto y cómo trabajar sin molestar al usuario.

## 1. Estado

- Fases F0 a F8 cerradas con sus gates (`bash scripts/check-phase.sh 0..8`). Último commit del plan: `a8094e3` ("F8: phase closed"), en `origin/main`.
- Sesión de mantenimiento del 2026-09-15 ("carril rápido", once entregas 0.1.0 → 0.1.5): header reordenado y reducido, ícono con el día del mes en bandeja, ventana y dock, ronda completa de animaciones (componentes 21 a 24: tooltip, animaciones, vista año, snackbar), vista año, formulario de evento como modal, cajón sin bloques inertes, diálogo "Add calendar", popup de evento sin controles inertes, renderer DMABUF apagado, y las correcciones de `docs/99` (entradas del 2026-09-15). La app ya no es una réplica pixel a pixel del header, del formulario ni del cajón: el usuario decidió cada desvío; las mediciones siguen siendo la fuente de cada valor.
- Sesión del 2026-09-16 (0.1.6): el dock de GNOME se quedaba con el día anterior porque el Shell no rescanea el tema si no cambia el mtime de `~/.local/share/icons/hicolor`; `tray.rs` lo toca tras escribir los PNG (sección 3, "Ícono del día").
- Paquete: `src-tauri/target/release/bundle/deb/Unified Google Calendar_0.1.6_amd64.deb`, entregado el 2026-09-16 (0.1.5 instalado el 2026-09-15; su `.deb` queda como respaldo hasta confirmar 0.1.6). Cada entrega sube la versión con `scripts/bump-version.sh`: `apt install` no reinstala la misma versión. La app queda en la bandeja al cerrar la ventana; hay que hacer **Quit** antes de instalar, y volver a abrirla después, porque la interfaz va embebida en el binario.
- Cuentas en la app del usuario: dos cuentas Google corporativas de Greelow y un calendario iCal de solo lectura (RappiCard, publicado por un Apps Script a un gist secreto; la URL vive cifrada en `tokens.bin`). El Gmail personal queda para más adelante, por decisión del usuario. El Apps Script emite `X-GOOGLE-CONFERENCE` desde el 2026-09-15 (servicio avanzado Calendar).
- Verificación al cierre de la sesión del 2026-09-15: `cargo clippy --all-targets -D warnings`, `cargo fmt --check`, 118 tests de Rust, `npm run lint`, `npm run typecheck`, 19 tests de vitest, `check-tokens.mjs`. `report.mjs --gate 4/7` no se volvió a correr: los desvíos pedidos por el usuario (header, cajón, formulario, menús) dejan sin sentido la comparación de layout de esos componentes; los demás no cambiaron.
- Medición: 24 componentes de calendar.google.com (20 originales más tooltip, animaciones, vista año y snackbar) en claro y oscuro, más la paleta de 24 colores de calendario y 11 de evento (`docs/design/measurements/`, `docs/design/tokens.json`).
- Sesión del usuario: Wayland, GPU AMD, dos monitores a escala 1. El renderer DMABUF de WebKitGTK 2.50 deja el texto borroso ahí; `main.rs` lo apaga.
- Memoria en release sobre Xvfb (render por software): 246.6 MB abierta, 91.7 MB el proceso Rust con la ventana oculta. Ver `99`, entrada de F8-T2.

## 2. Pendiente del lado del usuario

1. Seguir usando la app y reportar lo que vea. Recorrido hecho el 2026-09-15: semana, día, mes, año, agenda, popup, formulario, recurrencia, alcance de edición, cajón, header, bandeja.
2. Responder el diálogo de Online Accounts la primera vez (`09` sección D).
3. Con la app en vista semana, correr `bash scripts/measure-ram.sh` y pegar la tabla en `99`, sección "Mediciones de RAM por fase", con la nota "sesión real con GPU".
4. Opcional: Tailscale Funnel y la URL pública en Settings (`09` sección B). Sin eso la app hace polling cada 60 s.
5. Hecho el 2026-09-15: el calendario de prueba "UGC Fixtures" se borró (`docs/99`). Para volver a medir chips contra Google hay que recrearlo con `cargo run --example seed_fixtures -- shlomo.serber@greelow.com --seed` y borrarlo después.
6. Ajustes de la cuenta Greelow que la medición cambió y siguen así: formato 24 h, semana desde lunes, zona secundaria America/Mexico_City, apariencia "Device default", colores "Modern", densidad "Responsive to your screen", panel lateral oculto. Las casillas de calendarios del sidebar ya están todas marcadas otra vez. El usuario los puede cambiar cuando quiera, salvo si va a remedir.

## 3. Limitaciones conocidas

| Tema | Detalle | Dónde está registrado |
|---|---|---|
| odiff | El criterio de 0.5 % de píxeles distintos no se cumple: WebKitGTK y Chrome rasterizan el texto distinto. El layout (posición y tamaño de cada nodo) sí coincide. | `99`, "Criterio de aceptación visual" |
| RAM | 246.6 MB abierta contra 150 MB de objetivo; 91.7 MB Rust oculto contra 60 MB. Medido en Xvfb con llvmpipe. Falta la cifra real con GPU. | `99`, F8-T2 |
| Hover y animaciones | Medidos e implementados el 2026-09-15 (`docs/04` sección 10, `measurements/animations.md`). Los chips no tienen estado hover en Google. Quedan fuera los anillos de foco por teclado (versión 2). | `99`, 2026-09-15 |
| Renderer del webview | La app arranca con `WEBKIT_DISABLE_DMABUF_RENDERER=1` (`main.rs`) porque el renderer DMABUF dejaba el texto borroso en la sesión del usuario. Si en otra máquina hiciera falta el DMABUF, exportar la variable con `0` antes de lanzar la app. | `99`, 2026-09-15 |
| Meet en RappiCard | El feed iCal de RappiCard (Apps Script) no trae `X-GOOGLE-CONFERENCE`, ni LOCATION ni DESCRIPTION, así que el popup no puede mostrar el enlace de Meet. Arreglo del lado del script: emitir `X-GOOGLE-CONFERENCE:<hangoutLink>` (servicio avanzado de Calendar en Apps Script, `Calendar.Events.get(...).hangoutLink`) o al menos el enlace en DESCRIPTION; la app lo toma de cualquiera de los tres. | `99`, 2026-09-15 |
| Ícono del día | Bandeja y ventana lo reciben por Tauri; el dock lo lee de `~/.local/share/icons/hicolor/*/apps/unified-google-calendar.png`, que la app reescribe a medianoche. GNOME Shell solo vuelve a leer el tema cuando cambia el mtime de `~/.local/share/icons/hicolor`, así que `tray.rs` toca ese directorio después de escribir los PNG (0.1.6, reporte del 2026-09-16: el dock se había quedado con el 15). El Shell lo detecta unos 5 s después de cualquier búsqueda de ícono. | `99`, 2026-09-15 y 2026-09-16 |
| Deduplicación | Google oculta copias de un mismo evento invitado a varias cuentas de forma distinta a la app en el calendario de prueba. | `docs/design/measurements/week.md` |
| Settings | El diálogo de Settings reutiliza tokens de los diálogos medidos; la página de ajustes de Google no se midió. | `src/app/SettingsDialog.tsx` |
| Scrollbars | Se dejan las barras overlay de GTK sin estilo; estilizar `::-webkit-scrollbar` fuerza barras clásicas en WebKitGTK. Los scrollers que Google oculta (cabecera, fila de todo el día, gutter) van con `overflow` hidden: su pulgar se dibujaba encima de los menús. | `docs/design/measurements/scrollbars.md`, `99` 2026-09-15 |
| Desvíos de Google | Header, cajón, botón Create, menú de vista, formulario de evento y popup ya no replican el DOM de Google: son la versión reducida que pidió el usuario, construida con los mismos tokens medidos. `report.mjs` no aplica a esos componentes. | `99`, entradas del 2026-09-15 |
| Creación rápida | El click en la grilla no abre nada (decisión del usuario); `QuickCreate.tsx` queda sin disparador. | `99`, 2026-09-15 |
| Undo | El snackbar no ofrece "Undo" tras guardar o borrar. | `docs/design/measurements/snackbar.md` |

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
node scripts/measure/animations.mjs [escenario]    # movimiento: animations-<tema>.json (docs/04 sección 10)
```

Los escenarios de `animations.mjs` no necesitan "UGC Fixtures": usan el primer chip visible de la semana actual, la creación rápida se descarta y el formulario completo se cierra sin guardar. El depurador de la app en Xvfb (sección 5) sirve para ver las animaciones con entrada real: `xdotool mousemove/click` y `scrot` en ráfaga (unos 75 ms por captura con llvmpipe).

Si la sesión de Chrome caducó, `scripts/measure/session.mjs` lo detecta; el usuario tiene que iniciar sesión otra vez en ese perfil (`09` sección F). Es la única parte que lo necesita.

## 7. Reinstalar tras un cambio

```bash
TAURI_LINUX_AYATANA_APPINDICATOR=1 CARGO_BUILD_JOBS=2 npm run tauri build -- --bundles deb
bash scripts/bump-version.sh <x.y.z>   # solo si cambia la versión
```

El `sudo apt install "./src-tauri/target/release/bundle/deb/..."` lo corre el usuario. El build de release tarda varios minutos y usa `lto` con `codegen-units = 1`; no lanzarlo con otra compilación en curso.

## Registro de cambios

- 2026-09-16 — Sesión de planificación de la transición a Material 3: el usuario eligió "M3 total" sobre un mockup de tres opciones (`docs/design/m3/mockup.html`). Quedan escritos `docs/11-material3.md`, `docs/12-plan-m3.md`, `docs/design/m3/theme.json` (con `scripts/gen-m3-tokens.mjs` probado), `scripts/check-m3.sh`, el skill `m3-transition` y la entrada de `99`. El código en `main` sigue siendo la réplica de Google hasta que se ejecute el plan; mientras tanto, cualquier reporte del usuario se atiende como mantenimiento sobre el código actual sin invertir en fidelidad con Google. Al cerrar la transición, la sección 1 de este documento se reescribe (`docs/12` sección 7).
