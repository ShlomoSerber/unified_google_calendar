# Línea de hora actual (componente 9)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs now_line` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `now_line[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs now_line` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `now_line[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`).

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| now_line | light | 0 | 0 | n/a% |
| now_line | dark | 0 | 0 | n/a% |
| now_line-dot | light | 0 | 0 | 100.00% |
| now_line-dot | dark | 0 | 0 | 0% |

## Diferencias aceptadas

- Un solo nodo por estado (línea de 2 px, punto de 12 px con margen −5/−6.5 px); las posiciones verticales dependen de la hora de cada captura y se comparan por estilo. Cuando la hora actual queda fuera de la ventana visible la captura PNG no tiene contenido comparable.
