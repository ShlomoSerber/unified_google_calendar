# Fila de todo el día (componente 7)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs allday_row` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `allday_row[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs allday_row` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `allday_row[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`).

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| allday_row | light | 4 | 0 | 1.81% |
| allday_row | dark | 4 | 0 | 1.78% |

## Diferencias aceptadas

- La fila de referencia tiene celdas con 0, 1 y 2 chips y 48 px de alto (dos filas de 24 px). La app calcula el alto como filas × 24 px con un mínimo de una fila; el alto con cero eventos en toda la semana no se midió.
- odiff: rasterizado del texto de los chips.
