# Quick create (componente 15)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs quick_create` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `quick_create[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs quick_create` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `quick_create[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`). Las diferencias admitidas por componente están en `ALLOWED` de `report.mjs` y se justifican abajo.

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| quick_create | light | 37 | 10 | 2.62% |
| quick_create | dark | 37 | 10 | 2.60% |
| quick_create-with_title | light | 37 | 10 | 2.65% |
| quick_create-with_title | dark | 37 | 10 | 2.63% |

## Diferencias aceptadas

- Estados vacío y con título. La caja se ancla a la columna del slot y, si el slot queda por debajo de la posición centrada, su borde inferior queda 1 px por encima del slot (`layout.popup_anchor_note`).
- Las pestañas Task / Out of office / Appointment schedule y "Dock to sidebar" quedan inertes. Las filas de invitados, ubicación y descripción abren el formulario completo con el borrador; Meet y calendario se eligen aquí.
- Diferencias restantes: anchos de texto (métrica de fuente) y el calendario propuesto (Google eligió "UGC Fixtures", el único visible; la app propone el calendario por defecto de Ajustes o el primero con permiso de escritura).
