# Cabecera de semana (componente 6)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs week_header` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `week_header[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs week_header` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `week_header[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`).

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| week_header | light | 30 | 0 | 0.98% |
| week_header | dark | 30 | 0 | 0.97% |

## Diferencias aceptadas

- odiff: rasterizado de los números de día (26 px) y de los nombres de día. Con tolerancia de 1 px no hay diferencias de layout; con 0.5 px aparecen los anchos de texto de "16", "19" y "20" (0.3–0.6 px de diferencia de métrica de fuente).
