# 09 — Setup que hace el usuario a mano

Estos pasos los hace el usuario, no el modelo que implementa. El modelo puede pedirle que los ejecute cuando la fase lo requiera. Cada paso dice en qué fase de `08-plan-de-implementacion.md` hace falta.

## A. Proyecto en Google Cloud y credenciales OAuth

Hace falta en la fase 2. Se hace con la cuenta de Greelow. Si el paso A5 no permite "External", repetir todo con el Gmail personal.

1. Entrar a https://console.cloud.google.com con la cuenta de Greelow. Crear un proyecto nuevo llamado `unified-google-calendar`.
2. Menú → APIs & Services → Library. Buscar "Google Calendar API" y habilitarla.
3. APIs & Services → OAuth consent screen (en consolas nuevas se llama "Google Auth Platform" → "Branding").
4. App name `Unified Google Calendar`, user support email y developer contact: tu correo. Sin logo.
5. Audience: **External**. Si la opción no aparece o está bloqueada por la organización, parar acá y usar el Gmail personal.
6. Scopes: agregar `.../auth/calendar`, `openid`, `.../auth/userinfo.email`. Guardar.
7. Publishing status: pasar a **In production**. Va a decir que la app no está verificada. Aceptar. En "Testing" los refresh tokens caducan a los 7 días y la app pediría login cada semana.
8. APIs & Services → Credentials → Create credentials → OAuth client ID → Application type **Desktop app**. Nombre `unified-google-calendar-desktop`.
9. Copiar Client ID y Client secret y crear el archivo:

```bash
mkdir -p ~/.config/unified-google-calendar
cat > ~/.config/unified-google-calendar/oauth.json <<'EOF'
{ "client_id": "PEGAR_CLIENT_ID", "client_secret": "PEGAR_CLIENT_SECRET" }
EOF
chmod 600 ~/.config/unified-google-calendar/oauth.json
```

Qué esperar al agregar cada cuenta en la app: Google muestra "Google hasn't verified this app". Click en "Advanced" → "Go to Unified Google Calendar (unsafe)". Es normal para apps personales.

Si una cuenta Workspace muestra "Access blocked: Authorization Error" con `Error 400: admin_policy_enforced`, el admin de esa organización bloqueó apps de terceros. Solo el admin lo destraba marcando el Client ID como Trusted en Admin console → Security → API controls → App access control. Esa cuenta no entra a la app hasta entonces.

## B. Tailscale y Funnel

Hace falta en la fase 5. Sin esto la app funciona con polling cada 60 s.

1. Instalar Tailscale con el repo oficial para Ubuntu 25.04:

```bash
sudo mkdir -p --mode=0755 /usr/share/keyrings
curl -fsSL https://pkgs.tailscale.com/stable/ubuntu/plucky.noarmor.gpg | sudo tee /usr/share/keyrings/tailscale-archive-keyring.gpg >/dev/null
curl -fsSL https://pkgs.tailscale.com/stable/ubuntu/plucky.tailscale-keyring.list | sudo tee /etc/apt/sources.list.d/tailscale.list
sudo apt-get update && sudo apt-get install -y tailscale
sudo tailscale up
```

Abrir la URL que imprime y crear la cuenta o entrar. El plan Personal es gratis e incluye Funnel.

2. En https://login.tailscale.com/admin/dns: activar **MagicDNS** y **HTTPS Certificates**. Opcional: renombrar el tailnet para tener un hostname más corto.
3. En https://login.tailscale.com/admin/acls: agregar al policy file:

```json
"nodeAttrs": [
  { "target": ["autogroup:member"], "attr": ["funnel"] }
]
```

O usar el botón "Add Funnel to policy" que aparece la primera vez que se corre `tailscale funnel`.

4. Exponer el puerto de la app, montado en la raíz para que el path llegue intacto:

```bash
sudo tailscale funnel --bg --https=443 8080
tailscale funnel status
```

`--bg` hace que persista tras reinicios. El hostname es `https://<nombre-de-pc>.<tailnet>.ts.net`. Los registros DNS públicos pueden tardar hasta 10 minutos.

5. Con la app corriendo, comprobar desde cualquier red:

```bash
curl -i https://<nombre-de-pc>.<tailnet>.ts.net/healthz
```

Tiene que responder `200 ok`.

6. En la app, Settings → Push notifications → pegar la URL base y presionar Test.

Nota: el hostname queda en los logs públicos de Certificate Transparency, como cualquier certificado de Let's Encrypt.

## C. Verificación de dominio en Google, solo si hace falta

Google documenta que la verificación de dominio para push "ya no es necesaria". La app intenta `events.watch` directo. Si Google rechaza el webhook con un error de dominio no autorizado, hacer esto:

1. https://search.google.com/search-console → Add property → **URL prefix** → `https://<nombre-de-pc>.<tailnet>.ts.net/`.
2. Método "HTML file". Descargar el `google<token>.html`.
3. Copiarlo a `~/.config/unified-google-calendar/verify/`. La app lo sirve en `/google<token>.html`.
4. Verify en Search Console.
5. https://console.cloud.google.com → APIs & Services → Domain verification → Add domain → el mismo hostname.
6. En la app, Settings → Push notifications → Retry.

`ts.net` está en la Public Suffix List, así que no se puede verificar como dominio completo, solo como prefijo de URL. Es el mismo caso que `github.io` o `ngrok.io`, que sí se verifican por archivo HTML.

## D. Online Accounts de GNOME

Hace falta en la fase 6. La app lo ofrece hacer sola. Si el usuario prefiere hacerlo a mano:

Settings → Online Accounts → cada cuenta de Google → apagar el interruptor **Calendar**. El correo y los contactos siguen. Así el panel de GNOME muestra solo lo que escribe la app y no duplica eventos.

## E. Feriados de Argentina

Hace falta en la fase 3. La app lo hace sola la primera vez que se agrega una cuenta Gmail personal: suscribe `en.ar#holiday@group.v.calendar.google.com` con `calendarList.insert`. Si el usuario prefiere otra cuenta, puede cambiarlo en Settings. También se puede hacer a mano en calendar.google.com → Other calendars → Browse calendars of interest → Regional holidays → Argentina.

## F. Perfil de Chrome para medir la UI

Hace falta en la fase 4 y en la 7. Ver `04-fidelidad-visual.md` sección 2. Resumen:

```bash
google-chrome --user-data-dir="$HOME/.chrome-measure" --window-size=1440,900 https://calendar.google.com
```

Iniciar sesión una vez con la cuenta principal, ajustar Settings de Google Calendar como dice el documento 04 y crear los eventos de prueba de `docs/design/fixture-events.md`.
