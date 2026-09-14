# Botón Create y menú (componente 3)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs create_button` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `create_button[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs create_button` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `create_button[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`).

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| create_button | light | 3 | 0 | 3.69% |
| create_button | dark | 3 | 0 | 2.45% |
| create_button-open | light | 6 | 0 | 3.64% |
| create_button-open | dark | 6 | 0 | 3.72% |

## Diferencias aceptadas

- odiff: rasterizado de texto e íconos de fuente (`add`, `arrow_drop_down`). Sin diferencias de layout en ambos estados y temas.
- El menú solo tiene "Event" activo; los otros tres ítems conservan la geometría medida y quedan deshabilitados (`docs/99`, F4-T3).
