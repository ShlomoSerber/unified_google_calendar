# Cajón completo (componentes 3, 4 y 5 en su contenedor)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs sidebar` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `sidebar[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs sidebar` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `sidebar[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore "<nombres de calendarios y secciones>" --ignore-hidden` más `odiff --antialiasing --threshold 0.1`).

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| sidebar | light | 79 | 3 | 4.48% |
| sidebar | dark | 79 | 3 | 4.51% |

## Diferencias aceptadas

- El volcado del cajón entero fija los desplazamientos entre el botón Create, el minicalendario, "Meet with…", "Booking pages" y la lista. Las dos secciones intermedias quedan inertes en la app (`docs/99`, F4-T3).
- Las diferencias de layout restantes son las de la lista de calendarios (ver `calendar_list.md`).
