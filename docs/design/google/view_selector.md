# Selector de vista (componente 17)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs view_selector` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `view_selector[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs view_selector` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `view_selector[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`). Las diferencias admitidas por componente están en `ALLOWED` de `report.mjs` y se justifican abajo.

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| view_selector | light | 19 | 0 | 3.98% |
| view_selector | dark | 19 | 0 | 7.62% |

## Diferencias aceptadas

- Sin diferencias de layout. "Year", "4 days" y las tres casillas ("Show weekends", "Show declined events", "Show completed tasks") quedan inertes: son versión 2 o Tasks.
