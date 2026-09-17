# 10 — Estado al cierre de la implementación y guía de mantenimiento

Escrito el 2026-09-15, al terminar el plan de `08`. Desde acá el proyecto está en modo mantenimiento: el usuario usa la app instalada, reporta lo que ve y quien implementa lo corrige. Este documento dice qué hay, qué falta, qué se sabe que no está perfecto y cómo trabajar sin molestar al usuario.

## 1. Estado

- **Transición a Material 3, 2026-09-16 (versión 0.2.0).** Toda la UI se reescribió sobre `@material/web` 2.5.0, React 19.3 y los tokens de `docs/design/m3/theme.json` (`docs/11-material3.md`); el plan de `docs/12-plan-m3.md` se ejecutó entero en una sesión, commits `M0-T1` a `M6: transition closed`. Cambió lo que se ve (superficies, controles con ripple y foco, chips tonales, diálogos `md-dialog`, grilla de 48 px por hora, cajón de 300 px), no lo que hace cada control ni el backend, salvo el borrado del comando `dev_dump` y la ruta `/dev/measure` (solo depuración). `QuickCreate` desapareció. Se borraron la réplica de Google y su pipeline de medición (46 MB → 38 MB de repo sin `node_modules`, `target` ni `.git`); las mediciones quedan en `docs/design/google/`. RAM en release sobre Xvfb: 202.6 MB abierta en vista semana (77.6 MB el proceso Rust, 110.6 MB el WebKitWebProcess), 44 MB menos que 0.1.6 (`99`, fila M6). Paquete de la transición: 0.2.0 (borrado el 2026-09-17 junto con 0.1.6 y 0.2.1; en `src-tauri/target/release/bundle/deb/` quedan la entrega anterior y la actual). Gate: `bash scripts/check-m3.sh`.
- Fases F0 a F8 cerradas con sus gates (`bash scripts/check-phase.sh 0..8`). Último commit del plan: `a8094e3` ("F8: phase closed"), en `origin/main`.
- Sesión de mantenimiento del 2026-09-15 ("carril rápido", once entregas 0.1.0 → 0.1.5): header reordenado y reducido, ícono con el día del mes en bandeja, ventana y dock, ronda completa de animaciones (componentes 21 a 24: tooltip, animaciones, vista año, snackbar), vista año, formulario de evento como modal, cajón sin bloques inertes, diálogo "Add calendar", popup de evento sin controles inertes, renderer DMABUF apagado, y las correcciones de `docs/99` (entradas del 2026-09-15). La app ya no es una réplica pixel a pixel del header, del formulario ni del cajón: el usuario decidió cada desvío; las mediciones siguen siendo la fuente de cada valor.
- Sesión del 2026-09-16 (0.1.6): el dock de GNOME se quedaba con el día anterior porque el Shell no rescanea el tema si no cambia el mtime de `~/.local/share/icons/hicolor`; `tray.rs` lo toca tras escribir los PNG (sección 3, "Ícono del día").
- Sesión del 2026-09-16 (0.2.1): el espejo de Evolution Data Server ignoraba el interruptor de visibilidad de los calendarios y dejaba en el panel de GNOME las fuentes de calendarios ocultos en la app (feriados de Argentina por triplicado). `mirror.rs` ahora quita la fuente de un calendario oculto y la vuelve a crear al mostrarlo (`99`, 2026-09-16).
- Sesión del 2026-09-17 (0.2.3 y 0.2.4): el usuario redujo la regla a tres botones (primario `md-filled-button`, secundario `md-text-button`, `md-icon-button`); `md-fab`, `md-outlined-button` y `md-assist-chip` ya no se registran. La etiqueta de la bandeja se reafirma cada minuto con un espacio de ancho cero alternado porque la extensión AppIndicator perdía la primera actualización (`99`, 2026-09-17); confirmado en vivo, y desde 0.2.4 la segunda pasada va 5 s después del arranque.
- Sesión del 2026-09-16 (0.2.2, "carril rápido" sobre la UI M3): inventario de las doce familias de botón y unificación según la regla nueva de `docs/11` sección 8 (una `md-filled-button` por diálogo, `md-outlined-button` para el peso medio, `md-text-button` para descartar y agregar filas, `md-assist-chip` solo para salir de la app desde el popup); `md-filled-tonal-button` ya no se usa; `src/components/DayNumber.tsx` reemplaza los cuatro círculos de día (`99`, "Jerarquía única de botones").
- Entregas anteriores: 0.1.6 (2026-09-16, ícono del dock) y 0.1.5 (2026-09-15). Cada entrega sube la versión con `scripts/bump-version.sh`: `apt install` no reinstala la misma versión. La app queda en la bandeja al cerrar la ventana; hay que hacer **Quit** antes de instalar, y volver a abrirla después, porque la interfaz va embebida en el binario.
- Cuentas en la app del usuario: dos cuentas Google corporativas de Greelow y un calendario iCal de solo lectura (RappiCard, publicado por un Apps Script a un gist secreto; la URL vive cifrada en `tokens.bin`). El Gmail personal queda para más adelante, por decisión del usuario. El Apps Script emite `X-GOOGLE-CONFERENCE` desde el 2026-09-15 (servicio avanzado Calendar).
- Verificación al cierre de la sesión del 2026-09-15: `cargo clippy --all-targets -D warnings`, `cargo fmt --check`, 118 tests de Rust, `npm run lint`, `npm run typecheck`, 19 tests de vitest, `check-tokens.mjs`. `report.mjs --gate 4/7` no se volvió a correr: los desvíos pedidos por el usuario (header, cajón, formulario, menús) dejan sin sentido la comparación de layout de esos componentes; los demás no cambiaron.
- Medición (histórica, sin vigencia desde el 2026-09-16): 24 componentes de calendar.google.com en claro y oscuro, más la paleta de colores, conservados en `docs/design/google/`.
- Sesión del usuario: Wayland, GPU AMD, dos monitores a escala 1. El renderer DMABUF de WebKitGTK 2.50 deja el texto borroso ahí; `main.rs` lo apaga.
- Memoria en release sobre Xvfb (render por software) antes de la transición: 246.6 MB abierta, 91.7 MB el proceso Rust con la ventana oculta (`99`, F8-T2); después, 202.6 MB abierta (`99`, fila M6).

## 2. Pendiente del lado del usuario

1. Seguir usando la app y reportar lo que vea. Recorrido hecho el 2026-09-15: semana, día, mes, año, agenda, popup, formulario, recurrencia, alcance de edición, cajón, header, bandeja.
2. Responder el diálogo de Online Accounts la primera vez (`09` sección D).
3. Con la app en vista semana, correr `bash scripts/measure-ram.sh` y pegar la tabla en `99`, sección "Mediciones de RAM por fase", con la nota "sesión real con GPU".
4. Opcional: Tailscale Funnel y la URL pública en Settings (`09` sección B). Sin eso la app hace polling cada 60 s.
5. Hecho el 2026-09-15: el calendario de prueba "UGC Fixtures" se borró (`docs/99`). Ya no hace falta para nada: la UI no se mide contra Google.
6. Ajustes de la cuenta Greelow que la medición de 2026-09-14 cambió y siguen así: formato 24 h, semana desde lunes, zona secundaria America/Mexico_City, apariencia "Device default", colores "Modern", densidad "Responsive to your screen", panel lateral oculto. El usuario los puede cambiar cuando quiera.

## 3. Limitaciones conocidas

| Tema | Detalle | Dónde está registrado |
|---|---|---|
| RAM | 202.6 MB abierta contra 150 MB de objetivo (77.6 MB Rust, 110.6 MB WebKitWebProcess). Medido en Xvfb con llvmpipe, que suma `libLLVM` y `libgallium` al proceso Rust. Falta la cifra real con GPU. | `99`, F8-T2 y M6 |
| Hover y animaciones | Los componentes `@material/web` traen ripple, capa de estado y anillo de foco; los elementos propios (días, chips, filas) llevan `md-ripple` y `md-focus-ring`. Las transiciones de la app están en `theme.json` (`app_motion`). `prefers-reduced-motion` no se contempla (`docs/11` sección 10). | `docs/11`, secciones 8 y 10 |
| md-select con valor vacío | `md-outlined-select` no vuelve a resolver un `value` de cadena vacía cuando sus opciones se renderizan, y solo resuelve un valor contra opciones ya presentes en el DOM. Los "ninguno" usan el valor `none` y el select de repetición vuelve a fijar su valor en un efecto cuando aparece la opción personalizada. | `src/event/FullForm.tsx`, `src/app/SettingsDialog.tsx` |
| Renderer del webview | La app arranca con `WEBKIT_DISABLE_DMABUF_RENDERER=1` (`main.rs`) porque el renderer DMABUF dejaba el texto borroso en la sesión del usuario. Si en otra máquina hiciera falta el DMABUF, exportar la variable con `0` antes de lanzar la app. | `99`, 2026-09-15 |
| Meet en RappiCard | El feed iCal de RappiCard (Apps Script) no trae `X-GOOGLE-CONFERENCE`, ni LOCATION ni DESCRIPTION, así que el popup no puede mostrar el enlace de Meet. Arreglo del lado del script: emitir `X-GOOGLE-CONFERENCE:<hangoutLink>` (servicio avanzado de Calendar en Apps Script, `Calendar.Events.get(...).hangoutLink`) o al menos el enlace en DESCRIPTION; la app lo toma de cualquiera de los tres. | `99`, 2026-09-15 |
| Ícono del día | Bandeja y ventana lo reciben por Tauri; el dock lo lee de `~/.local/share/icons/hicolor/*/apps/unified-google-calendar.png`, que la app reescribe a medianoche. GNOME Shell solo vuelve a leer el tema cuando cambia el mtime de `~/.local/share/icons/hicolor`, así que `tray.rs` toca ese directorio después de escribir los PNG (0.1.6, reporte del 2026-09-16: el dock se había quedado con el 15). El Shell lo detecta unos 5 s después de cualquier búsqueda de ícono. | `99`, 2026-09-15 y 2026-09-16 |
| Deduplicación | Google oculta copias de un mismo evento invitado a varias cuentas de forma distinta a la app. | `docs/design/google/week.md` (histórico) |
| Scrollbars | Se dejan las barras overlay de GTK sin estilo; estilizar `::-webkit-scrollbar` fuerza barras clásicas en WebKitGTK. La grilla de la semana es el único scroller vertical; la cabecera y la fila de todo el día quedan fijas encima. | `99` 2026-09-15 |
| Popovers dentro de diálogos | El selector de fecha y los menús se posicionan con `position: fixed` calculado por JS porque WebKitGTK 2.50 no tiene Popover API ni anchor positioning; dentro de un `md-dialog` (top layer) funciona porque el diálogo no crea contexto de contención una vez terminada su animación. | `docs/11` sección 2, `src/event/DatePicker.tsx` |
| Creación rápida | El click en la grilla no abre nada (decisión del usuario); `QuickCreate` se borró en M3-T2. | `99`, 2026-09-15 y 2026-09-16 |
| Undo | El snackbar no ofrece "Undo" tras guardar o borrar. | `docs/11` sección 7 |
| Etiqueta de la bandeja | La extensión AppIndicator de GNOME puede perder una actualización de la etiqueta y no la repinta hasta recibir un valor distinto; la app alterna un espacio de ancho cero por minuto para forzarlo. Si la etiqueta falta, comprobar con `busctl --user get-property :1.N /org/ayatana/NotificationItem/tray_icon_tray_app_main org.kde.StatusNotifierItem XAyatanaLabel` que la app la puso. | `99`, 2026-09-17 |

## 4. Cómo se trabaja desde ahora

- Cada reporte del usuario se trata como una tarea: reproducir, corregir, test, checks, commit. Los commits usan `fix: <resumen>`, `docs: <resumen>` o `chore: <resumen>` en inglés; la convención `F<fase>-T<tarea>` terminó con el plan.
- Las reglas de `CLAUDE.md` siguen valiendo enteras: ningún valor fuera de `theme.json`, desvíos en `99`, sin dependencias nuevas sin registro, sin funciones de versión 2, sin `sudo`, sin tocar calendarios reales fuera de "UGC Fixtures".
- Antes de cada commit: `cargo clippy -D warnings`, `cargo test`, `npm run lint && npm run typecheck && npm test`, `node scripts/check-tokens.mjs`. Si el cambio toca la UI, además `bash scripts/check-m3.sh` y una mirada en Xvfb en claro y oscuro (sección 5).
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
# GDK_BACKEND=x11 y sin WAYLAND_DISPLAY: si no, GTK ignora DISPLAY y la ventana sale en el
# escritorio del usuario (pasó el 2026-09-16). GTK_THEME=Adwaita:dark para el tema oscuro.
env -u WAYLAND_DISPLAY GDK_BACKEND=x11 DISPLAY=:150 src-tauri/target/debug/unified-google-calendar &

# Captura de pantalla y logs
DISPLAY=:150 scrot -o /tmp/claude-1000/shot.png
tail -n 100 ~/.local/share/unified-google-calendar/logs/app.$(date +%F).log

# Cierre limpio
kill $(pgrep -f "^src-tauri/target/debug/unified-google-calenda[r]") \
     $(pgrep -f "^node .*/vit[e]$") \
     $(pgrep -f "^Xvfb :15[0]")
```

La ventana tarda unos 25 s en aparecer en Xvfb (Vite compila al primer pedido); `xdotool search --onlyvisible --name 'Unified Google Calendar'` dice cuándo está. Para interactuar: `xdotool mousemove X Y click 1`, `xdotool type`, `xdotool key Escape`; `scrot` captura. Vite recarga la página con cada cambio del frontend; un cambio de `vite.config.ts` reinicia el servidor y deja al webview en una página de error: hay que relanzar el binario.

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
  env -u WAYLAND_DISPLAY GDK_BACKEND=x11 DISPLAY=:150 dbus-run-session -- src-tauri/target/debug/unified-google-calendar &
```

`dbus-run-session` hace falta porque `tauri-plugin-single-instance` toma el nombre `com.greelow.unifiedgooglecalendar` en el bus de sesión: sin bus propio, el segundo proceso solo muestra la ventana de la instancia instalada y termina. Con bus propio esa instancia no ve GNOME (notificaciones, Online Accounts, EDS); para depurar eso, el usuario cierra su app primero.

Esa instancia empieza vacía, con el calendario local "Personal": crear ahí los eventos de prueba desde el formulario (o con un `import('./ipc')` temporal en `main.tsx` que llame a `ipc.createEvent`, quitado antes del commit). Copiar `oauth.json` al `XDG_CONFIG_HOME` de prueba solo si hace falta una cuenta Google, y agregar entonces la cuenta de Greelow con el calendario "UGC Fixtures" desde la propia app (el navegador del login se abre en `:150`). Para reproducir un bug con los datos reales del usuario, él cierra antes la app desde la bandeja y avisa; nunca lanzarla con sus datos mientras la instalada corre.

## 6. Revisar la UI

Ya no se mide contra Google. Para revisar la interfaz: la app en el Xvfb privado (sección 5), capturas en claro y en oscuro (`GTK_THEME=Adwaita:dark` al lanzar el binario), leídas con la herramienta de imágenes y comparadas con `docs/11-material3.md` sección 7 y `docs/design/m3/mockup.html`. Estados de hover y presión con `xdotool mousemove` y `mousedown`/`mouseup`. `bash scripts/check-m3.sh` verifica que los generados coincidan con `theme.json`, que ninguna hoja tenga literales y que no vuelva nada de la réplica.

## 7. Reinstalar tras un cambio

```bash
TAURI_LINUX_AYATANA_APPINDICATOR=1 CARGO_BUILD_JOBS=2 npm run tauri build -- --bundles deb
bash scripts/bump-version.sh <x.y.z>   # solo si cambia la versión
```

El `sudo apt install "./src-tauri/target/release/bundle/deb/..."` lo corre el usuario. El build de release tarda varios minutos y usa `lto` con `codegen-units = 1`; no lanzarlo con otra compilación en curso.

## Registro de cambios

- 2026-09-17 — Tres botones y etiqueta de la bandeja (0.2.3): sección 1 con la entrega, sección 3 con la limitación de la extensión AppIndicator.
- 2026-09-16 — Unificación de botones (0.2.2): sección 1 con la entrega.
- 2026-09-16 — Espejo de EDS y calendarios ocultos (0.2.1): sección 1 con la entrega, `docs/06` sección 4.5 con la regla.
- 2026-09-16 — Transición a Material 3 ejecutada y cerrada (0.2.0): sección 1 reescrita, sección 3 sin las limitaciones de la réplica (odiff, desvíos de Google, scrollbars medidas) y con las de M3 (`md-select`, popovers en diálogos), sección 4 con `check-m3.sh`, sección 5 con `GDK_BACKEND=x11`, sección 6 reescrita ("Revisar la UI").
- 2026-09-16 — Sesión de planificación de la transición a Material 3: el usuario eligió "M3 total" sobre un mockup de tres opciones (`docs/design/m3/mockup.html`). Quedan escritos `docs/11-material3.md`, `docs/12-plan-m3.md`, `docs/design/m3/theme.json` (con `scripts/gen-m3-tokens.mjs` probado), `scripts/check-m3.sh`, el skill `m3-transition` y la entrada de `99`. El código en `main` sigue siendo la réplica de Google hasta que se ejecute el plan; mientras tanto, cualquier reporte del usuario se atiende como mantenimiento sobre el código actual sin invertir en fidelidad con Google. Al cerrar la transición, la sección 1 de este documento se reescribe (`docs/12` sección 7).
