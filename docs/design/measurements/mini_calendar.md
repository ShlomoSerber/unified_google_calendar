# Mini calendario (componente 4)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs mini_calendar` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `mini_calendar[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs mini_calendar` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `mini_calendar[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`).

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| mini_calendar | light | 69 | 0 | 2.57% |
| mini_calendar | dark | 69 | 0 | 2.60% |
| mini_calendar-next_week | light | 69 | 0 | 2.57% |
| mini_calendar-next_week | dark | 69 | 0 | 2.59% |
| mini_calendar-other_month | light | 69 | 0 | 2.42% |
| mini_calendar-other_month | dark | 69 | 0 | 2.44% |

## Diferencias aceptadas

- Estados: mes actual, mes siguiente (`other_month`) y otra semana seleccionada (`next_week`, día seleccionado en `rgb(194, 231, 255)` / `rgb(0, 74, 119)` sobre `rgb(194, 231, 255)`).
- odiff: rasterizado de los números a 10 px. Sin diferencias de layout.
