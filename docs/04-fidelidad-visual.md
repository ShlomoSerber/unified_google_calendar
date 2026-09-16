# 04 — Fidelidad visual pixel a pixel

Fuente: `docs/research/pixel-perfect.md`. Regla central: **ninguna medida se inventa ni se toma de memoria**. Toda medida sale de la web real de Google Calendar con el procedimiento de este documento y queda registrada en `docs/design/tokens.json` y `docs/design/measurements/`.

## 1. Qué significa "pixel perfect" en este proyecto

Para cada componente, en tema claro y oscuro, con la ventana a 1440×900 y DPR 1:

1. **Layout idéntico**: el volcado JSON de rectángulos y estilos computados de nuestra app coincide con el de Google en todas las propiedades de la lista de la sección 3, con tolerancia de 0.5 px en posiciones y tamaños.
2. **Píxeles casi idénticos**: la comparación de capturas con `odiff --antialiasing --threshold 0.1` da menos de 0.5% de píxeles distintos por componente. El rasterizado de texto entre Chrome y WebKitGTK nunca es bit a bit idéntico. Ese margen existe solo para el texto.

Un componente se da por terminado cuando cumple las dos condiciones y el resultado queda anotado en `docs/design/measurements/<componente>.md`.

## 2. Preparación, una sola vez

### 2.1 Perfil de Chrome para medir

Chrome 136 y posteriores ignoran `--remote-debugging-port` sobre el perfil por defecto. Se usa un perfil dedicado:

```bash
google-chrome --user-data-dir="$HOME/.chrome-measure" --window-size=1440,900 https://calendar.google.com
```

El usuario inicia sesión una vez con su cuenta principal en ese perfil. Ese perfil no se usa para nada más.

### 2.2 Estado de referencia en Google Calendar

Para que las capturas sean comparables, Google Calendar se configura así en el perfil de medición:

- Idioma inglés (Estados Unidos). Formato 24 h. Semana empieza el lunes.
- Zona horaria principal la del sistema. Zona secundaria "Mexico City" activada con etiqueta vacía.
- Densidad "Responsive to your screen". Color set "Modern".
- Solo calendarios de prueba visibles, con eventos de prueba creados según `docs/design/fixture-events.md`, para que el contenido de ambas capturas sea el mismo.
- Ventana a 1440×900 con el device toolbar de DevTools en modo Responsive, DPR 1. Zoom 100%. Escala de GNOME 100%.

### 2.3 Escala y fuentes del sistema

Ambos lados renderizan con fontconfig. Para acercar el rasterizado:

```bash
gsettings set org.gnome.desktop.interface font-hinting 'slight'
gsettings set org.gnome.desktop.interface font-antialiasing 'grayscale'
```

Esto se anota en `99-decisiones.md` si el usuario lo acepta. No es obligatorio para el criterio de layout, solo mejora el diff de píxeles.

## 3. Procedimiento de medición de un componente

1. En Chrome, abrir DevTools, seleccionar el nodo raíz del componente en Elements. Queda como `$0`.
2. Pegar en Console el snippet `scripts/measure/dumpRegion.js` completo. Ejecutar `dumpRegion($0)`. Descarga un JSON y lo copia al portapapeles.
3. Guardar el JSON como `docs/design/measurements/<componente>-<light|dark>.json`.
4. Con `$0` todavía seleccionado, Command Menu (Ctrl+Shift+P) → "Capture node screenshot". Guardar como `docs/design/measurements/<componente>-<light|dark>.png`.
5. Repetir en el otro tema. El tema se cambia en Settings → Appearance en Google Calendar.
6. Extraer de los JSON los valores nuevos y agregarlos a `docs/design/tokens.json` con el nombre del componente como prefijo. Nunca sobrescribir un token existente sin anotar el motivo.
7. Implementar el componente en la app usando solo tokens.
8. En la app en modo dev, abrir el inspector, seleccionar el mismo nodo raíz, ejecutar el mismo `dumpRegion($0)`. Guardar como `docs/design/measurements/<componente>-<tema>-app.json`.
9. Correr `node scripts/measure/diff-layout.mjs <google.json> <app.json>`. Debe devolver 0 diferencias. Si hay diferencias, corregir y repetir desde 8.
10. Capturar el nodo en la app y correr `npx odiff <google.png> <app.png> <diff.png> --antialiasing --threshold 0.1`. Anotar el porcentaje en `docs/design/measurements/<componente>.md`.

El snippet `dumpRegion` está en el informe de investigación y se copia tal cual a `scripts/measure/dumpRegion.js`. Registra por nodo: ruta, tag, clases, rol, aria-label, texto, rect relativo al raíz y los estilos computados que no son el default de un `div`.

## 4. Lista de componentes a medir, en orden

| Orden | Componente | Nodo raíz en Google | Estados |
|---|---|---|---|
| 1 | Tokens de tema | `document.documentElement` con el snippet de custom properties | light, dark |
| 2 | Barra superior | `header[role=banner]` o el ancestro del botón Today | normal |
| 3 | Botón Create y menú | el botón con texto Create | cerrado, abierto |
| 4 | Mini calendario | el `div` con role grid en la barra lateral | mes actual, mes con hoy fuera |
| 5 | Lista de calendarios | "My calendars" y "Other calendars" | expandido, hover en fila |
| 6 | Cabecera de semana | fila con los nombres de día y números | día de hoy resaltado |
| 7 | Fila de todo el día | | con 0, 1 y 2 chips |
| 8 | Grilla de horas | columna de horas más columnas de días | zona secundaria activa |
| 9 | Línea de hora actual | | |
| 10 | Chip de evento | eventos de 15, 30, 45, 60, 90 min y solapados de 2 y 3 | normal, hover, tentativo, rechazado, pasado |
| 11 | Vista día | | |
| 12 | Vista mes | celda con 0 a 5 eventos y "more" | hoy, otro mes |
| 13 | Vista agenda | | |
| 14 | Popup de detalle de evento | tras click en un chip | con Meet, con invitados, con recurrencia, evento simple |
| 15 | Quick create | tras click en la grilla | vacío, con título |
| 16 | Formulario completo | "More options" | |
| 17 | Selector de vista | desplegable Week | abierto |
| 18 | Diálogo de recurrencia personalizada | | |
| 19 | Diálogo "Edit recurring event" | this, following, all | |
| 20 | Scrollbars | | |
| 21 | Tooltip | el portal que aparece bajo un botón del header 540 ms después del puntero | visible |
| 22 | Animaciones | ver sección 10 | cada escenario de `scripts/measure/animations.mjs` |
| 23 | Vista año | `[role=main]` en `/r/year/2026/9/14` | normal |
| 24 | Snackbar | la barra que sube desde el borde inferior al guardar o borrar un evento | "Saving..." |

Componentes 1 a 10 cierran la fase de UI base. 11 a 20 cierran la fase de UI completa. Ver `08-plan-de-implementacion.md`. 21 y 22 son de la ronda de animaciones (2026-09-15, mantenimiento).

## 5. Archivo de tokens

`docs/design/tokens.json` es la única fuente de verdad de colores, tamaños y tipografía. `scripts/gen-tokens.mjs` lo convierte en `src/styles/tokens.css` con custom properties. Estructura:

```json
{
  "meta": { "measured_at": "2026-09-20", "chrome": "140.0", "viewport": [1440, 900], "dpr": 1 },
  "font": {
    "family_display": "\"Google Sans Flex\", \"Google Sans Text\", \"Google Sans\", Roboto, Arial, sans-serif",
    "family_body": "...",
    "topbar_title": { "size": "22px", "weight": "400", "line_height": "28px", "letter_spacing": "normal" }
  },
  "color": {
    "light": { "surface": "#ffffff", "on_surface": "#3c4043", "outline": "#dadce0", "primary": "#1a73e8" },
    "dark":  { "surface": "#1e1f20", "on_surface": "#e3e3e3", "outline": "#444746", "primary": "#a8c7fa" }
  },
  "layout": {
    "topbar_height": "64px", "sidebar_width": "256px", "hour_row_height": "48px", "gutter_width": "56px"
  },
  "component": {
    "event_chip": { "radius": "4px", "padding": "2px 8px", "font_size": "12px", "line_height": "16px" }
  }
}
```

Los valores del ejemplo son ilustrativos y **no válidos** hasta que se midan. El archivo inicial se entrega con todos los valores en `null` y `scripts/check-tokens.mjs` falla si algún token usado en CSS sigue en `null`.

Los tokens `--gm3-*` exportados de Google se guardan crudos en `docs/design/measurements/gm3-light.json` y `gm3-dark.json`. Solo los que se usan pasan a `tokens.json`, con nombre propio.

## 6. Fuentes e íconos

| Familia | Licencia | Cómo se usa |
|---|---|---|
| Google Sans Flex | OFL | Empaquetada en `src-tauri/resources/fonts/`, archivo variable de `google/fonts/ofl/googlesansflex`. Declarada con `@font-face` bajo el nombre `"Google Sans Flex"`. |
| Roboto | OFL | Empaquetada, variable `Roboto[wdth,wght].ttf`. |
| Material Symbols Outlined | Apache 2.0 | Subset con `pyftsubset` de los íconos usados, con `--layout-features='liga'`. Lista en `docs/design/icons.txt`. |
| Google Sans Text, Google Sans | Propietarias | No se redistribuyen. Si la medición muestra que son la fuente efectiva de algún componente, se cargan en runtime desde `fonts.googleapis.com` con `display=swap` y fallback a Google Sans Flex. La CSP permite `style-src https://fonts.googleapis.com` y `font-src https://fonts.gstatic.com` solo por esto. Es una zona gris legal aceptable para una app personal no distribuida. |

La medición del componente 1 incluye el `fontFamily` computado de cada rol de texto y `document.fonts` para saber qué familia se resolvió de verdad.

`font-synthesis: none` en `base.css`. WebKitGTK 2.50 tiene un bug abierto que engrosa el texto respecto de Chrome. Se usan pesos reales de la fuente variable y se verifica el `fontWeight` computado con `dumpRegion`.

## 7. Paleta de eventos

`colors.get` de la API devuelve la paleta clásica. La UI de Google usa la moderna. Mapeo por `colorId`, verificado en `docs/research/google-calendar-api.md`:

| id | Nombre | Hex UI |
|---|---|---|
| 1 | Lavender | `#7986cb` |
| 2 | Sage | `#33b679` |
| 3 | Grape | `#8e24aa` |
| 4 | Flamingo | `#e67c73` |
| 5 | Banana | `#f6bf26` |
| 6 | Tangerine | `#f4511e` |
| 7 | Peacock | `#039be5` |
| 8 | Graphite | `#616161` |
| 9 | Blueberry | `#3f51b5` |
| 10 | Basil | `#0b8043` |
| 11 | Tomato | `#d50000` |

El color de texto sobre cada chip, y los tonos de hover, tentativo y rechazado, se miden en el componente 10 y van a `tokens.json`. Banana usa texto oscuro en Google. No asumirlo: medirlo.

## 8. Diferencias aceptadas entre WebKitGTK y Chrome

- Rasterizado de texto. Tolerado por el umbral de píxeles.
- Scrollbars nativas. Se estilizan con `::-webkit-scrollbar` para igualar ancho y color medidos.
- Anchor positioning no está en WebKitGTK 2.50. Los popups se posicionan con JS, replicando las reglas medidas: a la derecha del chip si entra, si no a la izquierda, y ajustado al viewport.
- `field-sizing` no existe en 2.50. Los inputs que crecen con el texto usan un span espejo.

Todo lo que se sospeche distinto se prueba primero con `CSS.supports()` dentro del webview, no en Chrome.

## 9. Logo y nombre

No se usa el logo de Google Calendar ni ninguna variante. El ícono de la app es propio. El nombre "Unified Google Calendar" se usa en la ventana y el `.desktop`. En la barra superior el hueco del logo y del título queda vacío con su ancho medido, a pedido del usuario (`docs/99-decisiones.md`, 2026-09-15); el ícono de la bandeja y de la ventana muestra el día del mes (`scripts/gen-day-icons.mjs`).

## 10. Movimiento

Las transiciones se miden con el mismo rigor que el layout. Nada se anima con valores inventados: cada duración, easing, desplazamiento y color de estado sale de `docs/design/measurements/animations-<tema>.json` y vive en `docs/design/tokens.json`, `component.motion`. El resumen legible está en `docs/design/measurements/animations.md`.

Procedimiento, sin el usuario:

```bash
node scripts/measure/animations.mjs [escenario ...]   # todos por defecto
node scripts/measure/capture.mjs tooltip               # componente 21: el tooltip visible
node scripts/measure/extract-tokens.mjs && node scripts/gen-tokens.mjs && node scripts/gen-measured-css.mjs
```

`animations.mjs` pone calendar.google.com en un estado conocido, inyecta `scripts/measure/animRecorder.js`, dispara la acción (click real por CDP, o hover) y guarda cada Web Animation que corre la página (`document.getAnimations()` muestreado por `requestAnimationFrame`: destino, keyframes, duración, easing) más la caja, opacidad y transform de los elementos vigilados cuadro por cuadro. Los escenarios `hover_*` y `press_*` guardan además la diferencia de estilos computados entre reposo y hover, en claro y oscuro. Todo es de solo lectura sobre la cuenta: los chips se abren y se cierran, la creación rápida y el formulario se descartan sin guardar, la casilla de un calendario se apaga y se vuelve a encender, y el script avisa si alguna quedó apagada.

Los tokens `component.motion` se leen desde CSS como `--motion-*` (`src/styles/motion.css`) y desde JS como `MOTION` (`src/styles/motion.ts`, generado). Las piezas del frontend:

- `src/app/ViewStage.tsx`: deslizamiento de la grilla al navegar y cross-fade al cambiar de vista.
- `src/lib/motion.ts`: `usePresence` (deja montado un diálogo mientras corre su animación de salida), el ripple Material de cada botón y el tooltip.
- `src/app/Tooltip.tsx`: el tooltip medido (componente 21), bajo el botón y centrado.
- `.ugc-state` en los spans de estado que ya existían en el DOM medido: tinte de hover en `::before`, ripple como tinte pulsado.

Fuera de alcance: anillos de foco por teclado (versión 2) y la barra de progreso de la página de Settings.

## Registro de cambios

- 2026-09-14 — Medición automatizada por CDP, propiedades extra de `dumpRegion`, tokens por nodo, ajustes de la cuenta de referencia y paleta clara/oscura medida: ver `docs/99-decisiones.md` (F4-T1, F4-T3, F4-T4). Criterio `odiff < 0.5 %` no alcanzado por rasterizado de texto: ver la entrada "Criterio de aceptación visual" en `docs/99-decisiones.md`.
- 2026-09-14 — Componentes 11 a 20 medidos y comparados; regla de anclaje del popup y del quick create sondeada; scrollbars superpuestas: ver `docs/99-decisiones.md` (Fase 7) y `docs/design/measurements/<componente>.md`.
- 2026-09-14 — `src/styles/measured.css` se genera con los valores medidos en literal (no `var()`), ver `docs/99-decisiones.md` (Fase 8).
- 2026-09-15 — Ronda de animaciones: sección 10, `scripts/measure/animations.mjs`, tokens `component.motion`, componente 21 (tooltip). Logo y título del header retirados; ícono con el día del mes. Ver `docs/99-decisiones.md` (2026-09-15).
- 2026-09-16 — **Este documento es histórico.** El usuario decidió abandonar la réplica pixel a pixel y pasar la UI a Material 3 (`docs/99-decisiones.md`, entrada 2026-09-16). La fuente de verdad visual es `docs/11-material3.md`; el procedimiento de medición, `scripts/measure/`, `tokens.json`, `measured.css` y el skill `measure-component` se borran en la fase M5 de `docs/12-plan-m3.md`. Las mediciones quedan como referencia en `docs/design/google/`. Nada de lo que dice este documento rige la implementación desde esa fecha.
