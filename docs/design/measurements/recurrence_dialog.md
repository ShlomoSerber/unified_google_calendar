# Diálogo de recurrencia personalizada (componente 18)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs recurrence_dialog` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `recurrence_dialog[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs recurrence_dialog` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `recurrence_dialog[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`). Las diferencias admitidas por componente están en `ALLOWED` de `report.mjs` y se justifican abajo.

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| recurrence_dialog | light | 30 | 6 | 1.58% |
| recurrence_dialog | dark | 30 | 6 | 1.15% |

## Diferencias aceptadas

- Construye `RRULE` con FREQ, INTERVAL, BYDAY (semanal), UNTIL o COUNT; el resultado vuelve al formulario por el store (`ui.pendingRrule`). El día seleccionado usa los tokens del botón "Thursday" del volcado (fondo primario, letra blanca).
- Diferencias restantes: ancho del texto "Repeat every" (métrica de fuente) que desplaza 2 px la fila.
