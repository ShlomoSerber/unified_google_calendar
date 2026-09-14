# Lista de calendarios (componente 5)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs calendar_list` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `calendar_list[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs calendar_list` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `calendar_list[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore "<nombres de calendarios y secciones>" --ignore-hidden` más `odiff --antialiasing --threshold 0.1`).

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| calendar_list | light | 3 | 3 | 39.58% |
| calendar_list | dark | 3 | 3 | 39.93% |
| calendar_list-hover | light | 3 | 4 | 41.11% |
| calendar_list-hover | dark | 3 | 4 | 41.56% |

## Diferencias aceptadas

- Contenido distinto por diseño: Google muestra "My calendars" con los calendarios de una cuenta; la app agrupa por cuenta (avatar + nombre) y pone los calendarios iCal en "Other calendars" (`docs/08` F4-T3, `docs/99`). Las filas de otras cuentas no existen en la referencia y las de la referencia (Birthdays, Tasks) no existen en la app, por eso se ignoran por nombre; las diferencias de layout que quedan son la altura total del panel (más filas) y el nombre de la sección.
- La fila medida en detalle es "UGC Fixtures" (checkbox marcado, fondo del color del calendario) y la de "Holidays in Argentina" (sin marcar, borde de 2 px del color); el estado `hover` mide el fondo `rgb(233, 238, 246)` de la fila. El botón "⋮" que Google muestra al pasar el mouse no existe en la versión 1.
- El color de cada checkbox es el del calendario traducido a la paleta medida (`docs/design/measurements/palette.json`).
