# Integración con el panel de calendario de GNOME Shell 48 vía Evolution Data Server (EDS)

Máquina analizada: Ubuntu 25.04, GNOME Shell 48.0, evolution-data-server 3.56.0-1, gnome-online-accounts 3.54.1,
python3-gi 3.50.0 instalado, `gir1.2-ecal-2.0` / `gir1.2-edataserver-1.2` / `gir1.2-ical-3.0` NO instalados (candidatos 3.56.0-1 / 3.0.19-4).
Toda la exploración fue de solo lectura. Fuentes verificadas: código de EDS tag 3.56.0, gnome-shell rama gnome-48, GOA 3.54.1, e introspección D-Bus en vivo.

## 0. Nombres de bus reales en esta máquina (busctl --user list)

```
org.gnome.Evolution-alarm-notify              4623 evolution-alarm-notify
org.gnome.evolution.dataserver.AddressBook10  4808 evolution-addressbook-factory
org.gnome.evolution.dataserver.Calendar8      4758 evolution-calendar-factory
org.gnome.evolution.dataserver.Sources5       4457 /usr/libexec/evolution-source-registry
org.gnome.evolution.dataserver.UserPrompter0  (activable)
org.gnome.Shell.CalendarServer                4450 gnome-shell-calendar-server
```

## 1. Cómo decide GNOME Shell qué calendarios mostrar

El panel no lee EDS desde JS: lo hace el helper `gnome-shell-calendar-server` (bus `org.gnome.Shell.CalendarServer`,
objeto `/org/gnome/Shell/CalendarServer`, métodos `SetTimeRange(x since, x until, b force_reload)`, señales
`EventsAddedOrUpdated(a(ssxxa{sv}))`, `EventsRemoved(as)`, `ClientDisappeared(s)`, propiedad `HasCalendars`).

Filtro exacto en `src/calendar-server/calendar-sources.c` (gnome-48):

```c
static gboolean
registry_watcher_filter_cb (ESourceRegistryWatcher *watcher, ESource *source, CalendarSources *sources)
{
  return e_source_has_extension (source, E_SOURCE_EXTENSION_CALENDAR) &&
         e_source_selectable_get_selected (e_source_get_extension (source, E_SOURCE_EXTENSION_CALENDAR));
}
...
sources->registry_watcher = e_source_registry_watcher_new (registry, NULL);
...
client = e_cal_client_connect_sync (source, source_type /* E_CAL_CLIENT_SOURCE_TYPE_EVENTS */, ...);
```

Condiciones para aparecer en el panel:
1. La fuente tiene sección `[Calendar]` (extensión `Calendar`).
2. `[Calendar] Selected=true` (ESourceSelectable; el valor por defecto de la propiedad `selected` es TRUE en `e-source-selectable.c`).
3. La fuente y todos sus padres están `Enabled=true` — lo aplica `ESourceRegistryWatcher` internamente con
   `e_source_registry_check_enabled()` (`e-source-registry-watcher.c` líneas 198/229), por eso no aparece en el filtro de Shell.

Shell no muestra alarmas ni recordatorios (no hay ninguna referencia a VALARM en calendar-sources.c ni en gnome-shell-calendar-server.c).

Cómo cambiar `Selected`: reescribir el key-file completo de la fuente con `org.gnome.evolution.dataserver.Source.Writable.Write(s data)`
(ver 2a) o con libedataserver `e_source_selectable_set_selected()` + `e_source_write_sync()`. Evolution/gnome-calendar hacen lo mismo.

## 2. Crear un calendario local ("En este equipo")

### Formato del key-file (copiado de `~/.config/evolution/sources/system-calendar.source`, el "Personal")

```ini
[Data Source]
DisplayName=Personal
Enabled=true
Parent=local-stub

[Calendar]
BackendName=local
Color=#62a0ea
Selected=true
Order=0
```
(seguido de `[Offline] StaySynchronized=true` y `[Refresh] Enabled=true / EnabledOnMeteredNetwork=true / IntervalMinutes=30`).

Para nuestro caso, el mínimo recomendado (la sección `[Alarms]` se explica en el punto 4):

```ini
[Data Source]
DisplayName=Trabajo (cuenta@gmail.com)
Enabled=true
Parent=local-stub

[Calendar]
BackendName=local
Color=#16a765
Selected=true
Order=0

[Alarms]
IncludeMe=false
ForEveryEvent=false

[Offline]
StaySynchronized=true
```

El backend `local` guarda los eventos en `~/.local/share/evolution/calendar/<uid>/calendar.ics`. El UID es el nombre del archivo
sin `.source` (Evolution usa 40 hex; cualquier cadena de `[A-Za-z0-9_-]` sirve, p.ej. `ugc-<sha1(account+calendarId)>`).

### 2a. D-Bus directo (verificado por introspección en vivo)

```
gdbus introspect --session --dest org.gnome.evolution.dataserver.Sources5 --object-path /org/gnome/evolution/dataserver/SourceManager
  interface org.freedesktop.DBus.ObjectManager { GetManagedObjects(out a{oa{sa{sv}}}); InterfacesAdded; InterfacesRemoved }
  interface org.gnome.evolution.dataserver.SourceManager {
    methods:
      CreateSources(in  a{ss} array);
      Reload();
      RefreshBackend(in  s source_uid);
  };
  node Source_19 {...}  node Source_24 {...}  ...   (un nodo por fuente, nombre NO estable entre reinicios)

gdbus introspect ... --object-path /org/gnome/evolution/dataserver/SourceManager/Source_16   (uid system-calendar)
  interface org.gnome.evolution.dataserver.Source {
    methods: InvokeCredentialsRequired(...); GetLastCredentialsRequiredArguments(...);
             UnsetLastCredentialsRequiredArguments(); InvokeAuthenticate(in as credentials);
    signals: CredentialsRequired(...); Authenticate(as credentials);
    properties: readonly s UID = 'system-calendar';  readonly s Data = '\n[Data Source]\nDisplayName...';  readwrite s ConnectionStatus = '';
  };
  interface org.gnome.evolution.dataserver.Source.Writable { methods: Write(in s data); };
```

`CreateSources(a{ss})`: diccionario `uid -> contenido completo del key-file`. Handler en `e-source-registry-server.c`
(`source_registry_server_create_source`): valida el key-file con `g_key_file_load_from_data`, falla con `G_IO_ERROR_EXISTS`
("UID “%s” is already in use") si el UID existe, escribe `~/.config/evolution/sources/<uid>.source` (`G_FILE_CREATE_PRIVATE`) y lo carga
con `E_SOURCE_PERMISSION_WRITABLE | E_SOURCE_PERMISSION_REMOVABLE` ("New sources are always writable + removable").
El objeto D-Bus aparece vía `ObjectManager.InterfacesAdded` con las interfaces `Source`, `Source.Writable` y `Source.Removable` (`Remove()`).

Alternativa sin D-Bus: escribir el archivo directamente en `~/.config/evolution/sources/`. El registro tiene un `GFileMonitor` sobre ese
directorio con debounce de 3 s (`source_registry_server_process_file_monitor_event`): CREATED → crea la fuente writable+removable;
CHANGED → `e_server_side_source_load()` recarga; DELETED → la elimina si es removable. Funciona, pero CreateSources/Write es síncrono y da error.

Nota: `system-calendar` no exporta `Source.Removable` en esta máquina porque es una fuente builtin embebida como GResource en
`/usr/libexec/evolution-source-registry` (`system-calendar.source` aparece en `strings`) y cargada sólo con WRITABLE, que gana sobre la
copia del usuario (comentario en `e_source_registry_server_load_file`). Nuestras fuentes con UID propio sí serán removibles.

Comandos (NO ejecutados; escriben):

```bash
# crear
gdbus call --session --dest org.gnome.evolution.dataserver.Sources5 \
  --object-path /org/gnome/evolution/dataserver/SourceManager \
  --method org.gnome.evolution.dataserver.SourceManager.CreateSources \
  "{'ugc-work-1234': '[Data Source]\nDisplayName=Trabajo\nEnabled=true\nParent=local-stub\n\n[Calendar]\nBackendName=local\nColor=#16a765\nSelected=true\n\n[Alarms]\nIncludeMe=false\nForEveryEvent=false\n'}"

# localizar el object path por UID (los nombres Source_N no son estables)
gdbus call --session --dest org.gnome.evolution.dataserver.Sources5 \
  --object-path /org/gnome/evolution/dataserver/SourceManager \
  --method org.freedesktop.DBus.ObjectManager.GetManagedObjects     # buscar UID == 'ugc-work-1234'

# actualizar (p.ej. Selected/Color/DisplayName): se reescribe el key-file completo
gdbus call --session --dest org.gnome.evolution.dataserver.Sources5 \
  --object-path /org/gnome/evolution/dataserver/SourceManager/Source_33 \
  --method org.gnome.evolution.dataserver.Source.Writable.Write "$(cat nuevo.source)"

# borrar
gdbus call ... --object-path .../Source_33 --method org.gnome.evolution.dataserver.Source.Removable.Remove
```

### 2b. libecal/libedataserver vía PyGObject (requiere gir1.2-ecal-2.0 + gir1.2-edataserver-1.2 + gir1.2-ical-3.0)

Firmas C verificadas en 3.56.0: `e_source_registry_new_sync(cancellable)`, `e_source_new_with_uid(uid, main_context, error)`,
`e_source_set_parent`, `e_source_set_display_name`, `e_source_get_extension(source, "Calendar")`,
`e_source_registry_commit_source_sync(registry, source, cancellable, error)` (si la fuente no existe llama a
`e_dbus_source_manager_call_create_sources_sync`; si existe, `Write`), `e_source_remove_sync`, `e_source_write_sync`.

```python
import gi
gi.require_version('EDataServer', '1.2'); gi.require_version('ECal', '2.0'); gi.require_version('ICalGLib', '3.0')
from gi.repository import EDataServer, ECal, ICalGLib

registry = EDataServer.SourceRegistry.new_sync(None)

def ensure_calendar(uid, name, color):
    src = registry.ref_source(uid)
    if src is None:
        src = EDataServer.Source.new_with_uid(uid, None)
        src.set_parent('local-stub')
    src.set_display_name(name)
    cal = src.get_extension(EDataServer.SOURCE_EXTENSION_CALENDAR)   # 'Calendar'
    cal.set_backend_name('local'); cal.set_color(color); cal.set_selected(True)
    al = src.get_extension(EDataServer.SOURCE_EXTENSION_ALARMS)      # 'Alarms'
    al.set_include_me(False); al.set_for_every_event(False)
    registry.commit_source_sync(src, None)     # CreateSources o Write según corresponda
    return src

def remove_calendar(uid):
    src = registry.ref_source(uid)
    if src: src.remove_sync(None)
```

### 2c. Crates Rust

Búsqueda en crates.io (`evolution-data-server`, `libecal`, `edataserver`, `ecal`, `evolution`): no existe ningún binding de EDS.
Los resultados `ecal`/`rustecal` son Eclipse eCAL (IPC) y `evolution*` es una herramienta de archivos. Opciones reales desde Rust:
`zbus` 5.19 (D-Bus puro) o generar bindings con `gir` (gtk-rs) sobre `ECal-2.0.gir`, lo cual es un proyecto en sí mismo.
Para generar VEVENT sirve el crate `icalendar` 0.17.

## 3. Añadir / modificar / borrar eventos

### Interfaz D-Bus (introspección en vivo, bus Calendar8)

```
/org/gnome/evolution/dataserver/CalendarFactory
  interface org.gnome.evolution.dataserver.CalendarFactory {
    OpenCalendar(in s source_uid, out s object_path, out s bus_name);
    OpenTaskList(...); OpenMemoList(...);
  }

/org/gnome/evolution/dataserver/Subprocess/4758/N   (path devuelto por OpenCalendar; bus_name p.ej. :1.74)
  interface org.gnome.evolution.dataserver.Calendar {
    RetrieveProperties(out as properties);  Open(out as properties);  Close();  Refresh();
    CreateObjects(in as ics_objects, in u opflags, out as uids);
    ModifyObjects(in as ics_objects, in s mod_type, in u opflags);
    RemoveObjects(in a(ss) uid_rid_array, in s mod_type, in u opflags);
    ReceiveObjects(in s ics_object, in u opflags);  SendObjects(...);
    GetObject(in s uid, in s rid, out s ics_object);  GetObjectList(in s query, out as ics_objects);
    GetFreeBusy(...); GetAttachmentUris(...); DiscardAlarm(in s uid, in s rid, in s alarm_uid, in u opflags);
    GetTimezone(in s tz_id, out s tz_object);  AddTimezone(in s tz_object);  GetView(in s query, out o object_path);
    signals: Error(s); FreeBusyData(as);
    properties: Online, Revision, Writable, CacheDir, Capabilities, DefaultObject, CalEmailAddress, AlarmEmailAddress
  }
```

Detalles verificados en el código 3.56.0:
- El nombre de interfaz es `org.gnome.evolution.dataserver.Calendar` (sin sufijo); el sufijo `8` está sólo en el bus name `Calendar8`.
- `ECalClient` llama a `Open()` justo después de `OpenCalendar` (`e_dbus_calendar_call_open_sync`, e-cal-client.c:1084). Hay que hacerlo.
- `ics_objects` son componentes `VEVENT` sueltos (no VCALENDAR); el cliente los serializa con `e_cal_client_sanitize_comp_as_string`.
  Si el evento usa TZID hay que registrar la VTIMEZONE con `AddTimezone` o usar UTC (`DTSTART:20260915T130000Z`).
- `mod_type`: nicks de `ECalObjModType` unidos por `:` (`e-data-cal.c` hace `g_strsplit(in_mod_type, ":")` + `g_flags_get_value_by_nick`):
  `this` (1), `this-and-prior` (2), `this-and-future` (4), `all` (7), `only-this` (8). Para eventos no recurrentes usar `all` o `this`.
- `opflags` (`ECalOperationFlags`): `0` = NONE; `1` CONFLICT_FAIL, `2` CONFLICT_USE_NEWER, `4` CONFLICT_KEEP_SERVER, `8` WRITE_COPY,
  `16` DISABLE_ITIP_MESSAGE. Para un backend local basta `0`.
- `RemoveObjects` recibe `a(ss)` = (uid, rid); rid = `""` para el evento maestro.
- `GetObjectList` usa el S-expression de EDS, p.ej. `(occur-in-time-range? (make-time "20260901T000000Z") (make-time "20261001T000000Z"))` o `#t`.

```bash
# 1) abrir
gdbus call --session --dest org.gnome.evolution.dataserver.Calendar8 \
  --object-path /org/gnome/evolution/dataserver/CalendarFactory \
  --method org.gnome.evolution.dataserver.CalendarFactory.OpenCalendar 'ugc-work-1234'
#   -> ('/org/gnome/evolution/dataserver/Subprocess/4758/10', ':1.74')
OBJ=/org/gnome/evolution/dataserver/Subprocess/4758/10; DEST=:1.74
gdbus call --session --dest $DEST --object-path $OBJ --method org.gnome.evolution.dataserver.Calendar.Open

# 2) crear (sin VALARM)
gdbus call --session --dest $DEST --object-path $OBJ --method org.gnome.evolution.dataserver.Calendar.CreateObjects \
  "['BEGIN:VEVENT\r\nUID:gcal-abc123@ugc\r\nDTSTAMP:20260914T120000Z\r\nDTSTART:20260915T130000Z\r\nDTEND:20260915T140000Z\r\nSUMMARY:Reunión\r\nEND:VEVENT\r\n']" 0
# 3) modificar
gdbus call ... --method org.gnome.evolution.dataserver.Calendar.ModifyObjects "['BEGIN:VEVENT...SEQUENCE:1...END:VEVENT']" 'all' 0
# 4) borrar
gdbus call ... --method org.gnome.evolution.dataserver.Calendar.RemoveObjects "[('gcal-abc123@ugc','')]" 'all' 0
gdbus call ... --method org.gnome.evolution.dataserver.Calendar.Close
```

### libecal (PyGObject)

Firmas C: `e_cal_client_connect_sync(source, type, wait_seconds, cancellable, error)`, `e_cal_client_create_object_sync(client, icalcomp, opflags, &out_uid, ...)`,
`e_cal_client_modify_object_sync(client, icalcomp, mod, opflags, ...)`, `e_cal_client_remove_object_sync(client, uid, rid, mod, opflags, ...)`.

```python
src = registry.ref_source('ugc-work-1234')
client = ECal.Client.connect_sync(src, ECal.ClientSourceType.EVENTS, 30, None)
comp = ICalGLib.Component.new_from_string("BEGIN:VEVENT\r\nUID:gcal-abc123@ugc\r\n...END:VEVENT\r\n")
ok, uid = client.create_object_sync(comp, ECal.OperationFlags.NONE, None)
client.modify_object_sync(comp, ECal.ObjModType.ALL, ECal.OperationFlags.NONE, None)
client.remove_object_sync('gcal-abc123@ugc', None, ECal.ObjModType.ALL, ECal.OperationFlags.NONE, None)
```

Para verificar que Shell ve los eventos (solo lectura): `gdbus monitor --session --dest org.gnome.Shell.CalendarServer` y observar `EventsAddedOrUpdated`.

## 4. Recordatorios: quién notifica y cómo evitar duplicados

- GNOME Shell no genera recordatorios. La notificación "Events and Tasks Reminders" viene de
  `/usr/libexec/evolution-data-server/evolution-alarm-notify` (paquete `evolution-data-server`, autostart
  `/etc/xdg/autostart/org.gnome.Evolution-alarm-notify.desktop`, bus `org.gnome.Evolution-alarm-notify`, PID 4623 en esta sesión).
  Usa `EReminderWatcher` (`e-alarm-notify.c:1112 e_reminder_watcher_new`), que dispara por cada `VALARM` (`e_cal_util_generate_alarms_for_uid_sync`).
- Filtro de fuentes de `EReminderWatcher` (`e-reminder-watcher.c`):

```c
return !e_source_has_extension (source, E_SOURCE_EXTENSION_ALARMS) ||
    e_source_alarms_get_include_me (e_source_get_extension (source, E_SOURCE_EXTENSION_ALARMS));
```

  Es decir, una fuente sin `[Alarms]` SÍ recibe recordatorios (por defecto `include-me` = TRUE). Los calendarios de Google
  de esta máquina tienen `[Alarms] IncludeMe=true ForEveryEvent=false`.
- Dos capas para que nuestra app sea la única que notifique:
  1. Escribir los VEVENT sin `VALARM` (así nada dispara aunque el usuario cambie ajustes).
  2. Declarar `[Alarms] IncludeMe=false` `ForEveryEvent=false` en nuestras fuentes (excluye la fuente completa del watcher;
     `ForEveryEvent=true` haría notificar todo evento aunque no tenga alarma, con el valor de gsettings `defall-reminder-*`).
- gsettings relevantes (org.gnome.evolution-data-server.calendar, valores actuales): `notify-enable-display true`,
  `notify-enable-audio true`, `notify-with-tray true`, `notify-past-events false`, `defall-reminder-enabled false`.
  No hay que tocarlos; con (1)+(2) basta.

## 5. Desactivar "Calendario" de una cuenta GOA sin borrar la cuenta

- Introspección de `org.gnome.OnlineAccounts` (`/org/gnome/OnlineAccounts/Accounts/account_1750163003_0`):

```
interface org.gnome.OnlineAccounts.Account {
  methods: Remove(); EnsureCredentials(out i expires_in);
  properties: readonly s ProviderType = 'google'; readonly s Id = 'account_1750163003_0';
              readwrite b MailDisabled = false;  readwrite b CalendarDisabled = false;
              readwrite b ContactsDisabled = false; readwrite b FilesDisabled = false; ...
}
interface org.gnome.OnlineAccounts.Calendar { readonly b AcceptSslErrors; readonly s Uri; }
```

- Persistencia: goa-daemon conecta `notify::calendar-disabled` a `goa_util_account_notify_property_cb`, que hace
  `goa_utils_keyfile_set_boolean (account, key, !value)` → escribe `CalendarEnabled=false` en `~/.config/goa-1.0/accounts.conf`
  (tabla `provider_features_info[]` en goaprovider.c: `.property = "calendar-disabled"`).
- Efecto en EDS (`module-gnome-online-accounts.c`): `e_binding_bind_property (goa_account, "calendar-disabled",
  source_extension, "calendar-enabled", SYNC_CREATE | INVERT_BOOLEAN)` → `[Collection] CalendarEnabled=false` en la colección.
  `e-collection-backend.c` liga `calendar-enabled` de la colección a `enabled` de cada hijo calendario
  (`include_master_source_enabled_transform`), así que los calendarios Google pasan a `Enabled=false`: **desaparecen del panel
  de Shell** (el watcher sólo entrega fuentes habilitadas), pero la cuenta, el correo y los contactos siguen. No se borran las fuentes
  hijas (siguen en `~/.cache/evolution/sources/<collection>/`), sólo se deshabilitan; reversible.
- Desde línea de comandos (no ejecutado): no hay clave gsettings; es la propiedad D-Bus:

```bash
busctl --user set-property org.gnome.OnlineAccounts /org/gnome/OnlineAccounts/Accounts/account_1750163003_0 \
  org.gnome.OnlineAccounts.Account CalendarDisabled b true
```

  Es lo mismo que hace el interruptor "Calendario" en Ajustes → Cuentas en línea. Nuestra app puede hacerlo con zbus tras pedir permiso
  al usuario, o simplemente pedirle que lo desactive en Ajustes.

## 6. Arquitectura recomendada

Modelo: una fuente EDS local por (cuenta Google, calendario) + una para eventos personales locales. UID determinista
(`ugc-<hash(accountEmail|calendarId)>`, `ugc-local`), `DisplayName` = "<nombre calendario> · <cuenta>", `Color` = el de Google,
`Selected=true`, `[Alarms] IncludeMe=false`. Eventos espejo con `UID` = id de Google (+ sufijo), sin VALARM, `SEQUENCE`/`LAST-MODIFIED`
para detectar cambios; borrados vía `RemoveObjects`. Sincronización incremental: `GetObjectList('#t')` → mapa uid→ics para diff.
Google se deshabilita en GOA con `CalendarDisabled=true` (punto 5) para evitar duplicados en el panel. El usuario puede seguir
mostrando/ocultando un calendario nuestro desde gnome-calendar/Evolution (cambian `Selected`, y podemos leerlo por `Data`).

Comparativa de implementación desde Rust (Tauri 2):

| Opción | Pros | Contras |
|---|---|---|
| **zbus directo** (Sources5 + Calendar8) | Sin dependencias apt extra (sólo EDS ya instalado en cualquier GNOME); todo en el proceso Rust; async con tokio; 5 llamadas D-Bus. | Hay que generar los VEVENT (crate `icalendar`), manejar zonas horarias (usar UTC), y resolver `Source_N` con `GetManagedObjects`/`InterfacesAdded`. |
| Helper Python + PyGObject | API de alto nivel (`ECal.Client`, `ESourceRegistry`), libical incluida. | Requiere `apt install gir1.2-ecal-2.0 gir1.2-edataserver-1.2 gir1.2-ical-3.0` (no instalados aquí); proceso externo, IPC propio, empaquetado (.deb/Flatpak) más frágil. |
| Bindings gir en Rust | Nativo. | No existen crates; generar y mantener bindings de ECal/EDataServer/ICalGLib es costoso. |

Recomendación: **zbus**. El camino mínimo es:
1. `ObjectManager.GetManagedObjects` en Sources5 → mapa `UID → path`; suscribirse a `InterfacesAdded/Removed`.
2. `SourceManager.CreateSources({uid: keyfile})` para fuentes nuevas; `Source.Writable.Write(keyfile)` para renombrar/recolorear;
   `Source.Removable.Remove()` al quitar una cuenta.
3. `CalendarFactory.OpenCalendar(uid)` → `(path, bus_name)`; `Calendar.Open()`; `CreateObjects/ModifyObjects/RemoveObjects`; `Close()`.
4. Antes de la primera sincronización, ofrecer al usuario desactivar Calendario en GOA (`CalendarDisabled=true`) o hacerlo vía zbus.

Paquetes apt: en runtime sólo `evolution-data-server` (ya presente en GNOME; proporciona los tres servicios y `evolution-alarm-notify`).
Para el enfoque Python: `python3-gi gir1.2-ecal-2.0 gir1.2-edataserver-1.2 gir1.2-ical-3.0`. Para desarrollo/depuración: `libglib2.0-bin` (gdbus), `systemd` (busctl).
Riesgos: los sufijos de bus (`Sources5`, `Calendar8`, `AddressBook10`) cambian entre versiones mayores de EDS; detectarlos con
`ListNames` + prefijo `org.gnome.evolution.dataserver.Calendar`. El panel de Shell limita la ventana de tiempo (`Since/Until`, ~±1 mes
alrededor de la vista), así que basta con espejar un rango razonable (p.ej. -1/+6 meses).
