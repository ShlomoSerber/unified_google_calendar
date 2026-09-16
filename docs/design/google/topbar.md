# Barra superior (componente 2)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs topbar` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `topbar[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs topbar` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `topbar[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore "Google Account" --ignore-hidden` más `odiff --antialiasing --threshold 0.1`).

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| topbar | light | 20 | 0 | 3.88% |
| topbar | dark | 20 | 0 | 3.75% |

## Diferencias aceptadas

- `--ignore "Google Account"`: el botón de cuenta de Google lleva el nombre y el e-mail del usuario en el aria-label; en la app el mismo nodo lleva los datos de la cuenta activa.
- `--ignore-hidden`: nodos de 1×1 px para lectores de pantalla; su posición depende de la posición estática de un elemento absoluto, invisible por definición.
- odiff: el logo de Google se reemplaza por el ícono propio (`docs/04` sección 9); el avatar de la cuenta es una inicial; el resto son diferencias de rasterizado de texto entre Chrome y WebKitGTK (`docs/04` sección 8). Sin diferencias de layout.
