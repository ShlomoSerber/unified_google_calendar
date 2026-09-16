# Referencia histórica: calendar.google.com medido el 2026-09-14

Esta carpeta guarda las mediciones que se hicieron sobre calendar.google.com durante la fase de
réplica pixel a pixel (`docs/04-fidelidad-visual.md`, histórico): por componente, un JSON con el
árbol de nodos y sus estilos computados en claro y en oscuro (`<componente>-light.json`,
`<componente>-dark.json`), el mismo volcado tomado sobre la app de entonces (`*-app.json`), y una
nota `<componente>.md` con lo observado. `palette.json` es la paleta de colores de chips que Google
pintaba; `animations.md` y `scrollbars.md`, las transiciones y barras de desplazamiento medidas.

**No rige nada.** Desde el 2026-09-16 la app sigue el sistema Material 3 de
`docs/11-material3.md` y todo valor visual sale de `docs/design/m3/theme.json`
(`docs/99-decisiones.md`, entrada "Transición a Material 3 total"). Nadie lee estos archivos para
decidir un valor; se conservan como registro de cómo era Google Calendar ese día. Las capturas PNG
nunca estuvieron en git y se borraron del disco al cerrar la transición (M5-T1).
