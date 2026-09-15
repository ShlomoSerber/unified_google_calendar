# Vista año (componente 23)

Fecha: 2026-09-15. Referencia: calendar.google.com, `/r/year/2026/9/14`, cuenta de Greelow.

## Procedimiento

- Google: `node scripts/measure/capture.mjs year_view`. Archivos `year_view-<tema>.json/.png` (2608 nodos, raíz 1168×824).
- Tokens: `docs/design/token-spec.json` componente `year` (solo nodos nombrados: raíz, grilla, bloque de mes, título, tabla, cabecera, celda, día, día de hoy) y `layout.year_month_cols` (ancho del bloque, 284 px).
- App: `src/views/year/YearView.tsx`; sin dump propio todavía (`app-components.mjs` no lo lista).

## Resultado

Doce bloques de 284×252 px en cuatro columnas y tres filas (x 0, 284, 568, 852; y 8, 260, 512) dentro de una grilla de 1168×764 con `padding-top` 8 px y `padding-right` 32 px. Cada bloque: título de 15 px / 500 (`"Google Sans"`, `rgb(68, 71, 70)`) en una caja de 32 px, tabla de 208×196 px con cabecera de 28 px (`M T W T F S S`, 12 px) y seis filas de 28 px; días como botones de 24×24 px con dígitos de 10 px / 500; hoy con fondo `rgb(11, 87, 208)` y texto blanco. Los días de meses vecinos se muestran con el mismo color que los del mes.

## Diferencias aceptadas

- La app hace fluida la grilla (`flex-wrap`) y scrollea la página cuando la ventana es más baja que la referencia; Google fija 1168×824 para 1440×900.
- Un click en un día abre la vista día; en Google abre un popup con los eventos del día (fuera de alcance).
