# 06 — Integración con Ubuntu y GNOME

Fuentes: `docs/research/eds-gnome.md`, `docs/research/tauri.md`. Todo nombre de bus, interfaz y método D-Bus mencionado acá fue verificado por introspección en la máquina del usuario con EDS 3.56 y GNOME Shell 48.

## 1. Notificaciones de recordatorio

Se usa `notify-rust` directo, backend `zbus`, sobre `org.freedesktop.Notifications`. No se usa `tauri-plugin-notification` porque no soporta botones de acción en escritorio.

Notificación de recordatorio:

```rust
Notification::new()
    .appname("Unified Google Calendar")
    .summary("Daily standup in 10 min")          // "<title> in <n> min" o "<title> now"
    .body("09:30 – 09:45 · Greelow")             // hora local · nombre de cuenta
    .icon("x-office-calendar")                   // hasta que exista el ícono propio instalado en hicolor
    .action("open", "Open")
    .action("join", "Join")                      // solo si hay Meet o link de conferencia
    .action("snooze", "Snooze 5 min")
    .hint(Hint::Resident(true))
    .timeout(Timeout::Never)
    .show()?
```

- `open` emite `window:show-event` con el `occurrence_id`, muestra la ventana y la UI abre el popup del evento.
- `join` abre `hangout_link` o el primer `entryPoints[].uri` de tipo `video` con el navegador del sistema.
- `snooze` reprograma el recordatorio a `now + 5 min` en memoria.
- `wait_for_action_async` corre en una tarea de tokio, nunca en el hilo principal.
- Una notificación por ocurrencia y recordatorio. Si el usuario tiene dos recordatorios en el mismo evento, son dos notificaciones.

Desktop file: `X-GNOME-UsesNotifications=true` para que GNOME agrupe las notificaciones bajo la app.

## 2. Bandeja

Feature `tray-icon` de Tauri con libayatana. Compilar con `TAURI_LINUX_AYATANA_APPINDICATOR=1`. La extensión `ubuntu-appindicators` ya está activa en la máquina del usuario.

Limitaciones en GNOME, verificadas en `docs/research/tauri.md`:

- No hay evento de click izquierdo. Cualquier click abre el menú.
- No hay tooltip.
- `set_title` sí funciona y muestra texto al lado del ícono.

Menú:

```
Next: Daily standup · 09:30        (deshabilitado, informativo)
──────────
Open Unified Google Calendar
Sync now
──────────
Quit
```

`set_title` se actualiza con el próximo evento no de día completo de hoy (zona primaria), formato `"09:30 Daily standup"`, truncado a 32 caracteres. Un evento empezado sigue en el título hasta dos minutos después de su inicio; después pasa al siguiente. Sin más eventos hoy, título vacío hasta mañana (cambio del 2026-09-15, `docs/99`; antes: cualquier evento dentro de las próximas 12 horas, y desaparecía al empezar). Se recalcula tras cada `calendar:updated` y cada minuto. Cada recálculo alterna un espacio de ancho cero (U+200B) al final del título (`tray::tick_title`): la extensión AppIndicator de GNOME solo repinta la etiqueta cuando el valor recibido difiere del que tiene en caché, y el 2026-09-17 se vio la etiqueta en el bus (`XAyatanaLabel` del `StatusNotifierItem`) y ausente en el panel hasta que cambió la reunión; con un valor distinto por minuto el panel repinta siempre a más tardar un minuto después.

## 3. Cerrar deja la app corriendo

```rust
.on_window_event(|window, event| {
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
})
.run(|_app, event| {
    if let RunEvent::ExitRequested { api, .. } = event { api.prevent_exit(); }
});
```

"Quit" del menú de bandeja llama `app.exit(0)`. `tauri-plugin-single-instance` es el primer plugin registrado: si el usuario lanza la app otra vez desde el dock, la instancia existente muestra y enfoca su ventana.

El WebKitWebProcess no se libera al ocultar la ventana. Es una limitación conocida y aceptada en la versión 1.

## 4. Espejo en el panel de GNOME Shell

### 4.1 Cómo funciona el panel

`gnome-shell-calendar-server` muestra toda fuente de Evolution Data Server que tenga sección `[Calendar]`, `Selected=true` y `Enabled=true` en la fuente y sus padres. No lee VALARM ni notifica nada. Las notificaciones "Events and Tasks Reminders" las genera `evolution-alarm-notify`, otro proceso, a partir de los VALARM.

### 4.2 Modelo

Una fuente local de EDS por cada calendario de la app, más una para los eventos personales:

- UID: `ugc-<sha1 hex de "<account_id>|<calendar_id>">` para Google, `ugc-local` para el calendario local.
- `DisplayName`: `"<summary> · <email>"` para Google, `"Personal (Unified Google Calendar)"` para local.
- Key-file:

```ini
[Data Source]
DisplayName=Aaron Feldman · aaron@greelow.com
Enabled=true
Parent=local-stub

[Calendar]
BackendName=local
Color=#0b8043
Selected=true
Order=0

[Alarms]
IncludeMe=false
ForEveryEvent=false

[Offline]
StaySynchronized=true
```

`[Alarms] IncludeMe=false` saca la fuente del watcher de `evolution-alarm-notify`. Además los VEVENT se escriben sin VALARM. Así la única que notifica es nuestra app.

### 4.3 Buses e interfaces

| Uso | Bus | Objeto | Interfaz |
|---|---|---|---|
| Registro de fuentes | `org.gnome.evolution.dataserver.Sources5` | `/org/gnome/evolution/dataserver/SourceManager` | `org.freedesktop.DBus.ObjectManager` y `org.gnome.evolution.dataserver.SourceManager` |
| Fuente individual | mismo | `/org/gnome/evolution/dataserver/SourceManager/Source_N` | `org.gnome.evolution.dataserver.Source` (propiedad `UID`, `Data`), `.Source.Writable` (`Write(s)`), `.Source.Removable` (`Remove()`) |
| Fábrica de calendarios | `org.gnome.evolution.dataserver.Calendar8` | `/org/gnome/evolution/dataserver/CalendarFactory` | `org.gnome.evolution.dataserver.CalendarFactory` (`OpenCalendar(s uid) -> (s path, s bus)`) |
| Calendario abierto | el `bus` devuelto | el `path` devuelto | `org.gnome.evolution.dataserver.Calendar` |

Los sufijos `Sources5` y `Calendar8` cambian entre versiones mayores de EDS. `eds::dbus` los descubre con `ListNames` filtrando por prefijo `org.gnome.evolution.dataserver.Sources` y `...Calendar`, y toma el de número más alto. Los nombres `Source_N` no son estables: se resuelven con `GetManagedObjects` buscando la propiedad `UID`.

### 4.4 Operaciones

Crear o actualizar fuente:

1. `GetManagedObjects` → mapa `UID → path`.
2. Si no existe: `SourceManager.CreateSources({uid: keyfile})`. Falla con `G_IO_ERROR_EXISTS` si el UID ya está.
3. Si existe y cambió nombre o color: `Source.Writable.Write(keyfile)` con el key-file completo.
4. Al quitar una cuenta o calendario: `Source.Removable.Remove()`.

Escribir eventos:

1. `CalendarFactory.OpenCalendar(uid)` → `(path, bus)`.
2. `Calendar.Open()` en ese path. Obligatorio, lo hace `ECalClient` internamente.
3. `Calendar.GetObjectList("#t")` → lista de VEVENT actuales. Parsear `UID` y `LAST-MODIFIED` para el diff.
4. `CreateObjects([vevent...], 0)`, `ModifyObjects([vevent...], "all", 0)`, `RemoveObjects([(uid, "")...], "all", 0)`.
5. `Calendar.Close()`.

Formato de VEVENT, todo en UTC para no tener que registrar VTIMEZONE:

```
BEGIN:VEVENT
UID:ugc-<account_id>-<event_id>
DTSTAMP:20260914T120000Z
DTSTART:20260915T130000Z
DTEND:20260915T140000Z
SUMMARY:Daily standup
LOCATION:...
LAST-MODIFIED:20260914T120000Z
SEQUENCE:3
END:VEVENT
```

Día completo: `DTSTART;VALUE=DATE:20260915` y `DTEND;VALUE=DATE:20260916`. Se escriben **ocurrencias**, no masters con RRULE. Así el diff es trivial y el panel no expande nada. El panel de Shell solo pide alrededor de un mes alrededor de la vista, así que se espejan ocurrencias entre hoy menos 30 días y hoy más 180 días.

### 4.5 Cuándo corre el espejo

- Tras cada `calendar:updated`, con debounce de 5 segundos, solo para los calendarios tocados.
- Una vez por día para mover la ventana de 30/180 días.
- Al quitar una cuenta: `Remove()` de sus fuentes. Los eventos se van con la fuente.
- Solo los calendarios visibles en la app tienen fuente. Al ocultar un calendario (interruptor del cajón o de Settings) el espejo hace `Remove()` de su fuente en la siguiente pasada; al volver a mostrarlo la crea de nuevo. El panel muestra exactamente lo que la app muestra.
- Si `evolution-source-registry` no está corriendo o el bus no responde, el espejo se salta y registra en el log. Nunca bloquea la app.

### 4.6 Evitar duplicados con Online Accounts

Las cuentas de Google en Online Accounts ya alimentan el panel con sus propios calendarios. Si la app también espeja, cada evento aparece dos veces. Solución: apagar Calendar en cada cuenta de Google de Online Accounts. La cuenta, el correo y los contactos siguen funcionando. Es reversible.

La app lo ofrece en el primer arranque con un diálogo "GNOME's calendar panel already shows N Google accounts. Turn off their Calendar in Online Accounts so events don't show twice?". Con Sí, ejecuta por `zbus`:

```
bus org.gnome.OnlineAccounts
objeto /org/gnome/OnlineAccounts/Accounts/<id>
interfaz org.gnome.OnlineAccounts.Account
set-property CalendarDisabled b true
```

Solo para cuentas con `ProviderType = google`. Es lo mismo que hace el interruptor en Settings → Online Accounts. Con No, se muestra el paso manual de `09-setup-usuario.md`.

### 4.7 Verificación

`scripts/check-eds.sh` lista las fuentes `ugc-*` con `gdbus`, abre una con `OpenCalendar`, cuenta objetos con `GetObjectList("#t")` y muestra con `gdbus monitor --dest org.gnome.Shell.CalendarServer` que llegan `EventsAddedOrUpdated` al cambiar el rango. Ese script es el criterio de aceptación de la fase de integración.

## 5. Dependencias en runtime

Solo `evolution-data-server`, que viene con GNOME y ya está instalado. No se instalan `gir1.2-ecal-2.0` ni ningún helper en Python. Todo va por `zbus`.
