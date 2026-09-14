# Chip de evento (componente 10)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs event_chip` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `event_chip[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs event_chip` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `event_chip[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`).

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| event_chip | light | 3 | 0 | 4.47% |
| event_chip | dark | 3 | 0 | 4.15% |
| event_chip-all_day | light | 2 | 0 | 6.69% |
| event_chip-all_day | dark | 2 | 0 | 6.92% |
| event_chip-declined | light | 3 | 4 | 51.37% |
| event_chip-declined | dark | 3 | 4 | 51.04% |
| event_chip-fifteen | light | 4 | 3 | 15.55% |
| event_chip-fifteen | dark | 4 | 3 | 13.53% |
| event_chip-forty_five | light | 3 | 0 | 6.20% |
| event_chip-forty_five | dark | 3 | 0 | 5.57% |
| event_chip-hover | light | 3 | 0 | 4.47% |
| event_chip-hover | dark | 3 | 0 | 4.15% |
| event_chip-ninety | light | 3 | 1 | 2.36% |
| event_chip-ninety | dark | 3 | 1 | 2.22% |
| event_chip-overlap_a | light | 3 | 0 | 8.77% |
| event_chip-overlap_a | dark | 3 | 0 | 8.44% |
| event_chip-overlap_b | light | 3 | 0 | 13.82% |
| event_chip-overlap_b | dark | 3 | 0 | 13.27% |
| event_chip-past | light | 3 | 0 | 3.56% |
| event_chip-past | dark | 3 | 0 | 3.55% |
| event_chip-sixty | light | 3 | 0 | 3.56% |
| event_chip-sixty | dark | 3 | 0 | 3.55% |
| event_chip-tentative | light | 3 | 3 | 68.32% |
| event_chip-tentative | dark | 3 | 3 | 67.95% |
| event_chip-thirty | light | 4 | 0 | 4.90% |
| event_chip-thirty | dark | 4 | 0 | 4.22% |
| event_chip-triple_a | light | 3 | 0 | 13.93% |
| event_chip-triple_a | dark | 3 | 0 | 12.95% |
| event_chip-triple_b | light | 3 | 0 | 14.26% |
| event_chip-triple_b | dark | 3 | 0 | 13.28% |
| event_chip-triple_c | light | 3 | 0 | 8.06% |
| event_chip-triple_c | dark | 3 | 0 | 7.38% |
| event_chip-with_location | light | 4 | 0 | 10.11% |
| event_chip-with_location | dark | 4 | 0 | 9.39% |

## Diferencias aceptadas

- Estados medidos: 15, 30, 45, 60 y 90 min, solapados de 2 y 3, día completo, con ubicación, hover, pasado, tentativo y rechazado. Hover no cambia ningún estilo medido (la sombra de `::before` está en `opacity: 0`); "pasado" no se atenúa con los ajustes de referencia.
- Tentativo y rechazado: la referencia es la copia del invitado en el calendario principal (mostrada junto a la de "UGC Fixtures"), de dos columnas de ancho y con el color de ese calendario; la app deduplica las dos copias en un chip de una columna con el color del calendario ganador. Se comparan los estilos (rayas `linear-gradient` a 45°, fondo de superficie y texto del color del calendario en el rechazado), no el ancho ni el color.
- Los chips de la columna 1+ de un grupo solapado llevan el contorno de 1 px del color de superficie (`::after` sondeado) y z-index 5 + columna.
- odiff: rasterizado de texto (WebKitGTK dibuja Google Sans Text 500 algo más gruesa que Chrome) y el borde vertical izquierdo, porque Chrome recorta la captura en x fraccionario (1123.375) y la app en entero.
