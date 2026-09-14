# 05 — Sincronización, OAuth y push

Fuentes: `docs/research/google-calendar-api.md`, `docs/research/tailscale-push.md`. Todo parámetro de API mencionado acá está verificado en esos informes.

## 1. OAuth 2.0

### 1.1 Credenciales

Archivo `~/.config/unified-google-calendar/oauth.json`, creado por el usuario según `09-setup-usuario.md`:

```json
{ "client_id": "....apps.googleusercontent.com", "client_secret": "..." }
```

Tipo de cliente en Google Cloud: "Desktop app". `client_secret` es opcional para este tipo y no se considera confidencial. Se envía igual porque las librerías oficiales lo hacen.

### 1.2 Flujo de alta de cuenta (`add_account`)

1. Generar `code_verifier` de 64 caracteres del alfabeto `[A-Za-z0-9-._~]` y `code_challenge = base64url(sha256(verifier))`. Generar `state` de 32 bytes aleatorios en base64url.
2. `TcpListener::bind("127.0.0.1:0")`. Tomar el puerto asignado `P`.
3. Abrir en el navegador del sistema con `tauri-plugin-opener`:

```
https://accounts.google.com/o/oauth2/v2/auth
  ?client_id=...
  &redirect_uri=http://127.0.0.1:P
  &response_type=code
  &scope=openid%20email%20https://www.googleapis.com/auth/calendar
  &code_challenge=...&code_challenge_method=S256
  &access_type=offline&prompt=consent
  &state=...
```

4. Esperar una conexión HTTP en el listener, máximo 5 minutos. Parsear `code` y `state` del query. Si `state` no coincide, responder 400 y abortar. Responder una página HTML mínima "You can close this tab" y cerrar el listener.
5. `POST https://oauth2.googleapis.com/token` con `client_id, client_secret, code, code_verifier, grant_type=authorization_code, redirect_uri`.
6. Del `id_token` leer `sub` y `email` sin verificar firma, porque viene directo del endpoint de token por TLS. `sub` es el `accounts.id`.
7. Guardar `refresh_token`, `access_token`, `expires_at` en el archivo cifrado. Si la cuenta ya existía, reemplazar.
8. Crear la fila en `accounts`, correr `calendarList.list` completo y luego full sync de cada calendario.

Scope único `https://www.googleapis.com/auth/calendar` porque la app necesita `calendarList.insert` para feriados, `events.watch`, `events.move` e `import`. Es scope sensible, no restringido. No requiere evaluación CASA.

### 1.3 Refresco de tokens

- `access_token` se refresca cuando faltan menos de 5 minutos para `expires_at`, con `grant_type=refresh_token`. Un `Mutex` por cuenta evita refrescos simultáneos.
- Respuesta `invalid_grant` significa refresh token revocado o vencido. La cuenta pasa a `sync_state = auth_required` y la UI muestra un botón "Sign in again" en la barra lateral. No se reintenta solo.
- `admin_policy_enforced` en el consentimiento significa que el admin de Workspace bloqueó la app. Se muestra el mensaje textual de Google y el `client_id`, para que el usuario lo pase a su admin.

### 1.4 Estado de publicación del proyecto

El consent screen debe estar en "In production" aunque no esté verificado. En "Testing" los refresh tokens vencen a los 7 días. Detalle en `09-setup-usuario.md`.

## 2. Sincronización de calendarios y eventos

### 2.1 calendarList

- Full: `GET /users/me/calendarList?showHidden=true&showDeleted=true&maxResults=250`, paginar hasta `nextSyncToken`. Guardar en `accounts.calendar_list_sync_token`.
- Incremental: mismos parámetros más `syncToken`. Entradas con `deleted=true` marcan `calendars.deleted=1` y borran sus eventos y ocurrencias.
- Un calendario nuevo dispara full sync de eventos y creación de canal push.
- `410 Gone` en calendarList: borrar el token y rehacer el full.

### 2.2 Eventos

Parámetros fijos para **todas** las llamadas a `events.list` de un calendario, porque el sync token exige el mismo set:

```
singleEvents=false
showDeleted=true
maxResults=2500
```

Sin `timeMin`, `timeMax`, `updatedMin`, `orderBy` ni `q`. Nunca.

Full sync:

1. `events.list` con los parámetros fijos, paginar por `nextPageToken` hasta recibir `nextSyncToken`.
2. En una transacción por página: `upsert` de cada evento en `events`. `status=cancelled` se guarda como tal.
3. Al terminar: guardar `calendars.sync_token`, marcar `full_sync_done=1`, correr `recurrence::expand` sobre todos los masters y materializar los no recurrentes dentro de la ventana.
4. Emitir `calendar:updated` con el rango completo de la ventana.

Incremental:

1. `events.list` con parámetros fijos más `syncToken`.
2. Por cada item: upsert. Si es master, re-expandir. Si es excepción, re-expandir su master. Si `status=cancelled` y es master, borrar sus ocurrencias. Si es un evento simple cancelado, borrar su ocurrencia.
3. Guardar el `nextSyncToken` nuevo.
4. Emitir `calendar:updated` con el rango mínimo que cubre los eventos tocados.
5. `410 Gone`: borrar `sync_token`, borrar eventos y ocurrencias del calendario, full sync.

Errores `403` o `429` con `usageLimits`: backoff exponencial `min(2^n + jitter, 64 s)`, máximo 5 intentos, luego `sync_state = error` hasta el siguiente tick.

### 2.3 Escrituras

Todas las escrituras esperan la respuesta y hacen upsert con ella. Reglas:

- `insert` y `update` llevan `conferenceDataVersion=1` siempre. Sin eso Google descarta `conferenceData`.
- `update` parte de `events.raw`, aplica los cambios y envía el objeto completo. `patch` solo para RSVP.
- `sendUpdates`: `all` si el evento tiene invitados y el usuario es organizador, `none` en cualquier otro caso. Al mover entre cuentas siempre `none`.
- Eventos recurrentes nuevos llevan `start.timeZone` y `end.timeZone` con el nombre IANA de la zona principal.
- Día completo: `start.date` y `end.date` con fin exclusivo.
- Meet: `conferenceData.createRequest` con `requestId` uuid y `conferenceSolutionKey.type = hangoutsMeet`. Si la respuesta trae `createRequest.status.statusCode = pending`, hacer `events.get` a los 2 segundos, máximo 3 veces.
- RSVP: `patch` con el array `attendees` completo tomado de `raw`, cambiando solo la entrada con `self=true`.

### 2.4 Ventana de datos

Google no permite filtrar por tiempo con sync token, así que `events` contiene todo el historial del calendario. `occurrences` solo cubre la ventana. Si un calendario tiene más de 50 000 eventos, la primera sincronización tarda y se muestra progreso por páginas en `sync:status`. No se recorta.

## 3. Push por Tailscale Funnel

### 3.1 Endpoint local

`webhook::server` en `127.0.0.1:<webhook_port>`, default 8080. Rutas:

- `POST /gcal/webhook`: valida y encola. Responde `200` con cuerpo vacío en menos de 50 ms.
- `GET /google<token>.html`: sirve el archivo de verificación de Search Console desde `~/.config/unified-google-calendar/verify/` si existe. Solo se usa si Google llegara a exigir verificación.
- `GET /healthz`: `200 ok`. Para que el usuario compruebe el túnel.
- Todo lo demás: `404`.

Validación del POST:

1. `X-Goog-Channel-ID` existe en `channels`. Si no, `404`.
2. `X-Goog-Channel-Token` es igual a `channels.token` comparado en tiempo constante. Si no, `404`.
3. `X-Goog-Resource-State`: `sync` se ignora y registra en `sync_log`. `exists` y `not_exists` encolan `SyncTick`.
4. El cuerpo se descarta sin leer. Límite 64 KB.

Tailscale reenvía las cabeceras `X-Goog-*` sin tocarlas y agrega `X-Forwarded-For` y `Tailscale-Funnel-Request`. Con el mount en `/` el path llega intacto. Con `--set-path` Tailscale recorta el prefijo, por eso el Funnel se monta en `/`.

### 3.2 URL pública

`settings.public_base_url` = `https://<pc>.<tailnet>.ts.net`. El usuario la configura en Settings de la app la primera vez. La app la valida con `GET <public_base_url>/healthz` desde el propio proceso, que sale a internet y vuelve por el Funnel. Si falla, `push_enabled=false` y se avisa.

### 3.3 Canales

Por cada calendario con `full_sync_done=1` y por la calendarList de cada cuenta:

```json
POST /calendars/{calendarId}/events/watch
{ "id": "<uuid v4>", "type": "web_hook",
  "address": "https://<pc>.<tailnet>.ts.net/gcal/webhook",
  "token": "<32 bytes base64url>",
  "params": { "ttl": "604800" } }
```

- Guardar `id`, `resourceId`, `token`, `expiration` en `channels`.
- Renovación: una tarea cada hora busca canales con `expiration_ts < now + 24h`, crea uno nuevo primero y después llama `channels.stop` sobre el viejo. Google no renueva solo.
- Al quitar una cuenta: `channels.stop` de todos sus canales.
- Si `watch` responde error de webhook no autorizado, la app registra el error, pone `push_enabled=false` y sigue con polling. El mensaje en Settings dice qué pasó y enlaza a `09-setup-usuario.md` sección de verificación de dominio.

### 3.4 Encolado

`SyncTick { account_id, calendar_id }` entra a un `tokio::sync::mpsc` de capacidad 256. El consumidor deduplica ticks pendientes del mismo calendario y ejecuta el incremental bajo el mutex del calendario. Un push de Google llega sin cuerpo y solo dice "algo cambió"; el incremental trae el detalle.

## 4. Red de seguridad

Google avisa que los push no son 100% confiables. Por eso:

- Polling de respaldo: incremental de todos los calendarios cada 10 minutos cuando `push_enabled=true`, cada 60 segundos cuando `push_enabled=false`.
- Al despertar: `sync::sleep` escucha `PrepareForSleep` de `org.freedesktop.login1.Manager` por `zbus`. Al recibir `false` corre incremental de todo y la revisión de canales.
- Al arrancar: incremental de todo.
- `sync_now` desde la UI: incremental de todo.

Costo del polling cada 60 s con 3 cuentas y 5 calendarios cada una: 15 requests por minuto, muy por debajo de la cuota de 600 por minuto por usuario.

## 5. Estados y UI

`accounts.sync_state`:

| Estado | Significado | UI |
|---|---|---|
| `idle` | Última sync OK | Punto verde con "Synced N min ago" en la barra superior |
| `syncing` | Full o incremental en curso | Punto azul animado |
| `error` | Fallo de red o de API en el último intento | Punto rojo, tooltip con el mensaje. Se reintenta en el próximo tick. |
| `auth_required` | Refresh token inválido o revocado | Fila de cuenta con botón "Sign in again" |

El texto de la barra superior toma el estado más grave entre las cuentas.

## 6. Orden de arranque de `sync::start`

1. Para cada cuenta con tokens: refrescar access token si hace falta.
2. calendarList incremental o full.
3. Por calendario: incremental o full, en paralelo entre cuentas, secuencial dentro de una cuenta.
4. Recalcular ventana y materializar.
5. Si `push_enabled`: asegurar canales.
6. Arrancar el temporizador de respaldo y el listener de suspensión.

Nada de esto bloquea la creación de la ventana. La UI arranca con lo que hay en SQLite.
