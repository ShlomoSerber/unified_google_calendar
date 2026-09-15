# Formulario completo (componente 16)

Fecha: 2026-09-14. Referencia: calendar.google.com, semana del 2026-09-14, cuenta de Greelow con solo "UGC Fixtures" visible (`docs/99`, F4-T1).

## Procedimiento

- Google: `node scripts/measure/capture.mjs full_form` (perfil `~/.chrome-measure`, Chrome headless por CDP, 1440×900, DPR 1; tema oscuro con `prefers-color-scheme` emulado). Archivos `full_form[-estado]-<tema>.json/.png`.
- App: `node scripts/measure/app-capture.mjs full_form` (Xvfb `:150`, binario de desarrollo, `POST /dev/measure`). Archivos `full_form[-estado]-<tema>-app.json/.png`.
- Comparación: `node scripts/measure/report.mjs` (equivale a `diff-layout.mjs --tolerance 1 --ignore-hidden` más `odiff --antialiasing --threshold 0.1`). Las diferencias admitidas por componente están en `ALLOWED` de `report.mjs` y se justifican abajo.

## Resultado

| Estado | Tema | Nodos comparados | Diferencias de layout | odiff |
|---|---|---|---|---|
| full_form | light | 66 | 12 | 1.19% |
| full_form | dark | 66 | 12 | 1.33% |

## Diferencias aceptadas

- Página de edición completa: título, fechas y horas, todo el día, recurrencia (menú y "Custom..." → componente 18), Meet, ubicación, notificaciones, calendario, color del evento, mostrar como, visibilidad, descripción, invitados y permisos. "Find a time", Drive, el formato del texto, las notas de reunión y los permisos de invitados quedan inertes (fuera de alcance).
- Diferencias restantes: anchos de texto (métrica de fuente); 2 px en "Add notification"; el calendario propuesto (ver quick_create).
