-- migración 0001 — esquema literal de docs/03-modelo-de-datos.md sección 1
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

-- La cuenta local y su calendario se crean en la migración 0001 (docs/03 sección 1, docs/08 F1-T1).
INSERT INTO accounts (id, kind, email, display_name, sort_order, created_at)
  VALUES ('local', 'local', NULL, 'This computer', 0, strftime('%s', 'now'));
INSERT INTO calendars (id, account_id, summary, color_bg, color_fg, access_role, is_primary, visible, default_reminders)
  VALUES ('local-personal', 'local', 'Personal', '#f4511e', '#ffffff', 'owner', 1, 1, '[{"method":"popup","minutes":10}]');
