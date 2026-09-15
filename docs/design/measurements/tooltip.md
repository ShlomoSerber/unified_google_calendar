# Tooltip (componente 21)

Fecha: 2026-09-15. Referencia: calendar.google.com, vista semana, cuenta de Greelow. El tooltip visible que Google renderiza en un portal (`div` fijo) 540 ms después de que el puntero llega a un botón del header; distinto del `[role=tooltip]` de 1×1 px a −10000 px que la cabecera conserva y que ya está en el dump `topbar`.

## Procedimiento

- Google: `node scripts/measure/capture.mjs tooltip` (puntero sobre "Next week" durante 1,2 s). Archivos `tooltip-<tema>.json/.png`.
- App: sin dump propio. El tooltip de la app (`src/app/Tooltip.tsx`) usa las reglas `.tooltip-*` generadas del dump y se posiciona por JS bajo el botón (`layout.tooltip_gap`, 4 px) centrado en él.
- Tiempos: `docs/design/measurements/animations.md` (`tooltip_*`).

## Resultado

| Tema | Nodos | Caja | Fondo | Texto |
|---|---|---|---|---|
| light | 3 | 73,1 × 24 px, 4 px bajo el botón, centrado | `rgb(48, 48, 48)` | `rgb(242, 242, 242)`, 12 px / 16 px, `Google Sans Flex` |
| dark | 3 | idem | `rgb(227, 227, 227)` | `rgb(48, 48, 48)` |

## Diferencias aceptadas

- No hay comparación automática con la app (`report.mjs`): el tooltip depende del puntero y del reloj. La verificación fue visual en Xvfb (captura a 1 s del hover).
