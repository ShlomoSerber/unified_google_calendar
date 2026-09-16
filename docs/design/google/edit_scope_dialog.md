# Diálogo "Edit recurring event" (componente 19)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs edit_scope_dialog` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `edit_scope_dialog[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs edit_scope_dialog` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `edit_scope_dialog[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`). Las diferencias admitidas por componente están en `ALLOWED` de `report.mjs` y se justifican abajo.

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| edit_scope_dialog | light | 7 | 6 | 23.02% |
| edit_scope_dialog | dark | 7 | 6 | 21.75% |

## Diferencias aceptadas

- La referencia se tomó con "Delete event" sobre la primera ocurrencia de "Weekly repeat": Google oculta "This and following events" en la primera ocurrencia y muestra dos opciones. La app ofrece siempre las tres de `docs/03` sección 4, de ahí las 5 diferencias (alto del diálogo y posiciones bajo la opción extra). Los estilos de opción, radio y botones coinciden. El título lleva `white-space: nowrap` porque WebKitGTK lo compone unos píxeles más ancho y lo partía en dos líneas.
