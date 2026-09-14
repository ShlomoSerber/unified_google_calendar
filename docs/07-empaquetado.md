# 07 — Empaquetado e instalación

Fuente: `docs/research/tauri.md` sección 7 y 11.

## 1. Dependencias de build en Ubuntu 25.04

```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev \
  python3-fonttools
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"
rustc --version   # 1.85 o superior
node --version    # 20 ya instalado
```

`python3-fonttools` provee `pyftsubset` para el subset de Material Symbols. Rust no está instalado en la máquina del usuario al momento de escribir esto.

## 2. Configuración de Tauri

`src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Unified Google Calendar",
  "version": "0.1.0",
  "identifier": "com.greelow.unifiedgooglecalendar",
  "mainBinaryName": "unified-google-calendar",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist",
    "removeUnusedCommands": true
  },
  "app": {
    "windows": [{
      "title": "Unified Google Calendar",
      "label": "main",
      "width": 1440, "height": 900, "minWidth": 900, "minHeight": 600
    }],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["deb"],
    "category": "Productivity",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.png"],
    "resources": ["resources/fonts/*"],
    "linux": {
      "deb": {
        "depends": ["libayatana-appindicator3-1", "evolution-data-server"],
        "section": "utils",
        "desktopTemplate": "./unified-google-calendar.desktop"
      }
    }
  }
}
```

`libwebkit2gtk-4.1-0` y `libgtk-3-0` los agrega Tauri solo. `depends` se suma a esos.

`src-tauri/unified-google-calendar.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=Unified Google Calendar
Comment=All your Google accounts in one calendar
Exec=unified-google-calendar
Icon=unified-google-calendar
Terminal=false
Categories=Office;Calendar;
StartupWMClass=unified-google-calendar
X-GNOME-UsesNotifications=true
```

`src-tauri/Cargo.toml`, perfil de release:

```toml
[profile.release]
codegen-units = 1
lto = true
opt-level = "s"
panic = "abort"
strip = true
```

## 3. Build

```bash
npm ci
TAURI_LINUX_AYATANA_APPINDICATOR=1 npm run tauri build -- --bundles deb
ls src-tauri/target/release/bundle/deb/
# unified-google-calendar_0.1.0_amd64.deb
```

La variable de entorno hace que el `.deb` dependa de `libayatana-appindicator3-1`, que es lo que Ubuntu 25.04 trae, y no de `libappindicator3-1`.

Layout dentro del paquete: binario en `/usr/bin/unified-google-calendar`, recursos en `/usr/lib/Unified Google Calendar/`, íconos en `/usr/share/icons/hicolor/`, desktop en `/usr/share/applications/`.

## 4. Instalación y desinstalación

```bash
sudo apt install ./src-tauri/target/release/bundle/deb/unified-google-calendar_0.1.0_amd64.deb
# desinstalar
sudo apt remove unified-google-calendar
```

Datos del usuario que el paquete no toca:

| Ruta | Contenido |
|---|---|
| `~/.config/unified-google-calendar/oauth.json` | Credenciales del proyecto de Google Cloud. Lo crea el usuario. |
| `~/.config/unified-google-calendar/verify/` | Archivo de verificación de Search Console, opcional. |
| `~/.local/share/unified-google-calendar/data.db` | SQLite. |
| `~/.local/share/unified-google-calendar/tokens.bin` | Tokens cifrados. |
| `~/.local/share/unified-google-calendar/logs/` | Logs con rotación. |
| `~/.config/evolution/sources/ugc-*.source` | Fuentes espejo en EDS. Se borran al quitar cuentas desde la app. |

`scripts/uninstall-data.sh` borra todo eso y las fuentes EDS, previa confirmación. No forma parte del `.deb`.

## 5. Primer arranque

1. Si no existe `oauth.json`, la ventana muestra una pantalla de bienvenida con los pasos de `09-setup-usuario.md` y un botón "I created the file, continue".
2. Pantalla "Add your first Google account" con el botón que dispara `add_account`.
3. Tras la primera cuenta: diálogo de Online Accounts de `06-integracion-gnome.md` sección 4.6.
4. Settings: pedir `public_base_url` del Funnel. Botón "Test" que llama `/healthz`. Si falla, se puede omitir y queda polling cada 60 s.
5. Vista de semana.

## 6. Versionado

`version` en `tauri.conf.json`, `package.json` y `Cargo.toml` van siempre iguales. `scripts/bump-version.sh <x.y.z>` los cambia juntos. Sin auto actualización: el usuario instala cada `.deb` a mano.
