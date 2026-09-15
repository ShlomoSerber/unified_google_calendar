# Popup de detalle de evento (componente 14)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs event_popup` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `event_popup[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs event_popup` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `event_popup[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`). Las diferencias admitidas por componente están en `ALLOWED` de `report.mjs` y se justifican abajo.

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| event_popup | light | 17 | 6 | 3.18% |
| event_popup | dark | 17 | 6 | 3.18% |
| event_popup-guests | light | 28 | 10 | 3.58% |
| event_popup-guests | dark | 28 | 10 | 3.59% |
| event_popup-meet | light | 30 | 12 | 3.48% |
| event_popup-meet | dark | 30 | 12 | 3.49% |
| event_popup-recurring | light | 18 | 6 | 3.25% |
| event_popup-recurring | dark | 18 | 6 | 3.16% |

## Diferencias aceptadas

- Estados: evento simple (Weekend), con Meet (con la fila de teléfono y "More phone numbers"), con invitados (lista de invitados con estado) y recurrente (línea "Weekly on Friday"). El popup se ancla a la columna del día del chip (a la derecha si entra, si no a la izquierda) y se centra verticalmente; ver `layout.popup_anchor_note` en `tokens.json`, sondeado con diez chips.
- "Take meeting notes", "Email event details", "Chat with guests" y el botón de notas de reunión son funciones de Workspace fuera de la versión 1: quedan visibles e inertes para conservar la geometría.
- Diferencias restantes: anchos de texto (métrica de fuente); "Created by: Shlomo Serber" en Google frente al e-mail de la cuenta en la app (la API no entrega el nombre del creador de un calendario secundario); los nombres de los invitados, que Google resuelve desde los contactos y la app solo conoce para sus propias cuentas; 2 px en el botón de notas de reunión (alineación no volcable). La fila "Going? Yes/No/Maybe" es propia (la referencia son copias del organizador).
