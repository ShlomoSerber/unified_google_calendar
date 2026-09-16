# Scrollbars (componente 20)

Fecha: 2026-09-14. Sin volcado JSON: los pseudoelementos `::-webkit-scrollbar` no salen en `dumpRegion`; se sondearon con `getComputedStyle(el, '::-webkit-scrollbar')` y `'::-webkit-scrollbar-thumb'` en claro y oscuro (`scripts/measure/session.mjs`). Valores en `docs/design/tokens.json` (`component.scrollbar.*`, con nota).

## Resultado

- Grilla de horas: barra de 16 px, pulgar `rgb(227, 227, 227)` / `rgb(71, 71, 71)` con borde de 4 px del color de superficie y radio 8. Cajón: barra de 8 px, pulgar del mismo gris, radio 8. En la referencia son barras superpuestas (`clientWidth = offsetWidth`, no se ven en las capturas).
- WebKitGTK ignora las reglas `::-webkit-scrollbar` salvo que se le pida una barra clásica, que entonces ocupa ancho (la barra del cajón de 8 px ensanchaba la barra lateral a 264 px). Por eso la app deja las barras sin estilizar: GTK dibuja su barra superpuesta al hacer scroll, que es el comportamiento más cercano al de Google. La columna de horas se sincroniza con la grilla y nunca muestra barra.
