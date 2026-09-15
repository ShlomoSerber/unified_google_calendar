# Snackbar (componente 24)

Fecha: 2026-09-15. Referencia: calendar.google.com, vista semana, cuenta de Greelow. Medido creando un evento "UGC probe" con la creación rápida y borrándolo desde el popup, con permiso explícito del usuario; el evento no existe más.

## Procedimiento

- Sonda ad hoc por CDP (misma base que `scripts/measure/animations.mjs`): tras "Save" y tras "Delete event" se buscó cada 120 ms el elemento con texto `Saving...`/`Event saved`/`Deleting...`/`Event deleted` en el borde inferior y se volcó con `dumpRegion`. Archivos `snackbar-save-light.json/.png`, `snackbar-save-dark.json/.png` (el volcado de `delete` capturó la barra anterior, "Event saved · Undo", y solo sirve de referencia del botón Undo).
- Tokens: `docs/design/token-spec.json` componente `snackbar`; `layout.snackbar_bottom`; `component.motion.snackbar_*`.

## Resultado

| Tema | Caja | Fondo | Texto |
|---|---|---|---|
| light | 288×48 px mínimo, centrada (x 576 en 1440), pegada al borde inferior, radio 4 px arriba, sombra de tres capas | `rgb(48, 48, 48)` | `rgb(242, 242, 242)`, línea 20 px, padding 14/16 px; botón de cierre de 36 px a la derecha |
| dark | idem | `rgb(227, 227, 227)` | `rgb(48, 48, 48)` |

Secuencia: "Saving..." aparece al instante (y 900 → 852 en unos 250 ms) y pasa a "Event saved · Undo" cuando la petición vuelve (~880 ms en la prueba); "Deleting..." → "Event deleted · Undo" igual (~730 ms).

## Diferencias aceptadas

- Sin "Undo" en la app (fuera de alcance). La barra final se mantiene 5 s (no medido).
- Sin comparación automática con la app: la barra depende del reloj de la petición.
