# Google Calendar API v3 — Investigación para app de escritorio multi-cuenta (Ubuntu)

Fecha: 2026-09-14. Todas las afirmaciones provienen de documentación consultada en esta sesión (URL junto a cada punto). Lo marcado como **[inferencia]** no está literalmente en la doc.

---

## 1. OAuth 2.0 para apps de escritorio

Fuente: https://developers.google.com/identity/protocols/oauth2/native-app

- **Loopback redirect**: `http://127.0.0.1:PORT` o `http://[::1]:PORT`. Es el "mecanismo recomendado para obtener el authorization code" en macOS/Linux/Windows. (Solo está deprecado en apps móviles.)
- **Custom URI schemes**: "Custom URI schemes are no longer supported due to the risk of app impersonation."
- **PKCE**: obligatorio. `code_verifier` = string aleatorio con `[A-Z][a-z][0-9]-._~`, 43–128 chars; `code_challenge_method=S256` (recomendado) o `plain`.
- **client_secret**: en el intercambio de token figura como **"Optional"**; para clientes "Desktop app" el secreto no se considera confidencial (la app no puede protegerlo). Nota de la doc: no aplica a Android/iOS/Chrome. En la práctica el client de tipo Desktop se crea con secret y las librerías oficiales lo envían; podés incluirlo, pero el diseño de seguridad se apoya en PKCE + loopback.
- **Token endpoint**: `POST https://oauth2.googleapis.com/token`.
- **Refresh tokens**: "always returned for installed applications" y "valid until the user revokes access or the refresh token expires". Límites: 100 refresh tokens por cuenta Google por client ID (al crear el 101.º se invalida el más viejo sin aviso) y un límite global por usuario (https://developers.google.com/identity/protocols/oauth2 → "Refresh token expiration").
- Causas de invalidación: revocación por el usuario, 6 meses sin uso, cambio de contraseña (solo scopes Gmail), exceder el máximo de tokens, política de admin (`admin_policy_enforced`), proyecto en Testing (ver §2).

```http
# Paso 1: navegador
GET https://accounts.google.com/o/oauth2/v2/auth?client_id=...&redirect_uri=http://127.0.0.1:53682
  &response_type=code&scope=openid%20email%20https://www.googleapis.com/auth/calendar
  &code_challenge=<S256>&code_challenge_method=S256&access_type=offline&prompt=consent&state=<rand>

# Paso 2: intercambio
POST https://oauth2.googleapis.com/token   (application/x-www-form-urlencoded)
client_id=...&client_secret=...(opcional)&code=...&code_verifier=...
&grant_type=authorization_code&redirect_uri=http://127.0.0.1:53682
```
```json
{ "access_token": "ya29...", "expires_in": 3599, "refresh_token": "1//0g...",
  "scope": "openid email https://www.googleapis.com/auth/calendar",
  "token_type": "Bearer", "id_token": "eyJ..." }
```

**Scopes** (https://developers.google.com/workspace/calendar/api/auth):
- Lectura/escritura total: `https://www.googleapis.com/auth/calendar` ("See, edit, share, and permanently delete all the calendars you can access"). Cubre events, calendarList, calendars, acl, colors, settings, watch, channels.stop.
- Alternativa granular: `calendar.events` + `calendar.calendarlist` (+ `calendar.calendars.readonly`). `calendar.events` = "View and edit events on all your calendars"; `calendar.calendarlist` = "See, add, and remove Google calendars you're subscribed to".
- Email de la cuenta: scopes `openid email` (OIDC). "The scope parameter must begin with the `openid` value and then include the `profile` value, the `email` value, or both". El `id_token` trae `email`, `email_verified` y `sub` ("unique among all Google Accounts and never reused" → usar `sub` como clave de cuenta). Userinfo: `GET https://openidconnect.googleapis.com/v1/userinfo` (https://developers.google.com/identity/openid-connect/openid-connect). El legacy `userinfo.email` sigue apareciendo pero está superseded por `email`.
- **Clasificación**: los scopes de Calendar son **sensibles, no restringidos**. La guía de scopes sensibles usa como ejemplo "reading events stored in Google Calendar" y "My app will use https://www.googleapis.com/auth/calendar…" (https://developers.google.com/identity/protocols/oauth2/production-readiness/sensitive-scope-verification). Restringidos = los que exigen CASA/security assessment anual; Calendar no figura ahí (https://developers.google.com/identity/protocols/oauth2/production-readiness/restricted-scope-verification). `openid/email/profile` son básicos (no sensibles).

## 2. Consent screen: Testing vs In production, Workspace

- **Testing**: "Projects configured with a publishing status of Testing are limited to up to 100 test users". "Authorizations by a test user will expire seven days from the time of consent. If your OAuth client requests an `offline` access type… and receives a refresh token, that token will also expire." (https://support.google.com/cloud/answer/15549945). Confirmado en https://developers.google.com/identity/protocols/oauth2: proyecto External + Testing → "refresh token expiring in 7 days", salvo que solo pida scopes básicos (name/email/profile).
- **In production sin verificar**: se muestra la pantalla "unverified app" antes del consent y "your app will be limited to 100 new users until it is verified" (https://support.google.com/cloud/answer/7454865). Verificación **no** necesaria para "Apps in development: if your app is experimental or a test build… unless you decide to launch it to the public". Para uso personal: publicar a producción sin verificar es viable (refresh tokens no caducan a los 7 días; solo aparece la advertencia y el tope de 100 usuarios).
- **Internal vs External**: "Projects associated with a Google Cloud Organization… can configure Internal users to limit authorization requests to members of the organization"; External = "available to any user with a Google Account" (15549945). Un proyecto de una organización Workspace puede elegir External (la doc lo presenta como opción; Internal es un *límite* opcional). Para apps solo internas: "scopes aren't listed on the consent screen and use of restricted or sensitive scopes does not require further review by Google" (https://developers.google.com/workspace/guides/configure-oauth-consent). Como vas a loguear Gmail + Workspace, necesitás **External**.
- **Bloqueo por admin de Workspace** (https://knowledge.workspace.google.com/admin/apps/control-which-apps-access-google-workspace-data): estados **Trusted** ("Can access all Google services"), **Limited** ("Can only access unrestricted Google services"), **Specific Google data** (solo scopes que el admin lista), **Blocked** ("Can't access any Google service"). Ajuste global para apps no configuradas: "Allow users to access any third-party apps" (default) / "…only request basic info needed for Sign in with Google" / "Don't allow". Si un servicio (p. ej. Calendar) está marcado Restricted, "only internal and third-party apps configured with a Trusted or Specific Google data access setting can access" it. El admin agrega la app buscando por "app's name or client ID".
- **Cómo lo ve el usuario**: pantalla de Google "Access blocked: Authorization Error" con `Error 400: admin_policy_enforced` (referenciado como causa de fallo de tokens en la doc de OAuth2 y en https://support.google.com/a/thread/168978251). No se puede resolver del lado del usuario; el admin debe marcar el Client ID como Trusted.

## 3. Sincronización incremental

Fuentes: https://developers.google.com/workspace/calendar/api/guides/sync , https://developers.google.com/workspace/calendar/api/v3/reference/events/list , https://developers.google.com/workspace/calendar/api/v3/reference/calendarList/list

- Flujo: full sync (`events.list` sin `syncToken`), paginar con `nextPageToken`; la última página trae `nextSyncToken` (nunca ambos a la vez). Luego `events.list?syncToken=…`. Si hay muchos cambios, la respuesta incremental también pagina.
- `syncToken` **no** se puede combinar con: `iCalUID`, `orderBy`, `privateExtendedProperty`, `q`, `sharedExtendedProperty`, `timeMin`, `timeMax`, `updatedMin`. "All events deleted since the previous list request will always be in the result set and it is not allowed to set `showDeleted` to False."
- "Each list request should use the same set of query parameters, including the initial request" → si el full sync usó `singleEvents=true`, todos los incrementales deben usarlo. Con `singleEvents=true` el token devuelve instancias expandidas (no el master); con `false` devuelve masters + excepciones. Para una app con vista local es más simple `singleEvents=false` + expandir localmente con `instances` o RRULE, o bien `singleEvents=true` acotado (pero entonces **no** podés usar `timeMin/timeMax` con el token, así que el full sync inicial debe hacerse sin límite temporal o aceptar un token que cubre "todo").
- **410 GONE**: token expirado o cambio de ACL → "should trigger a full wipe of the client's store and a new full sync".
- `showDeleted`: incluye eventos `status: "cancelled"`. "Cancelled instances of recurring events (but not the underlying recurring event) will still be included if showDeleted and singleEvents are both False."
- `updatedMin`: alternativa sin token; "entries deleted since this time will always be included regardless of showDeleted". No se combina con `syncToken`.
- `maxResults` default 250, máx 2500. `eventTypes` repetible: birthday, default, focusTime, fromGmail, outOfOffice, workingLocation.
- **calendarList.list** también soporta `syncToken` ("All entries deleted and hidden since the previous list request will always be in the result set"); incompatible con `minAccessRole` y `showOwnOrganizationOnly`; 410 → resync. Usá `showHidden=true` y `showDeleted=true` en el full sync.

```http
GET /calendar/v3/calendars/primary/events?singleEvents=false&showDeleted=true&maxResults=2500
→ { "items":[...], "nextPageToken":"CiAKGjBpNDd2Nmp2Zml2cXRwYjBpOXA" }
GET /calendar/v3/calendars/primary/events?singleEvents=false&showDeleted=true&pageToken=...
→ { "items":[...], "nextSyncToken":"CPDAlvWDx70CEPDAlvWDx70CGAU=" }
GET /calendar/v3/calendars/primary/events?singleEvents=false&showDeleted=true&syncToken=CPDAlvWDx70CEPDAlvWDx70CGAU=
→ 200 { "items":[{"id":"abc","status":"cancelled",...}], "nextSyncToken":"..." }   | 410 GONE → wipe + full sync
```

## 4. Push notifications (webhooks)

Fuentes: https://developers.google.com/workspace/calendar/api/guides/push , https://developers.google.com/workspace/calendar/api/v3/reference/events/watch , https://developers.google.com/workspace/calendar/api/v3/reference/calendarList/watch , https://developers.google.com/workspace/calendar/api/v3/reference/channels/stop

- `POST /calendar/v3/calendars/{calendarId}/events/watch` y `POST /calendar/v3/users/me/calendarList/watch`.
```json
{ "id": "01234567-89ab-cdef-0123456789ab", "type": "web_hook",
  "address": "https://mydomain.com/notifications",
  "token": "target=myApp-myCalendarChannelDest", "expiration": 1426325213000,
  "params": { "ttl": "604800" } }
```
```json
{ "kind": "api#channel", "id": "01234567-89ab-cdef-0123456789ab", "resourceId": "o3hgv1538sdjfh",
  "resourceUri": "https://www.googleapis.com/calendar/v3/calendars/my_calendar@gmail.com/events",
  "token": "target=myApp-myCalendarChannelDest", "expiration": 1426325213000 }
```
- `id` ≤ 64 chars (UUID), `token` ≤ 256 chars (no poner secretos), `expiration` = Unix ms. `params.ttl` en segundos: "Default is 604800 seconds" (7 días). La guía: expiración "determined either by your request or by any Google Calendar API internal limits or defaults (the more restrictive value is used)". **No se documenta un máximo explícito**; asumí ~7 días y renová antes.
- **Renovación**: "there's no automatic way to renew a notification channel… you must replace it with a new one by calling the watch method" con un `id` nuevo; hay solapamiento entre canal viejo y nuevo.
- **Stop**: `POST https://www.googleapis.com/calendar/v3/channels/stop` con `{ "id": "...", "resourceId": "..." }`.
- **Headers** recibidos: `X-Goog-Channel-ID`, `X-Goog-Message-Number` (1 en sync), `X-Goog-Resource-ID`, `X-Goog-Resource-URI`, `X-Goog-Resource-State` (`sync` | `exists` | `not_exists`), `X-Goog-Channel-Expiration` (si aplica), `X-Goog-Channel-Token` (si se definió). Body vacío (`Content-Length: 0`): "you will need to make another API call to see the full change details" → al recibir `exists`, correr `events.list?syncToken=…`.
- Primer mensaje: `sync` ("it's possible to receive the sync message even before you receive the watch method response").
- Respuesta esperada del webhook: `200/201/202/204/102`. Si devolvés `500/502/503/504` Google reintenta con backoff exponencial.
- **HTTPS**: obligatorio con certificado válido; inválidos: "Self-signed certificates", "signed by an untrusted source", revocados o con subject distinto al hostname → hace falta CA pública (Let's Encrypt sirve).
- **Verificación de dominio**: la guía actual de push de Calendar **ya no contiene** un paso de registro de dominio, y el help de API Console dice literalmente: "Domain verification in the API Console is no longer required to make push notifications work with your domains" (https://support.google.com/googleapi/answer/7072069). El procedimiento histórico (obsoleto pero documentado): verificar el sitio en Search Console y luego APIs & Services → Domain verification → Add domain. Si alguna vez lo necesitás: el método de archivo HTML "can be used for URL-prefix properties, but not Domain properties"; Domain property exige DNS (https://support.google.com/webmasters/answer/9008080). No hay restricción documentada de qué dominios se aceptan.
- **[inferencia]** Para una app de escritorio detrás de NAT necesitás un endpoint público (VPS, Cloudflare Tunnel, ngrok con dominio fijo). Alternativa sin webhook: polling con `syncToken` cada N minutos (la guía de cuota recomienda push sobre polling).

## 5. Eventos recurrentes

Fuentes: https://developers.google.com/workspace/calendar/api/guides/recurringevents , https://developers.google.com/workspace/calendar/api/concepts/events-calendars , https://developers.google.com/workspace/calendar/api/v3/reference/events/instances

- `recurrence[]`: "List of RRULE, EXRULE, RDATE and EXDATE lines" (RFC 5545), sin DTSTART/DTEND (van en `start`/`end`).
```json
"start": {"date":"2015-06-01"}, "end": {"date":"2015-06-02"},
"recurrence": ["EXDATE;VALUE=DATE:20150610", "RDATE;VALUE=DATE:20150609,20150611",
               "RRULE:FREQ=DAILY;UNTIL=20150628;INTERVAL=3"]
```
- `events.list` por defecto devuelve masters + excepciones ("instances that are not exceptions are not returned"); con `singleEvents=true` devuelve instancias. `GET /calendars/{cid}/events/{eventId}/instances` expande un master (params: `originalStart`, `timeMin`, `timeMax`, `showDeleted`, `maxResults`, `pageToken`, `timeZone`).
- Cada instancia tiene `recurringEventId` (id del master) y `originalStartTime` ("the time at which this event would start according to the recurrence data"). El `id` de instancia es `masterId_YYYYMMDDTHHMMSSZ` (visible en los ejemplos de la guía).
- **Solo este evento**: `PUT/PATCH /calendars/{cid}/events/{instanceId}` → crea una excepción. Para borrar una instancia: PUT con `"status": "cancelled"` (o DELETE del instanceId).
- **Este y siguientes**: 2 llamadas: (1) PUT al master con `RRULE:...;UNTIL=<antes del inicio de la instancia, en UTC>`; (2) POST de un nuevo master que empieza en esa instancia. Caveat: "Changing all following instances resets any exceptions happening after the target instance."
```http
PUT /calendar/v3/calendars/primary/events/recurringEventId
{ "summary":"Appointment", "start":{"dateTime":"2011-06-03T10:00:00.000-07:00","timeZone":"America/Los_Angeles"},
  "end":{"dateTime":"2011-06-03T10:25:00.000-07:00","timeZone":"America/Los_Angeles"},
  "recurrence":["RRULE:FREQ=WEEKLY;UNTIL=20110617T065959Z"] }
POST /calendar/v3/calendars/primary/events
{ "summary":"Appointment", "location":"Somewhere else",
  "start":{"dateTime":"2011-06-17T10:00:00.000-07:00","timeZone":"America/Los_Angeles"},
  "end":{"dateTime":"2011-06-17T10:25:00.000-07:00","timeZone":"America/Los_Angeles"},
  "recurrence":["RRULE:FREQ=WEEKLY"] }
```
- **Todos**: PUT/PATCH al master (`recurringEventId`).
- **Zonas horarias**: `start.dateTime` RFC3339 con offset; `start.timeZone` "Formatted as an IANA Time Zone Database name, e.g. 'Europe/Zurich'" — requerido en eventos recurrentes (los ejemplos siempre lo incluyen). Todo el día: `start.date` / `end.date` `yyyy-mm-dd`, `end` exclusivo (un día = end = start+1).

## 6. Asistentes y RSVP

Fuentes: https://developers.google.com/workspace/calendar/api/v3/reference/events , https://developers.google.com/workspace/calendar/api/v3/reference/events/patch

- `attendees[].responseStatus`: `needsAction` | `declined` | `tentative` | `accepted`. `attendees[].self`: "Whether this entry represents the calendar on which this copy of the event appears". `attendees[].organizer`: bool.
- **¿Soy el organizador?** `organizer.self == true` ("Whether the organizer corresponds to the calendar on which this copy of the event appears") o la entrada de `attendees` con `self:true && organizer:true`. **[inferencia]** Si `organizer.email` no coincide con ningún email de tus cuentas, el evento es una invitación.
- `guestsCanModify`: "Whether attendees other than the organizer can modify the event"; `guestsCanInviteOthers` análogo.
- **Responder**: `PATCH /calendars/primary/events/{id}?sendUpdates=all` con el array `attendees` completo (patch: "Array fields, if specified, overwrite the existing arrays") cambiando solo tu entrada:
```json
{ "attendees": [ {"email":"boss@x.com","organizer":true,"responseStatus":"accepted"},
                 {"email":"me@gmail.com","self":true,"responseStatus":"accepted"} ] }
```
- `sendUpdates`: `all` ("Notifications are sent to all guests"), `externalOnly` (solo invitados no-Google Calendar), `none`. `sendNotifications` está deprecado.

## 7. Google Meet

Fuente: https://developers.google.com/workspace/calendar/api/guides/create-events (+ Events resource)

- Requiere `?conferenceDataVersion=1` en insert/patch/update. Tipos: `hangoutsMeet` (Meet), `eventHangout`, `eventNamedHangout` (deprecado), `addOn`.
```http
POST /calendar/v3/calendars/primary/events?conferenceDataVersion=1
{ "summary":"Sync", "start":{"dateTime":"2026-09-20T10:00:00-03:00"}, "end":{"dateTime":"2026-09-20T10:30:00-03:00"},
  "conferenceData": { "createRequest": { "requestId": "7qxalsvy0e", "conferenceSolutionKey": { "type": "hangoutsMeet" } } } }
```
- Respuesta: `conferenceData.createRequest.status.statusCode` = `pending` → `success` ("changes to success after the conference information is populated"; puede requerir re-GET) o `failure`. Luego leer `conferenceData.entryPoints[]` (`entryPointType` `video|phone|sip|more`, `uri`, `label`, `pin`) y `conferenceData.conferenceId`; `hangoutLink` trae el link directo.

## 8. Colores

Fuentes: https://developers.google.com/workspace/calendar/api/v3/reference/colors , https://developers.google.com/workspace/calendar/api/v3/reference/calendarList , https://developers.google.com/apps-script/reference/calendar/event-color , https://google-calendar-simple-api.readthedocs.io/en/latest/colors.html , https://calpalette.com/google-calendar-hex-codes

- `GET https://www.googleapis.com/calendar/v3/colors` → `{ "kind":"calendar#colors", "updated":…, "calendar": { "1": {"background":"#ac725e","foreground":"#1d1d1d"}, … }, "event": { "1": {"background":"#a4bdfc","foreground":"#1d1d1d"}, … } }`. Paleta `calendar` = 24 ids, `event` = 11 ids.
- `event.colorId` (1–11) referencia la sección `event`; `calendarListEntry.colorId` referencia la sección `calendar` pero "This property is superseded by the backgroundColor and foregroundColor properties" (hex por calendario, editable con `colorRgbFormat=true`). Si el evento no tiene `colorId`, usar el color del calendario.
- **Discrepancia UI vs API** (confirmada por Apps Script + fuentes de terceros): la API devuelve la paleta "clásica" y la UI web usa la moderna. Ejemplos id → API (colors.get) / UI web:
  1 Lavender `#a4bdfc` / `#7986cb` · 2 Sage `#7ae7bf` / `#33b679` · 3 Grape `#dbadff` / `#8e24aa` · 4 Flamingo `#ff887c` / `#e67c73` · 5 Banana `#fbd75b` / `#f6bf26` · 6 Tangerine `#ffb878` / `#f4511e` · 7 Peacock `#46d6db` / `#039be5` · 8 Graphite `#e1e1e1` / `#616161` · 9 Blueberry `#5484ed` / `#3f51b5` · 10 Basil `#51b749` / `#0b8043` · 11 Tomato `#dc2127` / `#d50000`.
  Apps Script confirma los nombres: `PALE_BLUE ("1")… referred to as 'Lavender' in the Calendar UI`. Recomendación: mapear por **id** a tu propia paleta "moderna" en lugar de usar los hex de `colors.get`. En cambio, `calendarListEntry.backgroundColor` sí devuelve el hex real elegido por el usuario (p. ej. `#7986cb`).

## 9. Recordatorios

- `reminders.useDefault` (bool) y `reminders.overrides[]` `{ "method": "email"|"popup", "minutes": 0..40320 }`, máximo 5 overrides (Events resource).
- Defaults por calendario y por usuario: `calendarListEntry.defaultReminders[]` ("The default reminders that the authenticated user has for this calendar"), mismo formato. También vienen en la respuesta de `events.list` (`defaultReminders`).
```json
"reminders": { "useDefault": false, "overrides": [ {"method":"popup","minutes":10}, {"method":"email","minutes":1440} ] }
```

## 10. Cuotas

Fuente: https://developers.google.com/workspace/calendar/api/guides/quota

- Por proyecto: **10.000 requests/min**. Por usuario por proyecto: **600 requests/min**. Umbral diario sin costo: 1.000.000 requests/día por proyecto. Excedido → `403` o `429` con `usageLimits`; aplicar backoff exponencial `min((2^n)+random_ms, 32–64 s)` y "use push notifications instead of polling".
- **[inferencia]** No hay costo por método: cada llamada cuenta 1. Patrones de esta app: full sync inicial ≈ (páginas de 2500) × calendarios; incremental = 1 request por calendario por tick; renovar canales = 1 `watch`/canal/semana; `colors.get` una vez y cachear; `calendarList.list` con syncToken. Con 3–5 cuentas × ~10 calendarios, un polling cada 60 s queda muy por debajo de 600/min/usuario.

## 11. Feriados (Argentina)

Fuentes: https://developers.google.com/workspace/calendar/api/v3/reference/calendarList/insert ; IDs en https://gist.github.com/dhoeric/76bd1c15168ee0ee61ad3bf1730dcb65 y https://federicoscodelaro.com/blog/2024-02-01-google-calendar-holidays-ics/ (no oficiales; Google no publica la lista).

- IDs: `en.ar#holiday@group.v.calendar.google.com` (inglés), `es.ar#holiday@group.v.calendar.google.com` (español), `en.ar.official#holiday@group.v.calendar.google.com` (solo oficiales; sin `.official` incluye observancias).
- Suscribirse: `calendarList.insert` "Inserts an existing calendar into the user's calendar list" (id es el único campo requerido; requiere scope `calendar` o `calendar.calendarlist`):
```http
POST https://www.googleapis.com/calendar/v3/users/me/calendarList
{ "id": "es.ar#holiday@group.v.calendar.google.com" }
→ { "kind":"calendar#calendarListEntry", "id":"es.ar#holiday@group.v.calendar.google.com",
    "summary":"Feriados en Argentina", "accessRole":"reader", "backgroundColor":"#...", ... }
```
- Leer: `GET /calendars/es.ar%23holiday%40group.v.calendar.google.com/events?timeMin=...&timeMax=...&singleEvents=true` (URL-encode `#` y `@`). Son eventos all-day (`start.date`). También hay ICS público: `https://calendar.google.com/calendar/ical/<id>/public/basic.ics`.

## 12. Mover eventos entre calendarios / cuentas

Fuentes: https://developers.google.com/workspace/calendar/api/v3/reference/events/move , https://developers.google.com/workspace/calendar/api/v3/reference/events/import

- `POST /calendars/{calendarId}/events/{eventId}/move?destination={otherCalendarId}&sendUpdates=none`. "Only default events can be moved; birthday, focusTime, fromGmail, outOfOffice and workingLocation events cannot be moved." Requiere scope `calendar`, `calendar.events` o `calendar.events.owned`.
- **[inferencia]** `move` se ejecuta con un único access token, por lo que origen y destino deben ser calendarios accesibles (con permiso de escritura) por esa misma cuenta. Entre dos cuentas Google distintas (dos tokens) no hay endpoint: hay que `events.insert` (o `events.import`) en la cuenta B y `events.delete` en la A.
- `events.import` "adds a private copy of an existing event to a calendar"; requiere `iCalUID`, `start`, `end`; solo `eventType: default`. Conserva el `iCalUID` ("used to uniquely identify events accross calendaring systems") → útil para deduplicar el mismo evento visto desde varias cuentas (un evento con invitados aparece en cada cuenta invitada con el **mismo `iCalUID`** pero distinto `id`). Con `conferenceDataVersion=1` copia el Meet.
```http
POST /calendar/v3/calendars/primary/events/import
{ "iCalUID":"abc123@google.com", "summary":"Copia", "start":{"dateTime":"2026-09-20T10:00:00-03:00"},
  "end":{"dateTime":"2026-09-20T11:00:00-03:00"}, "attendees":[{"email":"x@y.com"}] }
```
- Al copiar con `insert` y `sendUpdates=all` se reenvían invitaciones; usar `sendUpdates=none` o `import` para evitar spam.

---

## Resumen de riesgos / decisiones

1. Usar client "Desktop app" + loopback `127.0.0.1` + PKCE S256; `client_secret` es opcional y no confidencial.
2. Publicar el consent screen a **In production** (External) aunque no se verifique: en Testing los refresh tokens mueren a los 7 días. Sin verificar hay pantalla "unverified app" y tope de 100 usuarios (irrelevante para uso personal).
3. Calendar es scope **sensible** (no restringido): sin CASA. Pedir `openid email` + `calendar` (o `calendar.events` + `calendar.calendarlist`). Usar `sub` del id_token como clave de cuenta.
4. Cuentas Workspace pueden fallar con `Error 400: admin_policy_enforced` si el admin no marcó tu Client ID como Trusted.
5. Sync: mismo set de params en todos los `list`; `syncToken` es incompatible con `timeMin/timeMax/updatedMin/orderBy/q`; 410 → wipe + full sync; `showDeleted` implícito.
6. Push: HTTPS con CA pública, TTL default 7 días sin renovación automática, verificación de dominio ya no requerida; el body llega vacío → siempre seguir con `list?syncToken`.
7. Recurrencia: "este y siguientes" = UNTIL en master + nuevo master, y borra excepciones posteriores; siempre mandar `timeZone` IANA.
8. RSVP = `patch` con el array `attendees` completo (los arrays se reemplazan) + `sendUpdates`.
9. Colores: `colors.get` devuelve la paleta clásica (`#a4bdfc`), la UI web usa la moderna (`#7986cb`); mapear por id.
10. `events.move` solo dentro de la misma cuenta; entre cuentas `import`/`insert` + `delete`, deduplicando por `iCalUID`.
