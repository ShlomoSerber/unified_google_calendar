# Grilla de horas (componente 8)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs hour_grid` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `hour_grid[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs hour_grid` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `hour_grid[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`).

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| hour_grid | light | 62 | 10 | 2.22% |
| hour_grid | dark | 62 | 10 | 2.24% |

## Diferencias aceptadas

- Las dos partes se vuelcan con la grilla desplazada a 420 px (07:00) en Google y en la app, porque la posición inicial de Google depende de la hora (`tokens.json`, `layout.week_initial_scroll_note`).
- Diferencias de layout restantes: anchos de texto de "Fifteen" y "Ninety" (métrica de fuente, ~1 px) y el chip "Declined": la referencia muestra la copia del organizador en "UGC Fixtures" (sin estilo de rechazo) porque el calendario principal estaba oculto, mientras que la app deduplica las dos copias en un solo chip con la respuesta del usuario (`docs/99`, F4-T4).
- Líneas de hora, sombra bajo la cabecera y degradados en los extremos: pseudoelementos sondeados con `getComputedStyle(el, '::after'|'::before')` y registrados en `tokens.json` (`component.week.*_note`).
- odiff: rasterizado de texto; la línea de ahora está en otra hora en cada captura.
