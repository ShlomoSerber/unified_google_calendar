# Vista agenda (componente 13)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs agenda_view` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `agenda_view[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs agenda_view` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `agenda_view[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`). Las diferencias admitidas por componente están en `ALLOWED` de `report.mjs` y se justifican abajo.

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| agenda_view | light | 94 | 13 | 1.79% |
| agenda_view | dark | 94 | 13 | 1.85% |

## Diferencias aceptadas

- Un grupo por día con eventos desde la fecha de anclaje, seis semanas (Google carga un rango deslizante). El separador rojo bajo el grupo de hoy y la ubicación como segunda columna de texto se miden en el volcado.
- Diferencias restantes: anchos de texto de títulos y de las etiquetas "Sep, Tue" (métrica de fuente, 1–3 px).
