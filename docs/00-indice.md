# Unified Google Calendar — Índice de documentación

Este proyecto se implementa siguiendo estos documentos en orden. Quien implementa no toma decisiones de arquitectura: están tomadas acá. Si un documento y el código se contradicen, gana el documento y se abre una nota en `docs/99-decisiones.md`.

| Archivo | Contenido | Quién lo usa |
|---|---|---|
| `00-indice.md` | Este índice y las reglas de lectura. | Todos |
| `01-requisitos.md` | Qué tiene que hacer la app y qué no. Cerrado con el usuario. | Todos |
| `02-arquitectura.md` | Stack, procesos, módulos, flujo de datos, límites de RAM, árbol de directorios. | Implementación |
| `03-modelo-de-datos.md` | Esquema SQLite, mapeo con la Calendar API, estados de sincronización, recurrencias. | Implementación |
| `04-fidelidad-visual.md` | **Histórico desde el 2026-09-16.** Cómo se midió Google Calendar real y cómo se compararon capturas y animaciones. Ya no rige la UI: ver `11`. | Referencia |
| `05-sincronizacion.md` | OAuth, sync tokens, push por Tailscale Funnel, renovación de canales, fallback a polling, conflictos. | Implementación |
| `06-integracion-gnome.md` | Notificaciones, bandeja, cerrar a segundo plano, espejo en Evolution Data Server. | Implementación |
| `07-empaquetado.md` | Build del `.deb`, dependencias, instalación, desinstalación, primer arranque. | Implementación y usuario |
| `08-plan-de-implementacion.md` | Fases, tareas con criterios de aceptación, orden, qué verificar al cerrar cada fase. | Implementación |
| `09-setup-usuario.md` | Lo que hace el usuario a mano: proyecto en Google Cloud, Tailscale, Online Accounts. | Usuario |
| `10-mantenimiento.md` | Estado al cierre del plan, pendientes del usuario, limitaciones conocidas y cómo depurar sin molestar al usuario. | Mantenimiento |
| `11-material3.md` | Sistema de diseño Material 3 de la app: tokens, tipografía, íconos, integración de `@material/web` con React 19, mapeo componente a componente, reglas de CSS. Fuente de verdad visual desde el 2026-09-16. | Implementación de UI |
| `12-plan-m3.md` | Plan de la transición a Material 3: fases M0 a M6, tareas con criterios de aceptación, orden, gate de cierre. | Implementación |
| `99-decisiones.md` | Registro de decisiones y desvíos durante la implementación. | Todos |

## Harness de Claude Code

Además de `docs/`, el repositorio incluye lo que necesita un modelo para implementar sin ambigüedad:

- `CLAUDE.md`: reglas del proyecto, comandos, convenciones y el protocolo de trabajo por fase.
- `.claude/rules/`: reglas por área que se cargan según el archivo tocado.
- `.claude/skills/`: procedimientos repetibles: `start-task` (tareas de `08`), `close-phase`, `m3-transition` (ejecuta `12` entero).
- `scripts/`: verificaciones automáticas que cada fase debe pasar antes de darse por cerrada.

## Reglas de lectura para quien implementa

1. Leer `CLAUDE.md`, luego `01`, `02` y `08` completos antes de tocar código.
2. Cada tarea de `08` nombra los documentos que aplican. Leerlos antes de empezar la tarea.
3. Nunca inventar un valor de UI. Todo valor sale de `docs/design/m3/theme.json` (`11` sección 3): tokens de sistema M3, una página de especificación de m3.material.io citada, o la grilla de 4 px. Si falta, se agrega a `theme.json` con su nota y se regenera.
4. Nunca cambiar una decisión de `02`, `03` o `05` sin registrarla en `99-decisiones.md` con motivo.
5. Cerrar una fase solo cuando pasan sus criterios de aceptación y los scripts de `scripts/`.
