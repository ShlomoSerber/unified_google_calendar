# Animaciones de calendar.google.com

Medido el 2026-09-15 con `node scripts/measure/animations.mjs` (Chrome 152 headless, 1440×900, perfil `~/.chrome-measure`). Para cada escenario el script pone la página en un estado conocido, inyecta `scripts/measure/animRecorder.js`, dispara la acción y guarda cada Web Animation que corre la página (`document.getAnimations()` muestreado por `requestAnimationFrame`: destino, keyframes, duración, easing) más la caja, opacidad y transform de los elementos vigilados cuadro por cuadro. Los escenarios `hover_*` y `press_*` guardan además la diferencia de estilos computados entre reposo y hover en claro y oscuro. Salida: `animations-light.json` y `animations-dark.json` (solo hovers en oscuro; los tiempos no dependen del tema).

Todos los valores derivados están en `docs/design/tokens.json`, `component.motion`. Esta tabla resume lo observado; el JSON tiene el detalle.

## Movimiento

| Escenario | Qué anima Google | Valor |
|---|---|---|
| `drawer_close`, `drawer_open` | Cajón: `transform translateX(∓256px) → none`; área principal: `scaleX` FLIP en paralelo | 300 ms, `cubic-bezier(0.4, 0, 0.2, 1)` |
| `nav_next`, `nav_prev`, `today`, `nav_next_month`, `minical_pick_day` | La grilla nueva entra con `opacity 0→1` y `translate(±56px)→0`; +56 hacia adelante, −56 hacia atrás. La grilla vieja se reemplaza sin animar | 200 ms, estándar |
| `view_to_*` | Cross-fade: el `[role=main]` viejo `opacity 1→0`, el nuevo `0→1`, en paralelo | 200 ms, estándar |
| `full_form_open/close`, `settings_page_open` | Cross-fade igual al cambio de vista; la burbuja de creación rápida se desvanece antes (150 ms lineal) | 200 ms, estándar |
| `event_popup_open`, `quick_create_open` | El diálogo entra con `opacity 0→1` y `translate(∓32px)→none`, alejándose del chip | 250 ms, estándar |
| `event_popup_close`, `quick_create_close` | `opacity 1→0` | 150 ms, lineal |
| `view_menu_open`, `create_menu_open`, `settings_menu_open` | `opacity 0→1` 30 ms lineal + `scale(0.8)→1` 120 ms `cubic-bezier(0, 0, 0.2, 1)` | ver tokens |
| `*_menu_close` | `opacity 1→0` | 75 ms, lineal |
| `minical_next`, `minical_prev` | Sin deslizamiento; solo el fondo de los días (semana seleccionada, hoy) transiciona | 100 ms, lineal |
| `calendar_toggle_*`, `hover_calendar_row` | Fondo de la fila | 100 ms, lineal |
| `hover_create`, `press_create` | `box-shadow` del botón Create al valor de hover | 280 ms, estándar |
| `tooltip_*`, `hover_*` con tooltip | Aparece 540–557 ms después del puntero: `opacity 0→1` y `scale(0.8)→1` 150 ms `cubic-bezier(0, 0, 0.2, 1)`; se oculta con `opacity 1→0` 75 ms `cubic-bezier(0.4, 0, 1, 1)` | ver tokens |

## Estados de botón (capa de estado Material)

Cada botón (Today, selector de vista, flechas, Settings, cajón, Create, días y flechas del mini calendario, casillas de la lista, ítems de menú) tiene un `span.UTNHae`:

- `::before` = tinte hover: `opacity 0 → 0.08` en 75 ms lineal. Fondo = color de estado del botón.
- `::after` = tinte pulsado: `opacity 0 → 0.1` en 105 ms lineal (0.12 en ítems de menú), vuelve a 0 en 250 ms lineal. Fondo `radial-gradient(closest-side, <color> max(100% - 70px, 65%), transparent 100%)`.
- Ripple: `element.animate` sobre `::after` desde un círculo de 6 px en el puntero hasta cubrir el botón, 450 ms `cubic-bezier(0.2, 0, 0, 1)`.

Colores de estado (claro / oscuro): botones de texto `rgb(31, 31, 31)` / `rgb(227, 227, 227)`; botones de ícono `rgb(68, 71, 70)` / `rgb(196, 199, 197)`; Create y días del mini calendario `rgb(0, 74, 119)` / `rgb(194, 231, 255)`. El botón del cajón cambia su fondo sin transición a `rgba(60, 64, 67, 0.08)` / `rgba(232, 234, 237, 0.08)`.

Los chips de evento no cambian de estilo al pasar el puntero (`hover_chip`: ningún cambio computado); al seleccionarlos su `::before` pasa a opacidad 1 en 100 ms pero es transparente.

## Fuera de alcance

- Anillos de foco por teclado (`gm3-focus-ring-*`): la navegación por teclado es versión 2.
- La barra de progreso lineal de la página de Settings (`gm3-lpi-*`): la app no carga páginas.
- `Escape` no llega al popup en Chrome headless; los escenarios de cierre usan el botón Close del diálogo.

## Tolerancias

Las duraciones se toman tal cual. Los offsets (56 px, 32 px) son literales de Google en píxeles CSS y se usan sin escalar. En WebKitGTK las transiciones corren con el mismo reloj; la comparación es visual, no de píxeles.
