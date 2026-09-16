# Vista mes (componente 12)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs month_view` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `month_view[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs month_view` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `month_view[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`). Las diferencias admitidas por componente están en `ALLOWED` de `report.mjs` y se justifican abajo.

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| month_view | light | 68 | 3 | 2.34% |
| month_view | dark | 68 | 3 | 2.39% |

## Diferencias aceptadas

- Cinco filas para septiembre de 2026 (la app calcula las semanas que cubre el mes, como Google; el minicalendario sigue mostrando seis). Celdas con 0 a 6 eventos: las que desbordan muestran "N more" en la última ranura, con las ranuras calculadas a partir de la altura real de la celda (24 px por chip).
- Diferencias restantes: anchos de texto de "Tue"/"Sun" y de un título (métrica de fuente).
