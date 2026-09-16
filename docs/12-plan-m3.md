# 12 — Plan de la transición a Material 3

Siete fases, M0 a M6, en orden, pensadas para ejecutarse **enteras en una sola sesión** con el skill `m3-transition`. Cada tarea `M<fase>-T<n>` se cierra cuando cumple todos sus criterios de aceptación y pasan sus comprobaciones; se commitea con `M<fase>-T<n>: <resumen en inglés>`. No se empieza una fase sin cerrar la anterior. La transición se cierra con la sección 7.

Documentos que aplican siempre: `CLAUDE.md`, `docs/11-material3.md` (la especificación; "11 §N" abajo), `docs/design/m3/theme.json`, `docs/99-decisiones.md` (entrada 2026-09-16). Cada tarea lista además lo que necesita.

Comprobaciones mínimas de cada tarea del frontend (abreviadas abajo como **checks**):

```bash
node scripts/gen-m3-tokens.mjs && node scripts/check-tokens.mjs && npm run lint && npm run typecheck && npm test
```

Verificación visual (abreviada como **mirar**): la app en el Xvfb privado con los directorios `XDG_*` de depuración (`docs/10` sección 5), captura en claro y en oscuro, leídas con la herramienta de imágenes, comparadas con 11 §7 y con `docs/design/m3/mockup.html`. Sin app abierta durante `cargo build`; `CARGO_BUILD_JOBS=2`; matar por PID.

Estado intermedio: desde M0-T2 hasta M5-T1 conviven dos hojas de tokens (`src/styles/tokens.css` nueva y `src/styles/legacy.css`, copia congelada de la vieja) y `measured.css`. Cada componente migrado deja de usar `legacy.css` y `measured.css`; `check-tokens.mjs` acepta las dos hojas mientras `legacy.css` exista. Así cada commit compila y pasa las comprobaciones aunque la UI esté a medias.

## Fase M0 — Fundaciones

Objetivo: dependencias, pipeline de tokens, fuentes, íconos y la integración de `@material/web` con React 19 listas, con la app vieja todavía funcionando encima.

### M0-T1 Dependencias y React 19
- Docs: 11 §2, 11 §5.
- Pasos: `npm install --save-exact react@19.3.0 react-dom@19.3.0 @material/web@2.5.0` y `npm install --save-dev --save-exact @types/react@19 @types/react-dom@19 @material/material-color-utilities@0.3.0` (la versión exacta de los `@types` es la última 19.x que npm devuelva; fijarla). Revisar `npm ls` sin peer warnings. Ajustar `src/main.tsx` si React 19 lo pide (`createRoot` no cambia). Correr la app vieja en Xvfb para confirmar que React 19 no rompió nada visible.
- Aceptación: `package.json` con las versiones exactas; `npm ci` limpio; checks verdes con la UI vieja; la app arranca en Xvfb y muestra la semana.

### M0-T2 Pipeline de tokens M3
- Docs: 11 §3, `scripts/gen-m3-tokens.mjs` (ya escrito y probado el 2026-09-16 con el paquete instalado en un directorio temporal).
- Pasos: `git mv src/styles/tokens.css src/styles/legacy.css` y actualizar los imports (`src/main.tsx`) para cargar `legacy.css`, luego `tokens.css` y `typescale.css` nuevos; `node scripts/gen-m3-tokens.mjs` (genera `tokens.css`, `typescale.css`, `layout.ts`, `motion.ts`, `palette.ts`). Como `layout.ts`, `motion.ts` y `palette.ts` los usan componentes viejos, mantener temporalmente `src/styles/legacy-layout.ts`, `legacy-motion.ts` y `legacy-palette.ts` (renombrados con `git mv` antes de generar) y apuntar los imports viejos a ellos. `scripts/gen-tokens.mjs` deja de correr (se borra en M5). `scripts/check-tokens.mjs`: las variables definidas se leen de `tokens.css` más `legacy.css` si existe; la regla de literales se mantiene; se agregan `--md-` y `--ugc-` a las familias admitidas. `src/styles/base.css`: `body` con `background: var(--md-sys-color-surface-container)`, `color: var(--md-sys-color-on-surface)` y la clase `md-typescale-body-medium`; quitar `--font-*`/`--color-*`.
- Aceptación: `tokens.css` generado con 246 tokens claros y 49 oscuros (la cifra puede variar con la versión de la librería, no con la edición a mano); `typescale.css` con 15 clases; checks verdes; la app vieja sigue arrancando (con `legacy.css` sus componentes no cambian).

### M0-T3 Fuentes, íconos y CSP
- Docs: 11 §4.
- Pasos: descargar y subconjuntar Material Symbols Outlined con el comando de 11 §4 hacia `src-tauri/resources/fonts/MaterialSymbolsOutlined-subset.woff2`; `src/styles/fonts.css` con los tres `@font-face` (Roboto, Roboto itálica, Google Sans Flex, Material Symbols Outlined) y sin `.icon`; quitar el `<link>` de Google Fonts de `index.html`; quitar `https://fonts.googleapis.com` y `https://fonts.gstatic.com` de `csp` y `devCsp` en `src-tauri/tauri.conf.json`; actualizar `src-tauri/resources/fonts/README.md`; borrar `GoogleMaterialIcons-subset.woff2` **al final de M2** (los componentes viejos lo usan hasta entonces; anotar en la tarea M5-T1). Verificar en Xvfb que `document.fonts.check('24px "Material Symbols Outlined"')` es verdadero (desde el inspector en dev o con un `console.log` temporal en `main.tsx` que se quita antes del commit).
- Aceptación: el woff2 nuevo pesa menos de 30 KB; la app arranca sin errores de CSP en el log; `grep fonts.googleapis index.html src-tauri/tauri.conf.json` no devuelve nada.

### M0-T4 `@material/web` en React
- Docs: 11 §5.
- Pasos: `src/m3/register.ts` con la lista de 11 §5; `src/m3/jsx.d.ts` con `IntrinsicElements` para cada etiqueta de la lista (y `md-focus-ring`, `md-ripple`, `md-divider`); `vitest.setup.ts` con el `vi.mock` de `register` y `setupFiles` en `vite.config.ts`; `src/main.tsx` importa `./m3/register` antes de `App`. Prueba de humo: un `md-filled-button` temporal en `App.tsx` que se ve en Xvfb con ripple al `xdotool click` y que se quita antes del commit.
- Aceptación: `npm run typecheck` acepta `<md-dialog ref={r} onclosed={...}>` y rechaza una etiqueta `md-*` no declarada; `npm test` pasa con el mock; en Xvfb el botón de prueba se dibujó con la fuente Roboto y el ripple.

## Fase M1 — Shell

Objetivo: barra superior, cajón, mini calendario, lista de calendarios, tooltip y snackbar en M3. La grilla sigue vieja.

### M1-T1 App y barra superior
- Docs: 11 §7 (fondo, barra, superficie principal), 11 §9 (`App.tsx`, `TopBar.tsx`, `ViewSelector.tsx`).
- Pasos: `App.css` con las superficies y el radio; `TopBar.tsx` + `.css` nuevos con el `md-menu` de vistas dentro; borrar `ViewSelector.tsx/.css` y `dialog.kind = 'view-menu'` de `src/state/ui.ts` (y su test); `App.tsx` deja de montar `ViewSelector`. `installRipples()` se quita de `App.tsx` (los componentes viejos que aún lo usaban pierden el ripple hasta migrar: aceptado).
- Aceptación: checks; mirar: barra con título title-large, botones `md-*`, menú de vistas abre con `positioning="fixed"` y cambia la vista; `src/app/App.test.tsx` actualizado (banner, botón "Today" por `aria-label`/texto, `main`).

### M1-T2 Cajón: Sidebar, CreateButton, MonthGrid, MiniCalendar
- Docs: 11 §7 (cajón, Create, mini calendario), 11 §9.
- Pasos: `src/components/MonthGrid.tsx` + `.css` (props: `monthTs`, `tz`, `todayTs`, `selected?: {from,to}`, `size: 'normal' | 'compact'`, `onPick(ts)`); `MiniCalendar.tsx` sobre `MonthGrid`; `Sidebar.tsx`/`.css`; `CreateButton.tsx` con `md-fab`; `App.tsx` ya no posiciona el botón en absoluto (`.app-create` desaparece).
- Aceptación: checks; mirar: FAB extendido, mini calendario con hoy en `primary` y la semana visible en `primary-container`, ripple en los días; `App.test.tsx` sigue encontrando el `grid` del mini calendario.

### M1-T3 Lista de calendarios
- Docs: 11 §6, 11 §7, 11 §9 (`CalendarList.tsx`).
- Pasos: `CalendarList.tsx` + `CalendarList.css` propio (sale de `Sidebar.css`); `md-checkbox` con los tokens de color por `--data-calendar-color`; `src/lib/colors.ts` con `chipColors` (11 §6) y `src/lib/colors.test.ts` actualizado (mapa conocido en claro y oscuro, fallback `color-mix`).
- Aceptación: checks; mirar: checkboxes del color de cada calendario, filas pill con hover, secciones que colapsan; apagar y encender un calendario desde Xvfb con `xdotool` cambia la grilla (vieja) y no deja errores en el log.

### M1-T4 Tooltip y Snackbar
- Docs: 11 §7, 11 §9, 11 §10.
- Pasos: `Tooltip.tsx` como envoltorio (API `<Tooltip text="...">`), usado en `TopBar`, `MiniCalendar` y `CalendarList`; `Snackbar.tsx` + `.css`; `src/lib/motion.ts` sin `installRipples`/`installTooltips`; `src/styles/motion.css` reducido a lo de 11 §9 con `--ugc-motion-*`. Los componentes viejos que aún tienen `data-tooltip` o `ugc-state` los pierden hasta migrar (aceptado; M5 verifica que no quede ninguno).
- Aceptación: checks; mirar: tooltip bajo "Today" tras 500 ms con `xdotool mousemove`; snackbar al crear un evento en el calendario de prueba local (cuenta `local`) con el formulario viejo.

## Fase M2 — Vistas

Objetivo: semana, día, mes, año y agenda en M3.

### M2-T1 Semana y día
- Docs: 11 §6, 11 §7 (cabecera, gutter, líneas, línea de ahora, chips), 11 §9 (`WeekView`, `WeekHeader`, `DayHeader`, `AllDayRow`, `EventChip`).
- Pasos: reescribir `WeekView.tsx/.css`, `WeekHeader.tsx`, `AllDayRow.tsx`, `EventChip.tsx`; borrar `DayHeader.tsx`; `LAYOUT` nuevo en el cálculo de posiciones; `layout.test.ts` no cambia.
- Aceptación: checks; mirar en semana y en día, claro y oscuro: chips tonales con borde, chip de 30 min de una línea, solapados en columnas, todo el día, línea de ahora en hoy, gutter con dos zonas cuando `secondary_tz` está fijada, sombra bajo la cabecera al scrollear; `App.test.tsx` encuentra 7 `columnheader`.

### M2-T2 Mes
- Docs: 11 §7, 11 §9 (`MonthView`).
- Aceptación: checks; mirar: celdas, número de hoy en pill, chips de todo el día de 20 px, puntos de eventos con hora, "N more" en un mes cargado (usar el calendario local de prueba para crear 6 eventos un mismo día y borrarlos después).

### M2-T3 Agenda
- Docs: 11 §7, 11 §9 (`AgendaView`).
- Aceptación: checks; mirar: grupos por día, círculo de hoy, filas de 48 px con ripple.

### M2-T4 Año
- Docs: 11 §7, 11 §9 (`YearView`); `MonthGrid` en modo `compact`.
- Aceptación: checks; mirar: doce tarjetas en cuatro columnas a 1440 px, hoy marcado, click en un día abre el día.

## Fase M3 — Evento

Objetivo: popup, formulario, selector de fecha, recurrencia y alcance en M3; `QuickCreate` fuera.

### M3-T1 Popup de evento
- Docs: 11 §7, 11 §9 (`EventPopup`), 11 §10.
- Aceptación: checks; mirar: abrir un chip en semana y en mes, popup a la derecha y a la izquierda (chip del domingo), Meet como chip, menú "Move to" con `md-menu`, borrar un evento del calendario local con snackbar.

### M3-T2 Formulario y selector de fecha
- Docs: 11 §5 (diálogos, eventos `closed`/`cancel`), 11 §7, 11 §9 (`FullForm`, `DatePicker`, `QuickCreate`).
- Pasos: `FullForm.tsx/.css` como `md-dialog`; `DatePicker.tsx/.css`; borrar `QuickCreate.tsx/.css` y `dialog.kind = 'quick-create'` del store; `writableCalendars` se muda con su uso.
- Aceptación: checks; mirar: crear un evento en el calendario local con hora, con todo el día, con Meet marcado (en local no crea Meet; la fila debe mostrarse igual), con dos notificaciones; editar y guardar; el diálogo cierra con Escape y con Cancel; la fecha se elige desde el popover.

### M3-T3 Recurrencia personalizada
- Docs: 11 §7, 11 §9 (`RecurrenceDialog`).
- Pasos: reescribir; agregar `src/event/recurrence.test.ts` con `parseRule`/`buildRule` (diario, semanal con BYDAY, mensual, anual, INTERVAL, COUNT, UNTIL, prefijo `RRULE:` tolerado).
- Aceptación: checks incluyendo los tests nuevos; mirar: abrir "Custom" desde el formulario, elegir semanal martes y jueves, "Ends after 5", Done, el formulario muestra la regla; guardar en local crea las ocurrencias en la grilla.

### M3-T4 Alcance de edición
- Docs: 11 §9 (`EditScopeDialog`).
- Aceptación: checks; mirar: editar la segunda ocurrencia de la serie de M3-T3 con "This and following" y borrar "All events"; el diálogo se apila sobre el formulario sin confundirse con él.

## Fase M4 — Diálogos

### M4-T1 Settings
- Docs: 11 §7, 11 §9 (`SettingsDialog`).
- Aceptación: checks; mirar: lista de cuentas como `md-list`, campos, selects, Save persiste `secondary_tz` (verificar releyendo `get_settings` en el log o con la grilla mostrando la segunda zona).

### M4-T2 Welcome y GNOME Online Accounts
- Docs: 11 §9 (`WelcomeDialog`, `GoaDialog`).
- Aceptación: checks; mirar Welcome con la instancia de depuración vacía (sin `oauth.json`): no cierra con Escape ni con el scrim; GOA no hace falta mirarlo con GNOME (la instancia de Xvfb no lo tiene): basta que compile y que el diálogo se dibuje montándolo desde un estado forzado durante la verificación y volviéndolo atrás antes del commit.

### M4-T3 Add calendar
- Docs: 11 §9 (`AddCalendarDialog`).
- Aceptación: checks; mirar: el "+" de Other calendars abre el diálogo; Add deshabilitado hasta llenar nombre y URL; Cancel cierra.

## Fase M5 — Limpieza

Objetivo: no queda nada de la réplica de Google ni de las hojas de transición.

### M5-T1 Borrado del pipeline viejo
- Docs: 11 §11, `docs/99` (2026-09-16, punto 5).
- Pasos: borrar `src/styles/measured.css`, `src/styles/legacy.css`, `src/styles/legacy-*.ts`, `scripts/gen-measured-css.mjs`, `scripts/gen-tokens.mjs`, `scripts/measure/`, `src/dev/`, `docs/design/tokens.json`, `docs/design/token-spec.json`, `docs/design/icons.txt`, `docs/design/fixture-events.md`, `src-tauri/resources/fonts/GoogleMaterialIcons-subset.woff2`, `.claude/skills/measure-component/`; `git mv docs/design/measurements docs/design/google` y escribir `docs/design/google/README.md` (qué es, fecha, que no rige nada); `src-tauri`: borrar `commands/dev.rs`, `dev_dump` de `commands/mod.rs` (registro y lista de `removeUnusedCommands`) y la ruta `/dev/measure` de `webhook/server.rs`; `.gitignore` sin la línea de `measurements`; `scripts/check-phase.sh` sin las ramas `4|7`; `check-tokens.mjs` sin la tolerancia a `legacy.css`; `src/main.tsx` sin `measured.css` ni el import de `dev/measure`; `eslint.config.js` sin `scripts/measure/dumpRegion.js`. Correr `cargo clippy -D warnings`, `cargo test` (`CARGO_BUILD_JOBS=2`) además de los checks. Después del build, `cargo clean` no hace falta (no cambió `Cargo.lock`).
- Aceptación: `bash scripts/check-m3.sh` pasa todo salvo la sección "release"; `du -sh` del repo (sin `node_modules` ni `target`) bajó y se reporta cuánto.

### M5-T2 Documentación, reglas y README
- Docs: `CLAUDE.md`, `.claude/rules/`, `README.md`, `docs/10-mantenimiento.md`.
- Pasos: `README.md`: sección de fidelidad reemplazada por dos líneas sobre `theme.json` y `gen-m3-tokens.mjs`; `docs/10` sección 1 (estado), sección 3 (quitar odiff, scrollbars medidas, desvíos de Google; agregar lo que WebKitGTK haga distinto con M3 si algo apareció), sección 6 reescrita ("Ya no se mide contra Google; para revisar la UI, sección 5"); `CLAUDE.md` y `.claude/rules/*.md` revisados para que no nombren archivos borrados; `docs/design/README` si hace falta. Sin tocar `docs/01` a `09` salvo su "Registro de cambios".
- Aceptación: `grep -rn "measure-component\|tokens.json\|measured.css\|scripts/measure\|report.mjs\|diff-layout" CLAUDE.md README.md .claude docs/10-mantenimiento.md docs/11-material3.md docs/12-plan-m3.md` solo devuelve menciones históricas explícitas ("se borró", "histórico"); checks.

## Fase M6 — Cierre

Ver sección 7.

## 7. Procedimiento de cierre de la transición

1. `bash scripts/check-m3.sh` pasa entero (incluye clippy, tests, lint, typecheck, vitest, tokens al día, borrados, versión y `.deb`). Orden práctico: primero `scripts/bump-version.sh 0.2.0`, luego el build, luego el gate.
2. Build: `TAURI_LINUX_AYATANA_APPINDICATOR=1 CARGO_BUILD_JOBS=2 npm run tauri build -- --bundles deb`, sin otra compilación en curso.
3. RAM: con el binario de release en el Xvfb privado y los directorios `XDG_*` de depuración (cuenta local con algunos eventos; no hace falta cuenta Google), vista semana, `bash scripts/measure-ram.sh`; fila `| M6 | <fecha> | ... |` en `docs/99`, tabla "Mediciones de RAM por fase", con la nota "Xvfb, llvmpipe, release, sin cuentas Google".
4. `docs/10-mantenimiento.md` sección 1: párrafo de la transición (fecha, versión 0.2.0, qué cambió, cifra de RAM, `.deb` en `src-tauri/target/release/bundle/deb/`, 0.1.6 como respaldo hasta que el usuario confirme).
5. `docs/99`: si durante la transición hubo un desvío respecto de `11` o `12`, su entrada ya tiene que estar; verificar con `git log` que cada tarea tiene commit.
6. Commit `M6: transition closed`.
7. Mensaje al usuario, en español, con: la ruta del `.deb` y los dos comandos que debe correr él (Quit desde la bandeja y `sudo apt install ./src-tauri/target/release/bundle/deb/<archivo>.deb`); qué va a ver distinto (una lista corta); la cifra de RAM; qué mirar primero (semana, popup, formulario, oscuro); y que 0.1.6 sigue instalable como vuelta atrás.

## 8. Cuándo detenerse y preguntar al usuario

Solo en estos casos; en todos los demás se decide y se anota en `docs/99` si contradice `11`:

- Un comando con `sudo` (instalar el `.deb`, paquetes del sistema).
- `@material/web` no funciona en WebKitGTK 2.50 por algo que 11 §2 no previó y no tiene solución local (documentar el error exacto antes de preguntar).
- Un criterio de aceptación requiere sus cuentas reales de Google (ninguno lo requiere: todo se verifica con el calendario local).
- Una decisión de diseño que el mockup y 11 §7 no cubren y cambia lo que el usuario ve de forma notable (por ejemplo, dónde va un control que hoy no existe). Lo demás se resuelve con el criterio de M3 y se anota.

## 9. Fuera de alcance

Todo lo de `docs/01` versión 2. Además: cambios de comportamiento (qué hace cada botón), cambios en el backend, cambios en el contrato IPC, nuevas vistas, `prefers-reduced-motion`, densidad configurable, tema por semilla elegible por el usuario. Si aparece la tentación, se anota como idea en `docs/10` sección 3 y se sigue.
