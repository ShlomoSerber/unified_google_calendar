# 03 — Modelo de datos

SQLite, un archivo `~/.local/share/unified-google-calendar/data.db`, modo WAL, `synchronous=NORMAL`, `foreign_keys=ON`. Todo timestamp es entero Unix en segundos UTC. Toda fecha de día completo es texto `YYYY-MM-DD`. Todo id de texto de Google se guarda tal cual.

## 1. Esquema

```sql
-- migración 0001
CREATE TABLE accounts (
  id            TEXT PRIMARY KEY,           -- 'local' o el `sub` del id_token de Google
  kind          TEXT NOT NULL,              -- 'local' | 'google'
  email         TEXT,                       -- NULL para local
  display_name  TEXT NOT NULL,              -- 'This computer' para local
  sort_order    INTEGER NOT NULL DEFAULT 0,
  sync_state    TEXT NOT NULL DEFAULT 'idle', -- 'idle'|'syncing'|'error'|'auth_required'
  sync_error    TEXT,
  last_sync_at  INTEGER,
  calendar_list_sync_token TEXT,
  created_at    INTEGER NOT NULL
);

CREATE TABLE calendars (
  id            TEXT NOT NULL,              -- calendarId de Google o uuid para local
  account_id    TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  summary       TEXT NOT NULL,
  description   TEXT,
  color_bg      TEXT NOT NULL,              -- hex de calendarListEntry.backgroundColor o elegido
  color_fg      TEXT NOT NULL,
  access_role   TEXT NOT NULL,              -- 'owner'|'writer'|'reader'|'freeBusyReader'
  is_primary    INTEGER NOT NULL DEFAULT 0,
  visible       INTEGER NOT NULL DEFAULT 1, -- estado del checkbox en la barra lateral
  hidden_remote INTEGER NOT NULL DEFAULT 0, -- calendarListEntry.hidden
  deleted       INTEGER NOT NULL DEFAULT 0,
  time_zone     TEXT,
  default_reminders TEXT NOT NULL DEFAULT '[]', -- JSON [{method,minutes}]
  sync_token    TEXT,                       -- nextSyncToken de events.list
  full_sync_done INTEGER NOT NULL DEFAULT 0,
  sort_order    INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (account_id, id)
);

CREATE TABLE events (
  account_id    TEXT NOT NULL,
  calendar_id   TEXT NOT NULL,
  id            TEXT NOT NULL,              -- event.id de Google o uuid local
  ical_uid      TEXT,                       -- event.iCalUID; local: uuid@unified-google-calendar
  etag          TEXT,
  status        TEXT NOT NULL,              -- 'confirmed'|'tentative'|'cancelled'
  summary       TEXT,
  description   TEXT,
  location      TEXT,
  color_id      TEXT,                       -- event.colorId (1..11) o NULL
  start_ts      INTEGER,                    -- NULL si all_day
  end_ts        INTEGER,
  start_date    TEXT,                       -- solo all_day
  end_date      TEXT,                       -- exclusivo, como Google
  all_day       INTEGER NOT NULL,
  time_zone     TEXT,                       -- IANA de start.timeZone
  recurrence    TEXT,                       -- JSON array de líneas RRULE/EXDATE/RDATE, NULL si no recurre
  recurring_event_id TEXT,                  -- master si esta fila es una excepción
  original_start_ts INTEGER,                -- originalStartTime de la excepción
  original_start_date TEXT,
  organizer_email TEXT,
  organizer_self INTEGER NOT NULL DEFAULT 0,
  attendees     TEXT NOT NULL DEFAULT '[]', -- JSON [{email,displayName,responseStatus,organizer,self,optional}]
  reminders     TEXT NOT NULL DEFAULT '{"useDefault":true}', -- JSON de event.reminders
  hangout_link  TEXT,
  conference    TEXT,                       -- JSON de conferenceData o NULL
  html_link     TEXT,
  transparency  TEXT,                       -- 'opaque'|'transparent'
  visibility    TEXT,
  event_type    TEXT NOT NULL DEFAULT 'default',
  guests_can_modify INTEGER NOT NULL DEFAULT 0,
  created_ts    INTEGER,
  updated_ts    INTEGER,
  raw           TEXT,                       -- JSON completo de Google para no perder campos al hacer PUT
  PRIMARY KEY (account_id, calendar_id, id),
  FOREIGN KEY (account_id, calendar_id) REFERENCES calendars(account_id, id) ON DELETE CASCADE
);
CREATE INDEX events_master ON events(account_id, calendar_id, recurring_event_id);
CREATE INDEX events_ical ON events(ical_uid);

-- Ocurrencias materializadas: una fila por instancia visible dentro de la ventana de datos.
CREATE TABLE occurrences (
  id            TEXT PRIMARY KEY,           -- account_id|calendar_id|event_id|start_ts (o start_date)
  account_id    TEXT NOT NULL,
  calendar_id   TEXT NOT NULL,
  event_id      TEXT NOT NULL,              -- fila de events que la genera (master o excepción)
  master_id     TEXT,                       -- id del master si viene de una recurrencia
  start_ts      INTEGER NOT NULL,           -- para all_day: medianoche UTC de start_date
  end_ts        INTEGER NOT NULL,
  all_day       INTEGER NOT NULL,
  status        TEXT NOT NULL,
  FOREIGN KEY (account_id, calendar_id, event_id) REFERENCES events(account_id, calendar_id, id) ON DELETE CASCADE
);
CREATE INDEX occ_range ON occurrences(start_ts, end_ts);
CREATE INDEX occ_event ON occurrences(account_id, calendar_id, event_id);

CREATE TABLE channels (
  id            TEXT PRIMARY KEY,           -- uuid que mandamos en watch
  account_id    TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  calendar_id   TEXT,                       -- NULL para el canal de calendarList
  resource_id   TEXT NOT NULL,
  token         TEXT NOT NULL,              -- secreto aleatorio, base64url 32 bytes
  expiration_ts INTEGER NOT NULL,
  created_at    INTEGER NOT NULL
);

CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL                       -- JSON
);
-- claves: primary_tz, secondary_tz, week_start, hour_format, theme, default_calendar,
--         data_window_past_days (365), data_window_future_days (730), webhook_port (8080),
--         public_base_url (https://<pc>.<tailnet>.ts.net), push_enabled (bool)

CREATE TABLE sync_log (
  id        INTEGER PRIMARY KEY AUTOINCREMENT,
  at        INTEGER NOT NULL,
  account_id TEXT,
  calendar_id TEXT,
  kind      TEXT NOT NULL,                  -- 'full'|'incremental'|'push'|'poll'|'wake'|'error'
  detail    TEXT
);
```

La cuenta `local` y su calendario `local-personal` se crean en la migración 0001. El calendario de feriados no es local: se suscribe en la cuenta Gmail del usuario con `calendarList.insert` de `en.ar#holiday@group.v.calendar.google.com`. Ver `09-setup-usuario.md`.

## 2. Mapeo con la Calendar API

| Google | Tabla.columna | Regla |
|---|---|---|
| `calendarListEntry.id` | `calendars.id` | |
| `calendarListEntry.backgroundColor` | `calendars.color_bg` | Es el hex real elegido por el usuario. No usar `colorId`. |
| `calendarListEntry.selected` | `calendars.visible` | Solo en la primera importación. Después manda el usuario en la app. |
| `event.start.dateTime` | `start_ts` | Parsear RFC3339 con offset y convertir a UTC. |
| `event.start.timeZone` | `time_zone` | Obligatorio al escribir eventos recurrentes. Si Google no lo manda, usar `calendars.time_zone`. |
| `event.start.date` | `start_date`, `all_day=1` | `end.date` es exclusivo. Un evento de un día tiene `end_date = start_date + 1`. |
| `event.recurrence[]` | `recurrence` | Guardar líneas tal cual. |
| `event.recurringEventId` | `recurring_event_id` | La fila es una excepción. |
| `event.status = cancelled` | `status` | Se guarda. Si es una excepción cancelada, la ocurrencia del master en `originalStartTime` no se materializa. Si es un master cancelado, se borran todas sus ocurrencias. |
| `event.colorId` | `color_id` | Se resuelve a hex con la paleta moderna de `04-fidelidad-visual.md`, nunca con `colors.get`. |
| `event.attendees[]` | `attendees` JSON | Al hacer `patch` se manda el array completo porque Google reemplaza arrays. |
| `event.hangoutLink` | `hangout_link` | Link directo de Meet. |
| `event.conferenceData` | `conference` JSON | `entryPoints[].uri` para el botón Join. |
| Todo el objeto | `raw` | Para `PUT` se parte de `raw`, se aplican cambios y se envía. Evita borrar campos desconocidos. |

## 3. Ventana de datos y expansión

- Ventana: desde hoy menos `data_window_past_days` hasta hoy más `data_window_future_days`. Se recalcula al arrancar y una vez por día. Al mover la ventana se materializan las ocurrencias nuevas y se borran las que quedaron fuera.
- Full sync usa `events.list` con `singleEvents=false`, `showDeleted=true`, `maxResults=2500`, sin `timeMin` ni `timeMax`, porque el sync token no admite filtros de tiempo. Se guarda todo lo que Google devuelve. Eventos fuera de la ventana no generan ocurrencias, pero sí filas en `events`.
- `recurrence::expand(master)`:
  1. Construir `DTSTART` a partir de `start_ts` y `time_zone` o `start_date`.
  2. Parsear `recurrence` con el crate `rrule`. Las líneas `EXDATE` y `RDATE` se aplican.
  3. Iterar instancias dentro de la ventana, máximo 5000 por master.
  4. Para cada instancia, si existe una excepción con `original_start_ts` igual, usar los datos de la excepción. Si esa excepción está `cancelled`, saltar.
  5. Duración de cada instancia = `end_ts - start_ts` del master.
  6. Reemplazar en una transacción todas las ocurrencias con `master_id = master.id`.
- Eventos no recurrentes generan exactamente una ocurrencia, si caen dentro de la ventana.
- Zonas: la expansión se hace en la zona `time_zone` del master, para que un evento semanal a las 10:00 siga a las 10:00 aunque cambie el horario de verano. Luego se convierte a UTC.

## 4. Edición de recurrencias

`scope` es `this`, `following` o `all`. Reglas, tomadas de `docs/research/google-calendar-api.md` sección 5:

| Scope | Google | Local |
|---|---|---|
| `this` | `events.update` sobre el id de instancia `masterId_YYYYMMDDTHHMMSSZ` con los cambios. Google crea la excepción. Guardarla como fila con `recurring_event_id`. | Insertar fila excepción con `recurring_event_id` y `original_start_ts`. |
| `following` | Dos llamadas: `update` del master con `RRULE;UNTIL=<instante anterior a la instancia, en UTC, formato YYYYMMDDTHHMMSSZ>`, y `insert` de un master nuevo que empieza en la instancia con la RRULE original sin `COUNT`. Si la RRULE tenía `COUNT`, recalcular el restante. Google borra las excepciones posteriores; la app también. | Igual, sobre la tabla local. |
| `all` | `update` del master. | Actualizar el master. |
| borrar `this` | `events.delete` del id de instancia. Google la marca `cancelled`. | Insertar excepción `cancelled`. |
| borrar `following` | `update` del master con `UNTIL`. | Igual. |
| borrar `all` | `events.delete` del master. | Borrar master, excepciones y ocurrencias en cascada. |

El id de instancia de Google se construye como `master.id + "_" + start en UTC con formato YYYYMMDDTHHMMSSZ`. Para masters de día completo el sufijo es `YYYYMMDD`. Si en la implementación aparece un caso donde el sufijo no coincide, se usa `events.instances` con `originalStart` para obtener el id real y se registra en `99-decisiones.md`.

## 5. Deduplicación entre cuentas

Un evento con invitados aparece en cada cuenta invitada con el mismo `ical_uid` y distinto `id`. Regla para `get_view`:

1. Agrupar ocurrencias por `(ical_uid, start_ts)`.
2. Elegir una para mostrar: primero la copia donde `organizer_self = 1`, si no la de la cuenta con menor `sort_order`.
3. La ocurrencia mostrada lleva `also_in: [account_id...]` con las otras cuentas.
4. Las copias no mostradas no cuentan para conflictos.

Los eventos locales tienen `ical_uid` propio con sufijo `@unified-google-calendar`, así nunca colisionan.

## 6. Conflictos

Dos ocurrencias no de día completo están en conflicto si se solapan en el tiempo y pertenecen a cuentas distintas, después de la deduplicación, y ninguna tiene `transparency = transparent` ni `status = cancelled`. `get_view` calcula `conflict_with: occurrence_id | null` por ocurrencia, tomando el primer conflicto por orden de inicio. No se persiste.

## 7. Mover entre cuentas

`move_event_account(event_id, target_calendar_id)`:

- Google a Google, misma cuenta: `events.move`. Solo `event_type = default`.
- Google a Google, distinta cuenta: `events.import` en destino con el mismo `iCalUID`, `sendUpdates=none`, `conferenceDataVersion=1`. Si responde 200, `events.delete` en origen con `sendUpdates=none`. Si el delete falla, se deja el duplicado y se informa.
- Local a Google: `events.insert` en destino con `iCalUID` local. Luego borrar la fila local.
- Google a local: copiar campos a una fila local nueva con `ical_uid` nuevo, borrar en Google con `sendUpdates=none`. Los invitados se pierden y la UI lo avisa antes de confirmar.
- Recurrentes: se mueve el master completo. `this` y `following` no aplican al mover en la versión 1.

## 8. Recordatorios

- Un evento con `reminders.useDefault = true` usa `calendars.default_reminders`. Si es `false`, usa `reminders.overrides`.
- Solo se notifican `method = popup`. Los `email` los manda Google.
- Local: mismo JSON. El calendario local tiene `default_reminders = [{"method":"popup","minutes":10}]`.
- `reminders::scheduler` consulta cada 60 s las ocurrencias con inicio entre ahora y ahora más 24 h, calcula `fire_at = start_ts - minutes*60`, y dispara las que caen en el próximo minuto y no fueron disparadas. Los disparos se registran en `settings` bajo `fired_reminders` como lista con expiración de 48 h, para no repetir tras reinicios.

## 9. Consultas que la UI necesita

`get_view(from, to, tz)` devuelve:

```json
{
  "from": 1789000000, "to": 1789604800,
  "occurrences": [
    {
      "id": "sub123|primary|abc|1789012345",
      "event_id": "abc", "calendar_id": "primary", "account_id": "sub123",
      "title": "Daily standup", "start": 1789012345, "end": 1789013245,
      "all_day": false, "color_bg": "#0b8043", "color_fg": "#ffffff",
      "status": "confirmed", "my_response": "accepted", "is_recurring": true,
      "has_meet": true, "attendee_count": 3, "conflict_with": null,
      "also_in": [], "is_local": false, "transparency": "opaque"
    }
  ],
  "calendars": [ { "account_id": "sub123", "id": "primary", "visible": true } ]
}
```

Solo se devuelven ocurrencias de calendarios visibles y con `status != cancelled`. El orden es por `start`, luego por duración descendente. La UI hace el layout de solapamiento por columna con esos datos.

`get_event(occurrence_id)` devuelve todo lo que muestra el popup: título, cuándo con texto de recurrencia legible, Meet o link de conferencia, ubicación, descripción, invitados con estado, organizador, recordatorios, cuenta y calendario, `also_in`, `conflict_with` expandido con título y horario, permisos (`can_edit`, `can_delete`, `can_rsvp`).

## 10. Migraciones

`db/migrations.rs` aplica archivos `NNNN_nombre.sql` embebidos con `include_str!`, en orden, registrando en `PRAGMA user_version`. Nunca se edita una migración aplicada. Un cambio de esquema es una migración nueva.
