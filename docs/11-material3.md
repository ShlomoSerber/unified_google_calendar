# 11 — Sistema de diseño Material 3

Vigente desde el 2026-09-16 (`docs/99-decisiones.md`, entrada "Transición a Material 3 total"). Reemplaza a `docs/04-fidelidad-visual.md` como fuente de verdad visual. Quien implementa no cambia nada de este documento sin registrarlo en `99`.

Fuentes de cada afirmación sobre librerías: `material-components/material-web` (repositorio, tokens `v0.192`, Apache 2.0), `material-foundation/material-color-utilities` (Apache 2.0), `react.dev` (referencia de custom elements en React 19) y las páginas de especificación de `m3.material.io` citadas en `docs/design/m3/theme.json`.

## 1. Objetivo y alcance

- La app deja de imitar calendar.google.com. Se ve como una app Material 3 ("Material You"): fondo `surface-container`, superficie de calendario redondeada, controles oficiales con ripple, estados y anillos de foco, chips tonales con el color del calendario en el borde. Referencia visual: `docs/design/m3/mockup.html`, pestaña "3 · M3 total", en claro y oscuro.
- Cambia todo `src/` salvo lo que no dibuja: `src/ipc/`, `src/types/`, `src/state/useViewData.ts`, `src/lib/dates.ts`, `src/lib/notice.ts`, `src/views/week/layout.ts` y las funciones listadas como "lógica a conservar" en la sección 9. El backend Rust no cambia, salvo borrar el endpoint de medición de desarrollo (sección 11).
- El contrato IPC de `docs/02` sección 5 no cambia. Los requisitos funcionales de `docs/01` no cambian; R8 (fidelidad pixel a pixel) queda derogado por `99`.
- El tema claro u oscuro sigue al sistema (R4.11) por `prefers-color-scheme`, como hasta ahora.

## 2. Dependencias y compatibilidad

| Paquete | Versión | Tipo | Para qué |
|---|---|---|---|
| `@material/web` | 2.5.0 fijada | runtime | Los componentes `<md-*>`. Se importa **componente por componente** (`@material/web/button/filled-button.js`); nunca `@material/web/all.js`, que es solo para prototipos. |
| `lit` | la que resuelva `@material/web` (`^3`) | runtime, transitiva | Base de los web components. No se importa directamente en la app. |
| `@material/material-color-utilities` | 0.3.0 fijada | desarrollo | Solo la usa `scripts/gen-m3-tokens.mjs`. La 0.4.0 publica imports ESM sin extensión y Node no la carga; verificado el 2026-09-16. |
| `react`, `react-dom` | 19.3.0 fijadas | runtime | React 19 pasa las props a los custom elements como propiedades cuando el elemento las define y escucha sus eventos desde JSX (sección 5). Sin esto cada componente M3 necesitaría un envoltorio con `useRef` y `addEventListener`. |
| `@types/react`, `@types/react-dom` | 19.x | desarrollo | Tipos de React 19. `@testing-library/react` 16.3 ya soporta React 19. |

Nada más entra. Sigue vigente la regla de `CLAUDE.md`: ninguna otra dependencia sin entrada en `99`.

WebKitGTK 2.50 (Ubuntu 25.04) frente a lo que `@material/web` y Lit necesitan:

| Capacidad | Estado en 2.50 | Consecuencia |
|---|---|---|
| Custom elements, shadow DOM, `adoptedStyleSheets` | soportado | nada |
| `ElementInternals` (`attachInternals`, formularios) | soportado desde Safari 16.4 | `md-checkbox`, `md-switch`, `md-select`, `md-text-field` funcionan sin polyfill |
| `<dialog>` con `showModal()` | soportado | `md-dialog` usa el top layer nativo: los diálogos apilados (recurrencia sobre el formulario) se ordenan solos |
| Popover API (`popover` attribute) | no en 2.50 | `md-menu` se usa con `positioning="fixed"` o `"absolute"`, nunca `"popover"` |
| `color-mix()` | soportado (`docs/02` sección 1) | las sombras de elevación y el fallback de colores de chip lo usan |
| Anchor positioning | no | los popups siguen posicionados con JS (`popupPosition`, sección 9) |

Vite mantiene `build.target: ['safari15']`; esbuild baja la sintaxis de Lit 3 (ES2021) sin problema. Si una duda de soporte aparece, se prueba con `CSS.supports()` o `'attachInternals' in HTMLElement.prototype` dentro del webview, no en Chrome.

## 3. Tokens: `docs/design/m3/theme.json`

Única fuente de verdad de todo valor visual. `node scripts/gen-m3-tokens.mjs` lo convierte en:

| Archivo generado | Contenido |
|---|---|
| `src/styles/tokens.css` | `:root` con `--md-ref-typeface-*`, `--md-icon-font`, `--md-sys-color-*` (claro), `--md-sys-typescale-*`, `--md-sys-shape-corner-*`, `--md-sys-motion-*`, `--md-sys-elevation-level<n>`, `--md-sys-state-*`, `--ugc-space-<n>`, `--ugc-layout-*`, `--ugc-motion-*`; y `@media (prefers-color-scheme: dark)` con los `--md-sys-color-*` oscuros. |
| `src/styles/typescale.css` | Una clase por rol: `.md-typescale-body-medium`, etc. (familia, tamaño, interlineado, peso, tracking). |
| `src/styles/layout.ts` | `LAYOUT` con los `layout.*` como strings y `layoutNumber()`. Lo usan la grilla (alturas de hora, chips) y el posicionado de popups. |
| `src/styles/motion.ts` | `MOTION` con los `app_motion.*` resueltos a valores literales (para `setTimeout` y WAAPI). |
| `src/styles/palette.ts` | `COLOR_MAP` (hex guardado → roles de color de chip por tema), `EVENT_COLORS`, `EVENT_COLOR_NAMES`, `LOCAL_CALENDAR_DEFAULT`. |

Grupos de `theme.json` y de dónde sale cada valor:

- `color`: **generado**. `seed` `#0b57d0`, variante `vibrant`, contraste 0. El generador construye `SchemeVibrant` claro y oscuro y emite todos los roles de `MaterialDynamicColors` (`surface`, `surface-container-*`, `on-surface`, `primary`, `primary-container`, `outline`, `outline-variant`, `inverse-surface`, `error`, ...). Nadie escribe un color de tema a mano. Para cambiar el tema se cambia la semilla o la variante y se regenera.
- `color.custom_colors`: los hex que el backend guarda para calendarios (paleta clásica de la API) y eventos (`colorId` 1 a 11). El generador deriva de cada uno un color personalizado M3 (sección 6).
- `typeface`, `typescale`, `shape`, `motion`, `elevation`, `state`: copiados de los tokens `v0.192` de `material-web` (URL en cada grupo). Son constantes del sistema; no se editan.
- `space`: la grilla de 4 px (`--ugc-space-1` = 4 px ... `--ugc-space-16` = 64 px). Todo margen, padding y gap de la app usa uno de estos.
- `layout`: los tamaños propios de la app (alturas de fila, anchos de cajón y gutter, tamaños de chip, popup, diálogos, snackbar, tooltip). Cada uno es múltiplo de 4 px o una referencia a un token M3, decidido a partir del mockup. Se puede agregar un token nuevo durante la transición si cumple esa regla y lleva una `<nombre>_note` con el motivo.
- `app_motion`: las transiciones propias de la app (deslizamiento de la grilla, popup, snackbar, tooltip, cajón) expresadas como nombres de tokens `--md-sys-motion-*`; el generador los resuelve.

Reglas:

1. Ninguna hoja de estilos escrita a mano contiene un píxel, un color, un tamaño de fuente, un peso, un radio ni una sombra literal. Solo `var(--md-sys-*)`, `var(--ugc-*)`, `var(--data-*)` (sección 8), `0`, `100%`, `auto`, `1fr` y `calc()` entre tokens. `node scripts/check-tokens.mjs` lo verifica y falla si un `var()` no está definido en `tokens.css`.
2. Los tokens de componente de `@material/web` (`--md-checkbox-*`, `--md-dialog-*`, `--md-fab-*`, `--md-outlined-text-field-*`, ...) se pueden fijar en la hoja del componente que los usa, siempre con un `var()` como valor. Ejemplo: `--md-checkbox-selected-container-color: var(--data-calendar-color)`.
3. Después de editar `theme.json`: `node scripts/gen-m3-tokens.mjs && node scripts/check-tokens.mjs` y se commitea el JSON junto con los generados. `scripts/check-m3.sh` regenera a un directorio temporal y compara: los generados commiteados tienen que coincidir.

## 4. Tipografía e íconos

| Familia | Rol | Archivo | Licencia |
|---|---|---|---|
| Roboto (variable, regular e itálica) | `--md-ref-typeface-plain`: body, label, title-medium, title-small | `src-tauri/resources/fonts/Roboto-VF.ttf`, `Roboto-Italic-VF.ttf` (ya empaquetadas) | OFL |
| Google Sans Flex (variable) | `--md-ref-typeface-brand`: display, headline, title-large | `GoogleSansFlex-VF.ttf` (ya empaquetada) | OFL |
| Material Symbols Outlined (variable: FILL, wght, GRAD, opsz) | `--md-icon-font`, lo que dibuja `<md-icon>` | `MaterialSymbolsOutlined-subset.woff2`, **nuevo**, subconjunto de `docs/design/m3/icons.txt` | Apache 2.0 |

- `src/styles/fonts.css` declara las tres con `@font-face` y `font-display: block`. `font-synthesis: none` sigue en `base.css`.
- Se retiran: el `<link>` a `fonts.googleapis.com` de `index.html`, las familias `Google Sans Text` y `Google Sans` de todo `src/`, `GoogleMaterialIcons-subset.woff2`, `docs/design/icons.txt`, y de la CSP de `tauri.conf.json` las entradas `https://fonts.googleapis.com` y `https://fonts.gstatic.com`. La app no carga nada de la red para dibujarse.
- Íconos: solo `<md-icon>nombre</md-icon>` con nombres de `docs/design/m3/icons.txt` (ligaduras). Los SVG con `d="..."` copiados de Google desaparecen. `md-icon` mide `--md-icon-size` (24 px por defecto; `--ugc-layout-icon-size-small` = 20 px en filas densas).
- Subconjunto (una vez, y cada vez que `icons.txt` cambia):

```bash
# fuente: google/material-design-icons, carpeta variablefont/
curl -L -o /tmp/claude-1000/MaterialSymbolsOutlined.ttf \
  'https://github.com/google/material-design-icons/raw/master/variablefont/MaterialSymbolsOutlined%5BFILL%2CGRAD%2Copsz%2Cwght%5D.ttf'
python3 -m fontTools.subset /tmp/claude-1000/MaterialSymbolsOutlined.ttf \
  --glyphs="$(paste -sd, docs/design/m3/icons.txt)" \
  --text="$(tr -d '\n' < docs/design/m3/icons.txt | tr -s '_a-z' | fold -w1 | sort -u | paste -sd '')" \
  --layout-features='rlig,liga' --no-layout-closure --flavor=woff2 \
  --output-file=src-tauri/resources/fonts/MaterialSymbolsOutlined-subset.woff2
```

  `pyftsubset` conserva los ejes de variación. Si un ícono nuevo no aparece, falta en `icons.txt` o en el subconjunto; nunca se vuelve a la fuente completa (unos 3 MB).
- `src-tauri/resources/fonts/README.md` se actualiza con la fila nueva y sin la de Material Icons.

## 5. `@material/web` dentro de React 19

- `src/m3/register.ts`: un import por componente usado, y nada más. Lista de la transición (se amplía solo si un componente nuevo entra en la sección 9):

```ts
import '@material/web/button/filled-button.js';
import '@material/web/button/text-button.js';
import '@material/web/checkbox/checkbox.js';
import '@material/web/chips/chip-set.js';
import '@material/web/chips/filter-chip.js';
import '@material/web/dialog/dialog.js';
import '@material/web/divider/divider.js';
import '@material/web/focus/md-focus-ring.js';
import '@material/web/icon/icon.js';
import '@material/web/iconbutton/icon-button.js';
import '@material/web/list/list.js';
import '@material/web/list/list-item.js';
import '@material/web/menu/menu.js';
import '@material/web/menu/menu-item.js';
import '@material/web/progress/circular-progress.js';
import '@material/web/radio/radio.js';
import '@material/web/ripple/ripple.js';
import '@material/web/select/outlined-select.js';
import '@material/web/select/select-option.js';
import '@material/web/switch/switch.js';
import '@material/web/textfield/outlined-text-field.js';
```

  `src/main.tsx` lo importa antes que `App`.
- `src/m3/jsx.d.ts`: declara cada etiqueta en `React.JSX.IntrinsicElements` con el tipo de su clase (`MdFilledButton`, `MdDialog`, ... exportadas por los mismos módulos): atributos HTML, `slot`, `ref` tipado a la clase, las propiedades del elemento en `Partial<...>` y los eventos como `on<evento>` en minúsculas. Patrón:

```ts
type M3<T extends HTMLElement, E extends string = never> = React.DetailedHTMLProps<React.HTMLAttributes<T>, T> &
  Partial<Omit<T, keyof HTMLElement | 'children'>> & { slot?: string } & { [K in E as `on${K}`]?: (e: Event) => void };
declare module 'react' {
  namespace JSX {
    interface IntrinsicElements {
      'md-dialog': M3<MdDialog, 'open' | 'opened' | 'close' | 'closed' | 'cancel'>;
      'md-checkbox': M3<MdCheckbox, 'change' | 'input'>;
      // ...
    }
  }
}
```

- Reglas de uso en JSX (fuente: react.dev, "Custom HTML elements"):
  - Propiedades con valor no string (`checked`, `selected`, `disabled`, `open`, `value` numérico, `anchorElement`) se pasan tal cual: React 19 las asigna como propiedad porque el elemento las define.
  - Atributos string (`label`, `variant`, `type`, `positioning`, `anchor`, `touch-target`, `supporting-text`, `has-icon`, `trailing-icon`) se pasan como atributo con el nombre del atributo, en kebab-case cuando así lo documenta `material-web`.
  - Eventos propios del elemento se escuchan con `on<evento>` **en minúsculas y con la misma grafía que el evento**: `onchange`, `oninput`, `onclose`, `onclosed`, `onopened`, `oncancel`, `onclick`. En los elementos `<md-*>` no se usa `onClick` ni `onChange` de React.
  - Abrir y cerrar `md-dialog` y `md-menu` por propiedad: `ref.current.show()` / `.close()` o `open={...}`; el cierre por Escape o scrim llega como evento `closed` (`cancel` para vetarlo con `preventDefault`, como hacen Welcome y GOA).
  - `md-menu` con `positioning="fixed"` y `anchor="<id del botón>"` cuando botón y menú comparten padre; con `anchorElement={ref.current}` cuando no.
  - Cada `md-icon-button` lleva `aria-label`. Cada campo lleva `label`.
- Tests: jsdom no implementa `ElementInternals`, así que `vitest.setup.ts` hace `vi.mock('./src/m3/register', () => ({}))` y los `<md-*>` quedan como elementos inertes sin shadow DOM. Los tests de componentes consultan roles y `aria-label` de los nodos propios de la app y no dependen del interior de un `<md-*>`. La lógica pura (fechas, layout de chips, `parseRule`/`buildRule`, colores) se prueba sin DOM.

## 6. Colores de calendario y de evento

- Cada calendario o evento llega del backend con un hex (`color_bg`). `COLOR_MAP` lo traduce a tres roles por tema: `color` (borde del chip y punto del calendario), `container` (fondo del chip), `onContainer` (texto del chip). Los deriva el generador con una paleta tonal del propio matiz y croma del color (tonos 40/90/10 en claro, 80/30/90 en oscuro), el método de "custom colors" de M3.
- `src/lib/colors.ts` expone `chipColors(hex, theme): ChipColors`. Si el hex no está en el mapa (un color que Google agregue), devuelve `color: hex`, `container: color-mix(in srgb, <hex> 24%, var(--md-sys-color-surface))` y `onContainer: var(--md-sys-color-on-surface)`. `useTheme()` sigue leyendo `prefers-color-scheme`.
- Los componentes inyectan esos valores como `--data-chip-color`, `--data-chip-container`, `--data-chip-on-container` y `--data-calendar-color` en `style` y la hoja los consume con `var(--data-*)`. Son datos, no tokens: es la única excepción a la regla 1 de la sección 3.
- `md-checkbox` de la lista de calendarios toma el color del calendario en `--md-checkbox-selected-container-color`, `-selected-hover-container-color`, `-selected-pressed-container-color`, `-selected-focus-container-color`, `-outline-color` y blanco (`--md-sys-color-on-primary`) en `-selected-icon-color`.

## 7. Estructura visual de la app

Roles y tamaños por pieza. Todo lo que no figura aquí toma el valor por defecto del componente M3.

| Pieza | Superficie y forma | Tipografía | Tamaños (`--ugc-layout-*`) |
|---|---|---|---|
| Fondo de la ventana (`.app`) | `surface-container` | body-medium, `on-surface` | |
| Barra superior | `surface-container`, sin borde ni sombra | título del rango: title-large (brand) | `topbar_height` 64, padding `space-2` |
| Cajón | `surface-container`, sin borde | | `sidebar_width` 300, padding `sidebar_padding_x` 16, gap `space-4`; se desliza con `--ugc-motion-drawer-*` |
| Botón Create | `md-filled-button` con `md-icon` `add` en `slot="icon"` y texto "Create event", alineado a la izquierda del cajón (2026-09-17: antes un `md-fab` extendido) | label-large | |
| Mini calendario | grilla `MonthGrid` de 7 columnas; días como botones circulares con `md-ripple` y `md-focus-ring`; hoy: `primary`/`on-primary`; semana visible: `primary-container`/`on-primary-container`; otro mes: `on-surface-variant` | label-medium (días y cabecera), title-small (mes) | `mini_cal_cell` 32, `mini_cal_header_height` 40 |
| Lista de calendarios | filas pill (`corner-full`) de 40 px con `md-ripple`; hover con capa de estado; cabecera de sección label-medium `on-surface-variant` con `md-icon` `expand_more`/`expand_less` | body-medium | `calendar_row_height` 40 |
| Superficie principal (`.app-main`) | `surface`, esquina superior izquierda `main_radius` 24, sin sombra | | |
| Cabecera de días | nombre del día label-medium `on-surface-variant`; número como `DayNumber` `day` (40 px) title-large (brand); hoy: círculo `primary`/`on-primary` y nombre `primary` | | `day_header_height` 72, `day_number_size` 40 |
| Gutter de horas | label-small `on-surface-variant`, una o dos columnas | | `gutter_width` 72 / `gutter_width_two_zones` 104 |
| Líneas de hora y columnas | `outline-variant`, 1 px | | `hour_row_height` 48 |
| Línea de ahora | `error`, 1 px, punto de 12 px, solo en la columna de hoy | | `now_line_height`, `now_dot_size` |
| Chip de evento | fondo `container` del calendario, texto `onContainer`, borde izquierdo de 3 px `color`, `corner-small`; `md-ripple` para hover y presión; título label-medium y hora body-small; con 30 min o menos solo el título | | `chip_*` |
| Chip de todo el día | igual, 24 px de alto, texto label-medium | | `allday_chip_height` |
| Vista mes | celdas con borde `outline-variant`; número del día como `DayNumber` `month` (pill de 24 px) label-large (hoy: `primary`/`on-primary`); chips de todo el día tonales de 20 px; eventos con hora como punto de 8 px + texto label-medium; "N more" como botón de texto label-medium `primary` | | `month_cell_header_height` 28, `month_chip_height` 20, `month_dot_size` 8 |
| Vista agenda | grupos por día: fecha como `DayNumber` `day` (40 px) title-medium (hoy `primary`), filas de 48 px con punto de 12 px del color del calendario, título body-large, hora body-medium `on-surface-variant`, `md-ripple` | | `agenda_row_height` 48, `agenda_dot_size` 12 |
| Vista año | doce tarjetas `surface-container-low` `corner-large` con el nombre del mes title-medium y una `MonthGrid` compacta de celdas de 28 px | | `year_month_min_width` 240, `year_cell_size` 28 |
| Popup de evento | `surface-container-high`, `corner-extra-large` 28, `elevation-level3`, 400 px; acciones `md-icon-button` arriba a la derecha; título headline-small (brand); filas con `md-icon` de 20 px `on-surface-variant` + texto body-medium; las acciones externas (Meet, teléfono, Open in Maps) como `md-text-button` con ícono bajo su texto | | `popup_width`, `popup_radius`, `popup_gap` |
| Formulario de evento | `md-dialog` de 640 px con `md-outlined-text-field`, `md-outlined-select`, `md-switch`, `md-text-button` con ícono para "Add Google Meet" y "Add notification"; acciones `md-text-button` Cancel y `md-filled-button` Save | | `form_width` |
| Selector de fecha | `md-outlined-text-field` de solo lectura con `md-icon-button` `calendar_today` al final; popover `surface-container-high` `corner-large` `elevation-level3` con `MonthGrid` | | |
| Recurrencia, alcance, Settings, Welcome, GOA, Add calendar | `md-dialog` de 560 px (Settings 640); radios `md-radio`; días de la semana `md-filter-chip` en `md-chip-set`; listas `md-list`/`md-list-item` | headline del diálogo: headline-small | `dialog_width`, `form_width` |
| Snackbar | `inverse-surface`, `corner-extra-small`, `elevation-level3`, texto body-medium `inverse-on-surface`, cierre `md-icon-button` con `--md-icon-button-icon-color: var(--md-sys-color-inverse-on-surface)`; centrado abajo | | `snackbar_*` |
| Tooltip | plain tooltip: `inverse-surface`, `corner-extra-small`, body-small `inverse-on-surface`, padding horizontal 8, alto mínimo 24; aparece bajo el control tras `tooltip_delay` | | `tooltip_*` |

Densidad: la app se usa en escritorio, así que todos los controles quedan con la densidad por defecto de M3 (objetivos táctiles de 48 px, `touch-target="wrapper"` en checkboxes y radios). En la barra superior y en las cabeceras de la grilla se admite `--md-icon-button-*` a 40 px porque conviven con texto de una línea.

## 8. Reglas de CSS y de componentes

- Un componente por archivo, PascalCase, con su `.css` al lado; clases con el prefijo del componente en kebab-case (`topbar-`, `week-`, `popup-`). No hay clases numeradas ni nodos vacíos: cada elemento del DOM tiene contenido o una función de layout.
- Solo `var()` de las familias `--md-sys-*`, `--md-ref-*`, `--ugc-*`, `--md-<componente>-*` (tokens de componente M3) y `--data-*` (sección 6). `node scripts/check-tokens.mjs` rechaza literales y variables no definidas; la excepción son los archivos generados y `fonts.css`.
- Tipografía por clase (`.md-typescale-<rol>`) o, si el elemento necesita solo un atributo, por token (`font-family: var(--md-sys-typescale-label-medium-font)`).
- Espaciado solo con `--ugc-space-<n>`. Radios solo con `--md-sys-shape-corner-*` (o un `--ugc-layout-*_radius` que apunte a uno). Sombras solo con `--md-sys-elevation-level<n>`.
- Estados de hover, foco y presión: en los `<md-*>` vienen incluidos. En los interactivos propios (días del mini calendario, números de día, chips, filas de calendario, filas de agenda, "N more") se ponen `<md-ripple>` y, cuando el elemento recibe foco por teclado, `<md-focus-ring>` como hijos de un contenedor `position: relative`. No se dibujan capas de estado a mano.
- Tres botones y nada más (`docs/99`, 2026-09-17; fuente m3.material.io/components/buttons/guidelines): `md-filled-button` es el primario, uno por superficie ("Create event" en el cajón; Save, OK, Done, Yes, Add, continue en la fila de acciones de cada diálogo); `md-text-button` es el secundario para todo lo demás con texto (Today, menú de vista, Cancel, Close, No, Sync now, Test, Add Google account, Add iCal, Turn off their Calendar, Sign in again, Remove, Add notification, Add Google Meet, Join with Google Meet, Join by phone, More phone numbers, Open in Maps), con `md-icon` en `slot="icon"` cuando la acción tiene ícono; `md-icon-button` para acciones de solo ícono con `aria-label`. `md-fab`, `md-outlined-button`, `md-filled-tonal-button`, `md-elevated-button` y `md-assist-chip` no se usan ni se registran; `md-filter-chip` sigue solo para la selección de días de la semana en la recurrencia, que es selección y no acción. Un `md-text-button` que encabeza una fila bajo campos de texto se alinea al borde de los campos con `margin-left: calc(-1 * var(--ugc-space-3))`, no con su propio padding. El número de día es siempre `DayNumber` (`src/components/DayNumber.tsx`, `data-size` `day`/`mini`/`year`/`month`); ningún otro componente dibuja un círculo de día.
- Accesibilidad mínima: cada `md-icon-button` con `aria-label`, cada grilla con `role="grid"`, cada chip con `role="button"` y su descripción (`chipDescription`), el popup con `role="dialog"` y `aria-labelledby`.
- Prohibido: valores literales, `installRipples`, `data-tooltip`, spans `.ugc-state`, DOM oculto para lectores de pantalla que duplique texto visible (se usa `aria-label`), SVG con paths de Google.
- Sin nuevas funciones: nada de teclado más allá de lo que los componentes M3 traen (Escape cierra diálogos y menús, Tab y flechas dentro de menús y radios), sin arrastrar, sin buscar, sin Tasks (`docs/01`, versión 2).

## 9. Mapeo componente a componente

Lo que cada archivo actual pasa a ser. "Conservar" es la lógica que no cambia y se copia tal cual (nombres de función del código actual). Referencia: inventario del 2026-09-16 hecho sobre `src/`.

| Archivo | Nueva implementación | Conservar | Se borra |
|---|---|---|---|
| `src/app/App.tsx` | Mismo esqueleto: `.app` (`surface-container`), barra, cajón (`.app-drawer` con ancho animado por `--ugc-motion-drawer-*`), `.app-main` (`surface`, radio 24). Ya no monta `QuickCreate` ni `ViewSelector`. Los diálogos se montan cuando su `kind` está activo y cada uno es un `md-dialog` que se abre solo al montarse. | `rangeStart`, `dialogOpen`, `dialogExitMs` (solo para el popup y el snackbar), el `useEffect` de arranque con las tres suscripciones, el despacho de vistas. | `installRipples()`; los `motion-layer motion-page` (los `md-dialog` animan solos). |
| `src/app/TopBar.tsx` | `md-icon-button` `menu` (cajón), título title-large, `md-icon-button` `chevron_left`/`chevron_right`, `md-text-button` "Today", botón de vista `md-text-button` con `md-icon` `arrow_drop_down` en `slot="icon"` y `trailing-icon`, y su `md-menu` (`positioning="fixed"`, cinco `md-menu-item`) en el mismo `TopBar`; a la derecha `md-icon-button` `help` (README) y `settings`. Tooltips con el componente `Tooltip` (sección 7). | `rangeTitle` con los casos de semana entre meses y años, `VIEW_LABEL`/`VIEW_UNIT`, `todayLabel`, `openHelp`. | La cadena `topbar-nXX`, los `topbar-view-ghost`, los SVG inline, `data-tooltip`. |
| `src/app/ViewSelector.tsx` + `.css` | Desaparece: el menú vive en `TopBar`. `dialog.kind = 'view-menu'` se quita de `src/state/ui.ts`. | La tabla `VIEWS` (a `TopBar`). | Todo el archivo, `vsel-*`, `layout.vsel_*`. |
| `src/app/Sidebar.tsx` | `div.sidebar` con gap `space-4`: `CreateButton`, `MiniCalendar`, `CalendarList`. | nada | `sidebar-head/foot/spacer-*`, `sidebar-sr-title`. |
| `src/app/CreateButton.tsx` | `md-filled-button` "Create event" con `md-icon` `add` en `slot="icon"`, dentro del cajón. | El cálculo de la franja por defecto (siguiente hora en punto del día ancla, una hora). | `create-button-shadow`, `layout.create_button_*`, `create-menu-*`. |
| `src/app/MiniCalendar.tsx` | Cabecera: mes title-small + dos `md-icon-button`; grilla: nuevo `src/components/MonthGrid.tsx` (7 columnas, seis filas, cada día un `DayNumber` `mini` o `year` con `today`/`selected`/`muted`). | `dayAria` (exportada, pasa a `MonthGrid`), la regla `shown.anchor === date ? shown.month : date`, `monthGrid(monthTs, tz, 6)`, `setDate(ts + 12*3600)`. | Los sufijos `-ripple`/`-label` por estado, los duplicados `-sr`. |
| `src/app/CalendarList.tsx` | Secciones con cabecera `<button>` + `md-ripple` + `md-icon` `expand_more`/`expand_less`; filas pill con `md-checkbox` (`touch-target="wrapper"`, colores por `--data-calendar-color`, sección 6), nombre body-medium con elipsis; "+" de Other calendars como `md-icon-button` `add`. | El toggle optimista con rollback en `CalendarRow.toggle()`, `byOrder`, `accountTooltip`, `calendarsOf`, la partición google/otros. | El tick duplicado (`-check-mark/-svg/-path`), `-check-mixed`, `-add-foot/-overlay`, `sidebar-list-gap`, `header` vs `header2`. |
| `src/app/Tooltip.tsx` | Componente `Tooltip` con hijos: envuelve el control, escucha `pointerenter`/`pointerleave`/`focusin`/`focusout` en el envoltorio, espera `tooltip_delay`, y dibuja el plain tooltip bajo el control (`position: fixed`, `left`/`top` calculados del rect). Un solo tooltip visible: estado en módulo. | La contención de `relatedTarget`, el cierre en `pointerdown` y `blur` de la ventana (de `installTooltips`, reescritos dentro del componente). | `installTooltips`, `data-tooltip`, `tooltip-root/-box/-label`. |
| `src/app/Snackbar.tsx` | Barra `inverse-surface` centrada abajo con texto y `md-icon-button` `close`; entra y sale deslizándose con `--ugc-motion-snackbar-*` y `usePresence`. | `dismissNotice`, `role="status"`, `aria-live`. `src/lib/notice.ts` no cambia. | La cadena `snackbar-close-cell/-close/-close-span/-close-icon-box`. |
| `src/app/ViewStage.tsx` | Igual; `src/styles/motion.css` pasa a `--ugc-motion-*` y `--md-sys-motion-*`. | Todo. | nada |
| `src/app/SettingsDialog.tsx` | `md-dialog` de 640 px con secciones (title-medium): cuentas como `md-list` de `md-list-item` (headline nombre, `supporting-text` estado y último error, `slot="end"` con `md-text-button` "Sign in again"/"Remove"), `md-text-button` "Add Google account" (ícono `add`) y "Sync now" (ícono `sync`); iCal con tres `md-outlined-text-field` y `md-text-button` "Add"; zonas horarias con dos `md-outlined-text-field`; calendario por defecto y cuenta de feriados con `md-outlined-select`; push con `md-outlined-text-field` + `md-text-button` "Test"; GNOME con `md-text-button`; mensaje de estado body-small; acciones Close/Save. | `current = draft ?? settings`, `run`, `save`, `testPush`, `addGoogle`, `addIcal`, `remove` con `window.confirm`, `goaToggle`, `writable`, `googleAccounts`, la codificación `account_id|calendar_id`. | El chrome `scope-*`, `settings-scrim`. |
| `src/app/WelcomeDialog.tsx`, `GoaDialog.tsx` | `md-dialog` no descartable (`oncancel` con `preventDefault`), texto body-medium, `md-filled-button` de acción (Welcome) o `md-text-button` No + `md-filled-button` Yes (GOA). | Toda la lógica de ambos (`configured`, `recheck`, `addAccount` con la cadena GOA, `answer`). | `welcome-scrim`, chrome `scope-*`. |
| `src/app/AddCalendarDialog.tsx` | `md-dialog` de 560 px: tres `md-outlined-text-field`, `md-text-button` "Add Google account" (ícono `add`) bajo su frase en el cuerpo, acciones `md-text-button` Cancel y `md-filled-button` Add. | `run`, `addIcal` con `trim`, `addGoogle`, el predicado del botón Add, el gating por `oauth_configured`. | `addcal-scrim`, `settings-input`, chrome `scope-*`. |
| `src/event/EventPopup.tsx` | Tarjeta (sección 7) posicionada con JS; cabecera con `md-icon-button` `edit`, `delete`, `more_vert` (abre `md-menu` con "Move to <cuenta>") y `close`; cuerpo: punto del color + título headline-small + fecha/recurrencia body-medium; fila Meet con `md-text-button` "Join with Google Meet" (ícono `videocam`) y el código body-small; teléfono como `md-text-button` "Join by phone" (`call`) y "More phone numbers" (`open_in_new`); invitados como lista propia (nombre body-medium, estado body-small `on-surface-variant`) con resumen; ubicación como texto body-medium con `md-text-button` "Open in Maps" (`open_in_new`) debajo; descripción; calendario y organizador. | `popupPosition`, `columnRect`, `attendeeSummary`, `order`, `nameOf`, `remove` con rama recurrente, el Escape que no actúa con overlay abierto, el cierre por click fuera, `meetCode`, la cadena `creator`, `can_edit`/`can_delete`. | Los ~217 `popup-*` medidos, los nodos laterales y `-sr`, `popup-creator-button` inerte, `create-menu-*`. |
| `src/event/QuickCreate.tsx` + `.css` | Se borra (sin disparador desde el 2026-09-15). `dialog.kind = 'quick-create'` se quita del store. | `writableCalendars` se muda a `src/event/FullForm.tsx` (o a `src/lib/calendars.ts`) porque `FullForm` la usa. | Todo lo demás, `qc-*`, `layout.qc_*`. |
| `src/event/FullForm.tsx` | `md-dialog` de 640 px: título `md-outlined-text-field` (autofocus); fila con `DatePicker` inicio, `md-outlined-select` hora inicio, `DatePicker` fin, `md-outlined-select` hora fin, `md-switch` "All day"; `md-outlined-select` "Repeat" (Custom abre `RecurrenceDialog`); Meet: `md-text-button` "Add Google Meet" (ícono `videocam`) que pasa a fila "Google Meet will be added on save" con `md-icon-button` `close`; `md-outlined-text-field` ubicación; `md-outlined-text-field` `type="textarea"` descripción; `md-outlined-select` calendario; `md-outlined-select` color con un punto de color en `slot="start"` de cada `md-select-option`; `md-outlined-select` Busy/Free; `md-outlined-select` visibilidad; notificaciones como filas `md-outlined-text-field type="number"` + `md-outlined-select` unidad + `md-outlined-select` método + `md-icon-button` `close`, y `md-text-button` "Add notification"; error body-small `error`; acciones Cancel / Save (`md-filled-button`, deshabilitado mientras `busy`). | `recurrenceOptionsFor`, `nextVisibility`, `TIMES` y `timeOption`, `withTime`, `moveStart`/`moveEnd`, el handshake `pendingRrule`/`appliedRule`, `knownRule`/`currentRule`/`onRecurrence`, `draft()` con la conversión de fin exclusivo, `save()` con sus tres ramas, la aritmética de recordatorios, el efecto de carga, el Escape que cede al overlay (pasa a `oncancel`). | `ef-*`, `--ff-*`, `scope-root`, los spans `ugc-state`. |
| `src/event/DatePicker.tsx` | `md-outlined-text-field` de solo lectura (`readonly`, valor `EEE, MMM d, yyyy`) con `md-icon-button` `calendar_today` en `slot="trailing-icon"`; popover con `MonthGrid` + cabecera de mes. | `pick` (conserva la hora), `toggle`, `shift`, `DATE_FORMAT`, `dayAria`. | El árbol `sidebar-minical-*` prestado. |
| `src/event/EditScopeDialog.tsx` | `md-dialog` de 560 px con tres `md-radio` (`touch-target="wrapper"`, etiquetas como `<label>`), acciones Cancel / OK. | `apply()` que cierra primero el diálogo de abajo, la captura de Escape con `stopImmediatePropagation` (pasa a `oncancel`), la tabla de opciones. | `scope-scrim*`, los radios duplicados, `scope-radio` vs `scope-radio2`. |
| `src/event/RecurrenceDialog.tsx` | `md-dialog` "Custom recurrence": "Repeat every" `md-outlined-text-field type="number"` (intervalo) + `md-outlined-select` (day/week/month/year, plural según intervalo); "Repeat on" (solo WEEKLY) `md-chip-set` con siete `md-filter-chip`; "Ends" con tres `md-radio` (Never / On + `DatePicker` / After + `md-outlined-text-field type="number"` + "occurrences"); acciones Cancel / Done. | `parseRule`, `buildRule` (se les agregan tests unitarios), `startDay`, `done` con la validación de fecha (la fecha ahora viene de `DatePicker`, no de texto), `toggleDay` que no vacía la lista, `NEXT_FREQ`/`FREQ_LABEL`. | Los ~200 `rec-*`, el combobox falso, los spinners, los radios duplicados. |
| `src/views/week/WeekView.tsx` | Un solo conjunto de clases `week-*`; modo día con `data-mode="day"` en la raíz. Estructura: cabecera (`WeekHeader` también en modo día, con una columna), fila de todo el día, scroller con gutter + columnas; líneas `outline-variant`; línea de ahora `error`. | `gmtLabel`, `chipColumn`, el scroll inicial (ahora `week_initial_scroll_hour` × `hour_row_height`, con la regla de la línea de ahora), `syncGutter` y `data-scrolled` (sombra `elevation-level1` bajo la cabecera al scrollear), el filtro de calendarios ocultos, la partición todo-el-día/con-hora con el rebase UTC→local, `layoutDay(items, {dayStart, minMinutes: 15})`, las etiquetas de hora, `nowTop`. | `week-allday-overlay*`, `week-row-pad`, `week-daycol-bg`, `day-main-n3`, `week-header-bottom`, `week-gutter-cell-first`, la duplicación `week-x day-x`, `week-daycol-sr`. |
| `src/views/week/WeekHeader.tsx`, `DayHeader.tsx` | Un solo `WeekHeader` que recibe `days` (1 o 7); cada columna: nombre label-medium + `DayNumber` `day` (hoy: `primary`). `DayHeader.tsx` se borra. | `dayHeaderAria`, `openDay`. | `-today` duplicados, `week-header-pad`, `day-header-*`. |
| `src/views/week/AllDayRow.tsx` | Fila con chips tonales de 24 px posicionados en porcentaje; sin lista de placeholders. | `layoutAllDay`, `rowCount`, la geometría porcentual, `open(o)` con el rect como ancla, `chipDescription`. | `allday-ul/li`, `allday-chip-sr`, `allday-pad`, la duplicación de modo día. |
| `src/views/week/EventChip.tsx` | Un solo DOM: contenedor `role="button"` con `md-ripple`, título (label-medium) y hora (body-small); clase `chip-short` cuando `minutes <= 30` (solo título); colores por `--data-chip-*`; `declined` (tachado y `container` al 50 %), `tentative` (borde discontinuo). | `chipDescription`, el mapeo RSVP, `chip-stacked` cuando `column > 0`, `open(e)` con el rect. | `chipShape` y los cuatro árboles tiny/short/mid/long, `chip-loc-*`, `chip-sr`, `chip-resize*`. |
| `src/views/month/MonthView.tsx` | Grilla de celdas `outline-variant`; número de día `DayNumber` `month`; chips de todo el día tonales de 20 px; eventos con hora como punto + texto; "N more" como botón de texto. | `visibleSlots`, el `ResizeObserver`, `monthGrid`, la partición y el rebase, la asignación de slots y el desborde, `layoutAllDay` por semana, `openEvent`, `openDay`, la selección de clase del número. | `month-row-bg`, `month-cell-sr`, `month-chip-sr`, `month-timed-sr`, `month-header-pres`, `day-main*`. |
| `src/views/year/YearView.tsx` | Doce tarjetas `surface-container-low` con `MonthGrid` compacta (`year_cell_size`). | El ancla a mediodía, `monthGrid(monthTs, tz, 6)`, `openDay`, `dayAria`. | `year-th-span`, `year-title-box`, `year-month-inner`, `year-page`. |
| `src/views/agenda/AgendaView.tsx` | Grupos por día (`DayNumber` `day` + etiqueta), filas de 48 px con `md-ripple`. | `AGENDA_DAYS`, el rebase, el bucle de agrupación con eventos multi-día repetidos, `openEvent`, `openDay`, la etiqueta de hora, `chipDescription`, el separador de hoy. | `agenda-date-sr`, `agenda-dot-sr`, los pares today/other duplicados. |
| `src/lib/motion.ts` | Queda `ms` y `usePresence`. | `ms`, `usePresence`. | `installRipples`, `rippleHost`, `installTooltips`, `tooltipFor`, `TooltipState`. |
| `src/styles/motion.css` | Solo las clases que usan `ViewStage`, el popup, el snackbar y el tooltip, con `--ugc-motion-*`. | `.app-view-layer*`, `.motion-popup-*`, `.snackbar-enter/-exit`, `.tooltip-enter/-exit`. | `.ugc-state*`, `.motion-menu*`, `.motion-page*`, las variables `--motion-*`. |
| `src/lib/colors.ts` | `chipColors(hex, theme)` y `useTheme()` (sección 6). | `useTheme`. | `chipBackground`. |
| `src/state/ui.ts` | `Dialog` sin `quick-create` ni `view-menu`. | Todo lo demás. | esas dos variantes. |
| `src/dev/measure.ts`, `src/styles/measured.css`, `src/styles/legacy.css` | Se borran en M5. | | |

Componentes nuevos: `src/components/MonthGrid.tsx` (+ `.css`), `src/components/DayNumber.tsx` (+ `.css`, 2026-09-16), `src/m3/register.ts`, `src/m3/jsx.d.ts`, `vitest.setup.ts`.

## 10. Movimiento

- Los `<md-*>` animan solos (diálogos, menús, ripples, switches, campos).
- Lo propio de la app usa `--ugc-motion-*` (resueltos a `--md-sys-motion-*` en `theme.json`): deslizamiento de la grilla al navegar (`nav_slide_*`), cross-fade al cambiar de vista (`view_fade_duration`), popup que sube desde el chip (`popup_open_*`, `popup_rise`) y se desvanece (`popup_close_*`), snackbar que entra desde abajo (`snackbar_slide_duration`), tooltip (`tooltip_*`), cajón (`drawer_*`).
- `usePresence` sigue existiendo para popup, snackbar y tooltip; los diálogos ya no lo necesitan.
- `prefers-reduced-motion` no se contempla en la versión 1 (no está en `docs/01`).

## 11. Qué se borra y qué queda como referencia

- Se borran del repo (M5): `src/styles/measured.css`, `src/styles/legacy.css`, `scripts/gen-measured-css.mjs`, `scripts/gen-tokens.mjs`, `scripts/measure/` entero, `src/dev/`, `docs/design/tokens.json`, `docs/design/token-spec.json`, `docs/design/icons.txt`, `docs/design/fixture-events.md`, `src-tauri/src/commands/dev.rs` con el registro de `dev_dump` y la ruta `POST /dev/measure` de `src-tauri/src/webhook/server.rs`, `src-tauri/resources/fonts/GoogleMaterialIcons-subset.woff2`, `.claude/skills/measure-component/`, y las ramas 4 y 7 de `scripts/check-phase.sh`.
- Se mueve a `docs/design/google/` con un `README.md` de una página: `docs/design/measurements/` (JSON y `.md`; los PNG ya estaban fuera de git). Es la referencia histórica de cómo era Google Calendar el 2026-09-14; nadie la lee para implementar.
- `docs/04-fidelidad-visual.md` queda con una nota al final que remite a este documento. `docs/research/pixel-perfect.md` no se toca (`docs/research/` es de solo lectura).

## 12. Verificación visual y de memoria

- No hay comparación contra Google. La verificación es mirar la app: correrla en el Xvfb privado (`docs/10` sección 5), `scrot` en claro y en oscuro (`GTK_THEME=Adwaita:dark` antes de lanzar el binario para el oscuro), y leer las capturas con la herramienta de lectura de imágenes. Se revisa contra la sección 7 y el mockup: superficies, radios, tipografía, densidad, estados de hover (con `xdotool mousemove`), y que ningún control quede sin etiqueta.
- RAM: al cerrar la transición se repite `bash scripts/measure-ram.sh` en el Xvfb con el build de release y se agrega la fila `M6` a `docs/99`. Expectativa: el WebKitWebProcess baja respecto de los 128 MB de F8-T2 porque desaparecen las 50 758 líneas de `measured.css`; Lit y unos veinte componentes suman menos de 300 KB de JS.
- Cualquier diferencia entre WebKitGTK y lo que muestra Chrome con el mockup se anota en `docs/10` sección 3 si no tiene arreglo.

## Registro de cambios

- 2026-09-16 — Versión inicial, escrita en la sesión de planificación de la transición (`docs/99`, entrada del 2026-09-16).
- 2026-09-16 — Transición ejecutada (M0 a M6, versión 0.2.0). Desvíos menores respecto de las secciones 4 y 9 registrados en `docs/99`, entrada "Ejecución de la transición a Material 3": nombres de glifo en `icons.txt`, tokens de layout nuevos en `theme.json`, dos filas de fecha y hora en el formulario, gutter dentro del scroller, valores `none` en los `md-select`.
- 2026-09-16 — Jerarquía única de botones (`docs/99`, "Jerarquía única de botones"): regla en la sección 8, `md-filled-tonal-button` fuera de la sección 5, filas de Settings, Add calendar, formulario y popup de las secciones 7 y 9, `DayNumber` en lugar de los cuatro números de día.
- 2026-09-17 — Tres botones (`docs/99`, "Tres botones"): la regla de la sección 8 se reduce a primario, secundario e ícono; `md-fab`, `md-outlined-button` y `md-assist-chip` salen de la sección 5 y de las filas de las secciones 7 y 9.
