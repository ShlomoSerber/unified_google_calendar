# Vista día (componente 11)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs day_view` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `day_view[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs day_view` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `day_view[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`). Las diferencias admitidas por componente están en `ALLOWED` de `report.mjs` y se justifican abajo.

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| day_view | light | 19 | 2 | 0.79% |
| day_view | dark | 19 | 2 | 0.85% |

## Diferencias aceptadas

- La vista día reutiliza la grilla de la semana con una columna (`WeekView mode="day"`); los nodos que difieren llevan además las clases `day-*` medidas en este volcado (cabecera con la etiqueta del día a la izquierda, fila de todo el día de 24 px, raíz sin padding).
- Ambas partes se vuelcan con la grilla en 420 px (07:00). Diferencias restantes: anchos de texto de los chips (métrica de fuente).
