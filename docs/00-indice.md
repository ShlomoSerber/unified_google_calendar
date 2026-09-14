# Unified Google Calendar — Índice de documentación

Este proyecto se implementa siguiendo estos documentos en orden. Quien implementa no toma decisiones de arquitectura: están tomadas acá. Si un documento y el código se contradicen, gana el documento y se abre una nota en `docs/99-decisiones.md`.

| Archivo | Contenido | Quién lo usa |
|---|---|---|
| `00-indice.md` | Este índice y las reglas de lectura. | Todos |
| `01-requisitos.md` | Qué tiene que hacer la app y qué no. Cerrado con el usuario. | Todos |
| `02-arquitectura.md` | Stack, procesos, módulos, flujo de datos, límites de RAM, árbol de directorios. | Implementación |
| `03-modelo-de-datos.md` | Esquema SQLite, mapeo con la Calendar API, estados de sincronización, recurrencias. | Implementación |
| `04-fidelidad-visual.md` | Cómo medir Google Calendar real, tokens de diseño, fuentes, método de comparación de capturas. | Implementación de UI |
| `05-sincronizacion.md` | OAuth, sync tokens, push por Tailscale Funnel, renovación de canales, fallback a polling, conflictos. | Implementación |
| `06-integracion-gnome.md` | Notificaciones, bandeja, cerrar a segundo plano, espejo en Evolution Data Server. | Implementación |
| `07-empaquetado.md` | Build del `.deb`, dependencias, instalación, desinstalación, primer arranque. | Implementación y usuario |
| `08-plan-de-implementacion.md` | Fases, tareas con criterios de aceptación, orden, qué verificar al cerrar cada fase. | Implementación |
| `09-setup-usuario.md` | Lo que hace el usuario a mano: proyecto en Google Cloud, Tailscale, Online Accounts. | Usuario |
| `99-decisiones.md` | Registro de decisiones y desvíos durante la implementación. | Todos |

## Harness de Claude Code

Además de `docs/`, el repositorio incluye lo que necesita un modelo para implementar sin ambigüedad:

- `CLAUDE.md`: reglas del proyecto, comandos, convenciones y el protocolo de trabajo por fase.
- `.claude/rules/`: reglas por área que se cargan según el archivo tocado.
- `.claude/skills/`: procedimientos repetibles, por ejemplo medir un componente de Google Calendar o cerrar una fase.
- `scripts/`: verificaciones automáticas que cada fase debe pasar antes de darse por cerrada.

## Reglas de lectura para quien implementa

1. Leer `CLAUDE.md`, luego `01`, `02` y `08` completos antes de tocar código.
2. Cada tarea de `08` nombra los documentos que aplican. Leerlos antes de empezar la tarea.
3. Nunca inventar una medida de UI. Si falta en `04`, medirla con el procedimiento de `04` y agregarla al archivo de tokens.
4. Nunca cambiar una decisión de `02`, `03` o `05` sin registrarla en `99-decisiones.md` con motivo.
5. Cerrar una fase solo cuando pasan sus criterios de aceptación y los scripts de `scripts/`.
