# Tailscale Funnel como receptor de push notifications de Google Calendar (Ubuntu 25.04)

Investigación del 2026-09-14. Todas las afirmaciones citan la URL consultada. Cuando un dato NO aparece en la documentación se indica explícitamente.

## 1. Tailscale Funnel

### Requisitos (fuente: https://tailscale.com/kb/1223/funnel y https://tailscale.com/docs/features/tailscale-funnel)
- Tailscale >= v1.38.3.
- **MagicDNS habilitado** en el tailnet.
- **HTTPS habilitado** ("HTTPS enabled and valid HTTPS certificates for your tailnet"). Se activa en el admin console > DNS > "HTTPS Certificates" > "Enable HTTPS"; los certificados los emite **Let's Encrypt** (validez 90 días) y los nombres de máquina aparecen en los logs públicos de Certificate Transparency (https://tailscale.com/kb/1153/enabling-https).
- **Atributo de nodo `funnel`** en el policy file. Sintaxis exacta (https://tailscale.com/kb/1337/acl-syntax):
  ```json
  "nodeAttrs": [
    { "target": ["autogroup:member"], "attr": ["funnel"] }
  ]
  ```
  Alternativa: en el admin console > Access controls > "Add Funnel to policy" (https://tailscale.com/docs/features/tailscale-funnel).
- **Plan**: "Tailscale Funnel is available for all plans" (https://tailscale.com/docs/features/tailscale-funnel). La tabla de https://tailscale.com/pricing lista `Funnel` con `personal: "Y"`. El plan Personal es gratis, hasta 6 usuarios (https://tailscale.com/kb/1154/free-plans-logos).

### Puertos, ancho de banda, hostname, TLS
- Sólo puertos **443, 8443 y 10000** (https://tailscale.com/kb/1223/funnel).
- Ancho de banda: "Traffic sent over a Funnel is subject to non-configurable bandwidth limits"; Tailscale no publica la cifra ("it's a funnel, not a hose. We don't announce what the bandwidth limit is", empleada de Tailscale en https://news.ycombinator.com/item?id=35375794). Para POSTs sin cuerpo de Google es irrelevante.
- Hostname **estable**: "Funnels have a predictable, stable DNS name, like amelie-workstation.pango-lin.ts.net" y "set or share your DNS name one time. Then, it's accessible anytime you turn your Funnel on" (https://tailscale.com/kb/1247/funnel-serve-use-cases). Formato: `<machine>.<tailnet-name>.ts.net`, donde tailnet-name puede ser `tailNNNN`, `tailnet-NNNN` o un nombre "divertido" tipo `yak-bebop` (https://tailscale.com/kb/1153/enabling-https). Los registros DNS públicos pueden tardar hasta 10 min en aparecer (https://tailscale.com/docs/features/tailscale-funnel).
- TLS: "TLS termination occurs at the Tailscale server on your device"; los relays de Funnel no descifran el tráfico (https://tailscale.com/kb/1223/funnel). Funnel sólo funciona sobre TLS.

### Instalación en Ubuntu 25.04 (repo apt oficial, bloque "Ubuntu 25.04 (Plucky Puffin)" de https://pkgs.tailscale.com/stable/)
```bash
sudo mkdir -p --mode=0755 /usr/share/keyrings
curl -fsSL https://pkgs.tailscale.com/stable/ubuntu/plucky.noarmor.gpg | sudo tee /usr/share/keyrings/tailscale-archive-keyring.gpg >/dev/null
curl -fsSL https://pkgs.tailscale.com/stable/ubuntu/plucky.tailscale-keyring.list | sudo tee /etc/apt/sources.list.d/tailscale.list
sudo apt-get update && sudo apt-get install tailscale
sudo tailscale up          # imprime una URL para autenticar el nodo en el tailnet
```
Alternativa oficial: `curl -fsSL https://tailscale.com/install.sh | sh` (https://tailscale.com/kb/1031/install-linux).

### Sintaxis CLI actual: `serve` vs `funnel` (https://tailscale.com/kb/1311/tailscale-funnel y https://tailscale.com/kb/1242/tailscale-serve)
- `tailscale serve` expone sólo dentro del tailnet; `tailscale funnel` expone a Internet. Misma gramática: `tailscale funnel [flags] <target>`.
- Flags: `--bg` (proceso en segundo plano), `--https=<443|8443|10000>`, `--set-path=<path>`, `--tcp=`, `--tls-terminated-tcp=`, `--yes`.
- Target: puerto (`3000`), `localhost:3000`, `https+insecure://localhost:8443`, archivo, `text:"..."`. Sólo se admite `http://127.0.0.1` como reverse proxy local.
- Ejemplos verbatim: `tailscale funnel localhost:3000`, `tailscale funnel status --json`, `tailscale funnel reset`, apagado: `tailscale funnel --https=443 --set-path=/foo off`.
- Comando recomendado para este proyecto (app local en 8080, ruta `/gcal/webhook`):
  ```bash
  sudo tailscale funnel --bg --https=443 --set-path=/gcal/webhook 8080
  tailscale funnel status
  # apagar: sudo tailscale funnel --https=443 --set-path=/gcal/webhook off
  ```

### Persistencia tras reinicio
"With --bg ... it runs persistently in the background until you turn it off. When you reboot the device or restart Tailscale ... `tailscale down` and `tailscale up`, Funnel will automatically resume sharing." Sin `--bg` hay que relanzarlo a mano (https://tailscale.com/kb/1311/tailscale-funnel). tailscaled se instala como servicio systemd por el paquete apt.

### Path, cabeceras y cuerpo (fuente: código `ipn/ipnlocal/serve.go`, commit e2ed432 de https://raw.githubusercontent.com/tailscale/tailscale/main/ipn/ipnlocal/serve.go, y https://tailscale.com/docs/features/tailscale-serve)
- **Path**: el proxy es un `httputil.ReverseProxy`; **recorta el mount point antes de reenviar** ("Trim the mount point from the URL path before proxying. (#6571)": `http.StripPrefix(strings.TrimSuffix(mountPoint,"/"), h)`). Con `--set-path=/gcal/webhook`, la app recibe `POST /`. Con mount en `/` (sin `--set-path`) el path llega intacto ("Funnel forwards HTTPS traffic from any path to http://localhost:3000", https://tailscale.com/kb/1247/funnel-serve-use-cases). Historia en https://github.com/tailscale/tailscale/issues/6571.
- **Cabeceras añadidas** (`addProxyForwardedHeaders`): `X-Forwarded-Host`, `X-Forwarded-Proto: https`, `X-Forwarded-For`. En peticiones Funnel se añade `Tailscale-Funnel-Request: ?1`.
- **Cabeceras Tailscale-User-***: se borran de la petición entrante ("Clear any incoming values squatting in the headers") y **no se añaden** en Funnel ("if c.Funnel != nil { ... return }"). Docs: "Funnel traffic, which is publicly available, does not include identity headers" y Serve "will remove them for security reasons, to avoid header spoofing" (https://tailscale.com/docs/features/tailscale-serve).
- **X-Goog-***: el ReverseProxy sólo modifica/borra las cabeceras listadas arriba; el resto (incluidas `X-Goog-Channel-ID`, `X-Goog-Channel-Token`, `X-Goog-Resource-State`, etc.) pasa intacto. Los cuerpos se reenvían (es un reverse proxy HTTP estándar de Go); las notificaciones de Google no traen cuerpo de todos modos (ver §4).

## 2. Verificación de dominio en Google para un hostname `*.ts.net`

### Qué exige Google hoy
- https://support.google.com/googleapi/answer/7072069 : "Domain verification ensures that push notifications ... are sent only to domains associated with the subscribing project". Aplica a Pub/Sub, Drive y Calendar. Pasos: 1) verificar propiedad en **Google Search Console**; 2) API Console > APIs & services > Domain verification > Add domain > introducir el dominio ya verificado.
- **Nota textual en esa misma página**: "Domain verification in the API Console is no longer required to make push notifications work with your domains." La guía actual de push de Calendar (https://developers.google.com/workspace/calendar/api/guides/push) **ya no contiene ninguna sección "Registering your domain"**; sólo exige HTTPS con certificado válido (no autofirmado, no revocado, hostname coincidente). Es decir: puede que ni haga falta verificar; pruébese primero `events.watch` directamente. Reportes antiguos de "Unauthorized WebHook callback channel" (https://groups.google.com/g/google-calendar-api/c/1wVxEZL3gM8) se resolvieron con verificación en Webmaster Tools y/o cambiando el tipo de credencial.

### ¿Se puede verificar `pc.tailnet.ts.net`?
- **ts.net está en la Public Suffix List** (líneas `ts.net` y `*.c.ts.net`, sección "Tailscale Inc.", versión 2026-09-08 de https://publicsuffix.org/list/public_suffix_list.dat). También están `ngrok.io`, `ngrok-free.app`, `ngrok.app`, `duckdns.org`, `trycloudflare.com`, `github.io`.
- Implicación: Search Console dice "We don't support domain-property verification of public suffixes" (https://support.google.com/webmasters/answer/34592). Por tanto NO se puede crear una "Domain property" de `ts.net`, pero **sí una "URL-prefix property"** `https://pc.tailnet.ts.net/` verificada por **archivo HTML** o **meta tag** (métodos disponibles sólo para URL-prefix; el archivo no puede requerir login ni redirecciones; Google re-comprueba periódicamente el token) (https://support.google.com/webmasters/answer/9008080).
- Evidencia de que funciona con hostnames de sufijo público:
  - `username.github.io` (github.io está en la PSL) se verifica como URL-prefix con archivo HTML (https://github.com/orgs/community/discussions/163649).
  - `ngrok.io` (en la PSL): gist con verificación por archivo HTML servido vía ngrok y webhook final `https://b3093b51.ngrok.io/google/webhook` para Google Calendar (https://gist.github.com/lorisleiva/36c0173ddf082f1495d25aca119ace7e); herramienta https://github.com/brh55/google-domain-verify creada exactamente para verificar subdominios ngrok para "Google webhook services".
  - **No se encontró ningún reporte específico de un `*.ts.net` verificado en Search Console** (búsquedas: "ts.net" "Search Console"/"Domain verification"; resultados sólo genéricos, p. ej. https://tailscale.com/docs/reference/examples/funnel). Por analogía con ngrok.io/github.io debería funcionar, pero está sin confirmar.
- Procedimiento propuesto:
  ```bash
  # 1) servir el archivo de verificación en la raíz del funnel (mount /)
  mkdir -p ~/gsc && cp ~/Downloads/google<token>.html ~/gsc/
  sudo tailscale funnel --bg --https=443 ~/gsc          # sirve el directorio en https://pc.tailnet.ts.net/
  curl -I https://pc.tailnet.ts.net/google<token>.html  # debe dar 200 sin redirect
  # 2) Search Console > Add property > URL prefix > https://pc.tailnet.ts.net/ > HTML file > Verify
  # 3) (opcional) Cloud Console > APIs & Services > Domain verification > Add domain > pc.tailnet.ts.net
  # 4) volver a exponer la app: sudo tailscale funnel --https=443 off && sudo tailscale funnel --bg --https=443 8080
  ```
  Mantener el archivo `google<token>.html` servido por la app en `/` (Search Console re-verifica periódicamente).

## 3. Alternativas si la verificación falla
- **(a) Dominio propio**: verificar Domain property por DNS TXT (cubre todos los subdominios, https://support.google.com/webmasters/answer/9008080). Funnel NO puede usar dominios propios: "Funnel can only use DNS names in your tailnet's domain" (https://tailscale.com/docs/features/tailscale-funnel); haría falta (b).
- **(b) Cloudflare Tunnel + dominio propio**: requiere "add a website to Cloudflare" (zona propia) (https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/get-started/create-remote-tunnel/). Túnel gratuito; la capa Access es gratis hasta 50 usuarios (https://community.cloudflare.com/t/50-user-limit-on-free-plan/546057). El path no se reescribe ("does not strip or rewrite the path"). Instalación (https://pkg.cloudflare.com/index.html) y servicio (https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/configure-tunnels/local-management/as-a-service/linux/):
  ```bash
  sudo mkdir -p --mode=0755 /usr/share/keyrings
  curl -fsSL https://pkg.cloudflare.com/cloudflare-main.gpg | sudo tee /usr/share/keyrings/cloudflare-main.gpg >/dev/null
  echo 'deb [signed-by=/usr/share/keyrings/cloudflare-main.gpg] https://pkg.cloudflare.com/cloudflared any main' | sudo tee /etc/apt/sources.list.d/cloudflared.list
  sudo apt-get update && sudo apt-get install cloudflared
  sudo cloudflared service install <TOKEN>   # token del túnel creado en el dashboard; luego systemctl status cloudflared
  ```
- **(c) Polling con syncToken** (https://developers.google.com/workspace/calendar/api/guides/sync): `events.list?syncToken=...` devuelve sólo cambios; hay que usar los mismos parámetros que en la sync inicial (si no, 400); ante `410 GONE` borrar el store y hacer full sync; paginar con `pageToken` hasta obtener `nextSyncToken`.
  - Cuota (https://developers.google.com/workspace/calendar/api/guides/quota): **10.000 req/min por proyecto**, **600 req/min por usuario y proyecto**, 1.000.000 req/día por proyecto antes de facturación.
  - Coste: N cuentas × M calendarios × 1 req/60 s = N·M req/min = 1.440·N·M req/día. Ej.: 3 cuentas × 5 calendarios = 15 req/min = 21.600/día (2,2 % del millón diario; 0,15 % del límite por minuto; por usuario 5/600). Incluso 20 cuentas × 10 calendarios = 200 req/min = 288.000/día, dentro de cuota. El límite real es la latencia (≤60 s) y el consumo de batería/red, no la cuota.
  - Recomendación de terceros: aun con push, hacer un incremental sync periódico como red de seguridad (https://nango.dev/blog/how-to-build-a-real-time-google-calendar-api-integration/).

## 4. Seguridad del endpoint público (fuente: https://developers.google.com/workspace/calendar/api/guides/push)
- Google envía cabeceras `X-Goog-Channel-ID`, `X-Goog-Channel-Token` (el `token` del watch, máx. 256 caracteres), `X-Goog-Resource-ID`, `X-Goog-Resource-State` (`sync`|`exists`|`not_exists`), `X-Goog-Message-Number`, `X-Goog-Channel-Expiration`. "Notification messages posted by the Google Calendar API to your receiving URL do not include a message body."
- Recomendaciones:
  1. Generar `token` aleatorio (≥32 bytes, base64url) por canal y guardarlo con el `id` (UUID) y `resourceId`; en cada POST comparar en tiempo constante `X-Goog-Channel-Token` **y** que `X-Goog-Channel-ID` exista en la tabla local; si no, responder 404 sin procesar.
  2. Ignorar el cuerpo (leer y descartar, con límite p. ej. 64 KB) y **nunca** confiar en él: los datos se obtienen con `events.list?syncToken` autenticado.
  3. Responder siempre rápido `200` (códigos de éxito aceptados: 200, 201, 202, 204, 102); encolar el trabajo. Sólo devolver 5xx si realmente se quiere que Google reintente.
  4. Rate-limit por IP/ruta y descartar métodos distintos de POST; aceptar sólo el path del webhook. Mensaje `sync` (`X-Goog-Resource-State: sync`, número 1) confirma el canal.
  5. Los canales expiran (TTL por defecto 604800 s = 7 días, https://developers.google.com/workspace/calendar/api/v3/reference/events/watch); renovar con nuevo `id` antes de expirar y llamar a `channels/stop` sobre el viejo.
- Tailscale no añade ninguna identidad a peticiones Funnel; sí llega `Tailscale-Funnel-Request: ?1` y `X-Forwarded-For` (IP pública de Google), útiles para logging (serve.go, citado en §1).

## 5. Laptop dormida / sin conexión
- Reintentos: "If your service ... returns 500, 502, 503, or 504, the Google Calendar API retries with exponential backoff. Every other return status code is considered to be a message failure." No se documenta número de reintentos ni el comportamiento ante host inalcanzable (https://developers.google.com/workspace/calendar/api/guides/push). Un tercero afirma que Google reintenta "for a limited time" y luego el canal "effectively becomes inactive" (https://www.codewords.ai/blog/google-calendar-webhooks); no está confirmado por Google.
- Google advierte en la misma guía: "Notifications are not 100% reliable. Expect a small percentage of messages to get dropped under normal working conditions."
- Con el PC dormido, Funnel no responde (el relay sólo reenvía si el nodo está conectado: "As long as your development machine stays connected to Tailscale", https://tailscale.com/kb/1247/funnel-serve-use-cases). Por tanto **al despertar** la app debe: (1) ejecutar `events.list?syncToken` por calendario (catch-up); (2) comprobar expiración de canales y re-crear los vencidos; (3) mantener un polling de respaldo (p. ej. cada 5–15 min) porque las notificaciones pueden perderse incluso en operación normal.

## Fuentes principales
https://tailscale.com/kb/1223/funnel · https://tailscale.com/docs/features/tailscale-funnel · https://tailscale.com/kb/1311/tailscale-funnel · https://tailscale.com/kb/1242/tailscale-serve · https://tailscale.com/docs/features/tailscale-serve · https://tailscale.com/kb/1247/funnel-serve-use-cases · https://tailscale.com/kb/1153/enabling-https · https://tailscale.com/kb/1337/acl-syntax · https://tailscale.com/pricing · https://pkgs.tailscale.com/stable/ · https://raw.githubusercontent.com/tailscale/tailscale/main/ipn/ipnlocal/serve.go · https://github.com/tailscale/tailscale/issues/6571 · https://news.ycombinator.com/item?id=35375794 · https://developers.google.com/workspace/calendar/api/guides/push · https://developers.google.com/workspace/calendar/api/guides/sync · https://developers.google.com/workspace/calendar/api/guides/quota · https://developers.google.com/workspace/calendar/api/v3/reference/events/watch · https://support.google.com/googleapi/answer/7072069 · https://support.google.com/webmasters/answer/9008080 · https://support.google.com/webmasters/answer/34592 · https://publicsuffix.org/list/public_suffix_list.dat · https://github.com/orgs/community/discussions/163649 · https://gist.github.com/lorisleiva/36c0173ddf082f1495d25aca119ace7e · https://github.com/brh55/google-domain-verify · https://groups.google.com/g/google-calendar-api/c/1wVxEZL3gM8 · https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/get-started/create-remote-tunnel/ · https://pkg.cloudflare.com/index.html · https://community.cloudflare.com/t/50-user-limit-on-free-plan/546057 · https://nango.dev/blog/how-to-build-a-real-time-google-calendar-api-integration/ · https://www.codewords.ai/blog/google-calendar-webhooks
