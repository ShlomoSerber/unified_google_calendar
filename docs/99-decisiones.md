# 99 — Registro de decisiones y desvíos

Cada entrada: fecha, quién decidió (usuario, modelo de diseño, modelo de implementación), qué se decidió, por qué, y qué documento afecta. Las decisiones de diseño previas a la implementación están en los documentos 01 a 07. Acá van las que surgen después.

Formato:

```
## AAAA-MM-DD — Título corto
- Quién: usuario | implementación
- Fase/tarea: F4-T3
- Decisión: ...
- Motivo: ...
- Afecta: docs/03-modelo-de-datos.md sección 4
```

## 2026-09-14 — Cierre de la fase de diseño
- Quién: usuario y modelo de diseño
- Decisión: la implementación la hace un modelo distinto siguiendo `docs/` y el harness de `.claude/`. No se escribió código de producto en la fase de diseño.
- Motivo: costo.
- Afecta: todo.

## 2026-09-14 — Tokens en archivo cifrado propio, no en GNOME Keyring
- Quién: usuario
- Decisión: `tokens.bin` con AES-256-GCM y clave derivada de machine-id y uid.
- Motivo: menos dependencias del sistema. El usuario aceptó el límite de que un proceso del mismo usuario puede derivar la clave.
- Afecta: `02-arquitectura.md` sección 7.

## 2026-09-14 — Fuentes propietarias de Google en runtime
- Quién: modelo de diseño, pendiente de confirmación del usuario en la fase 4
- Decisión: Google Sans Flex y Roboto empaquetadas. Google Sans Text solo si la medición la muestra como fuente efectiva, cargada desde fonts.googleapis.com.
- Motivo: no redistribuir fuentes propietarias. App personal no distribuida.
- Afecta: `04-fidelidad-visual.md` sección 6.

## Mediciones de RAM por fase

| Fase | Fecha | Proceso Rust PSS | WebKitWebProcess PSS | Total | Nota |
|---|---|---|---|---|---|
| | | | | | |
