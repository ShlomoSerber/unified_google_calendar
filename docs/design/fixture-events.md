# Eventos de prueba para medir y comparar

Se crean en un calendario secundario llamado `UGC Fixtures` dentro de la cuenta con la que se mide en Chrome. El mismo calendario se agrega a la app, así ambas capturas muestran el mismo contenido. Los demás calendarios se ocultan durante la medición.

La semana de referencia es la semana en curso al momento de medir. Los días se expresan relativos al lunes de esa semana. Horas en zona del sistema.

| Día | Hora | Título | Notas |
|---|---|---|---|
| Lun | todo el día | All-day one | chip de día completo |
| Lun | 09:00–09:15 | Fifteen | chip de 15 min |
| Lun | 10:00–10:30 | Thirty | chip de 30 min |
| Lun | 11:00–11:45 | Forty-five | chip de 45 min |
| Lun | 13:00–14:00 | Sixty | chip de 1 h |
| Lun | 15:00–16:30 | Ninety | chip de 90 min |
| Mar | todo el día | All-day two A | dos chips de día completo |
| Mar | todo el día | All-day two B | |
| Mar | 10:00–11:00 | Overlap A | dos solapados |
| Mar | 10:30–11:30 | Overlap B | |
| Mié | 14:00–15:00 | Triple A | tres solapados |
| Mié | 14:15–15:15 | Triple B | |
| Mié | 14:30–15:30 | Triple C | |
| Jue | 09:00–10:00 | With Meet | con Google Meet y descripción "Description line" y ubicación "Location text" |
| Jue | 11:00–12:00 | With guests | dos invitados de prueba de cuentas del usuario, uno aceptado, uno pendiente |
| Vie | 10:00–10:30 | Weekly repeat | repite semanal, para el diálogo de recurrencia |
| Vie | 12:00–13:00 | Tentative | el usuario respondió "Maybe" |
| Vie | 14:00–15:00 | Declined | el usuario respondió "No" |
| Sáb | 10:00–11:00 | Weekend | |

Para la vista mes, además, un día con 5 eventos de 30 minutos entre 08:00 y 12:00 llamados `Month 1` a `Month 5`, para que aparezca "+N more". Para la vista agenda no hace falta nada extra.

Los invitados de prueba deben ser cuentas del propio usuario. Nunca invitar a terceros.
