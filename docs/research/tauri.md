# Tauri 2 en Ubuntu 25.04 (GNOME 48 / Wayland / WebKitGTK 2.50.4) — investigación para app de calendario

Fecha: 2026-09-14. Fuentes: Context7 (`/tauri-apps/tauri-docs`, `/tauri-apps/plugins-workspace`, `/websites/tauri_app`), docs.rs, v2.tauri.app, webkitgtk.org, webkit.org, developers.google.com. Cada afirmación lleva la URL de donde salió.

Versiones observadas: `tauri-plugin-notification` 2.4.0 (usa `notify-rust` 4.11, [Cargo.toml](https://raw.githubusercontent.com/tauri-apps/plugins-workspace/v2/plugins/notification/Cargo.toml)); `notify-rust` 4.18.0 ([docs.rs](https://docs.rs/notify-rust/latest/notify_rust/)); `tauri-plugin-oauth` 2.1.0 ([docs.rs](https://docs.rs/tauri-plugin-oauth/latest/tauri_plugin_oauth/)); `keyring` 4.2.0 ([docs.rs](https://docs.rs/keyring/latest/keyring/)); `aes-gcm` 0.11.1 ([docs.rs](https://docs.rs/aes-gcm/latest/aes_gcm/)); `rusqlite` 0.40.2 ([docs.rs](https://docs.rs/rusqlite/latest/rusqlite/)).

---

## 1. System tray en Linux

**Feature flag** ([learn/system-tray](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/learn/system-tray.mdx)):
```toml
tauri = { version = "2", features = ["tray-icon"] }
```

**Libs**: en Linux el tray se implementa con `libappindicator` o `libayatana-appindicator` (crate `tray-icon`, feature `libappindicator` por defecto; existe alternativa `ksni` = StatusNotifierItem por D-Bus, con menos deps del sistema) — [tray-icon README](https://raw.githubusercontent.com/tauri-apps/tray-icon/dev/README.md). Build dep en Ubuntu: `libayatana-appindicator3-dev` ([prerequisites](https://v2.tauri.app/start/prerequisites/)). En runtime el `.deb` generado declara `libappindicator3-1` salvo que exportes `TAURI_LINUX_AYATANA_APPINDICATOR=1` al hacer `tauri build`, en cuyo caso declara `libayatana-appindicator3-1` (código en [tauri-cli rust.rs](https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-cli/src/interface/rust.rs); la variable está documentada en [environment-variables](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/reference/environment-variables.mdx): "force usage of libayatana-appindicator for system tray on Linux"). Como Ubuntu 25.04 ya tiene `libayatana-appindicator3` instalado, compila siempre con esa variable. GNOME necesita la extensión `ubuntu-appindicators` (ya activa) para mostrar AppIndicators; sin ella no aparece nada.

**Título dinámico / tooltip** ([docs.rs TrayIcon](https://docs.rs/tauri/latest/tauri/tray/struct.TrayIcon.html)):
- `set_title(&self, title: Option<S>)` — "Linux: shows the title only when an icon exists; suits frequently updated info". Sirve para "Próximo: Standup 10:30" al lado del icono.
- `set_tooltip(&self, tooltip: Option<S>)` — **Linux: no soportado**. No cuentes con tooltip en GNOME.
- `set_icon(Option<Image>)`, `set_menu(Option<M>)` (Linux: una vez puesto el menú no se puede quitar), `set_visible(bool)`.
- Builder ([docs.rs TrayIconBuilder](https://docs.rs/tauri/latest/tauri/tray/struct.TrayIconBuilder.html)): `with_id`, `icon`, `menu`, `title`, `tooltip`, `temp_dir_path` (solo Linux: dónde escribe el PNG del icono), `show_menu_on_left_click` (Linux: no soportado), `on_menu_event`, `on_tray_icon_event`, `build`.

**Click izquierdo en GNOME**: la doc oficial dice para `Click`/`DoubleClick`/`Enter`/`Move`/`Leave`: "Linux: Unsupported. The event is not emitted even though the icon is shown and will still show a context menu on right click" ([learn/system-tray](https://v2.tauri.app/learn/system-tray/)). Con AppIndicator **cualquier click abre el menú**; no hay "click izquierdo = mostrar ventana". Solución: primer item del menú "Abrir calendario" + item "Salir".

```rust
use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder, Manager};

let open_i = MenuItem::with_id(app, "open", "Abrir calendario", true, None::<&str>)?;
let quit_i = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
let menu = Menu::with_items(app, &[&open_i, &quit_i])?;
let tray = TrayIconBuilder::with_id("main")
    .icon(app.default_window_icon().unwrap().clone())
    .menu(&menu)
    .title("Sin eventos")                 // texto junto al icono (Linux OK)
    .on_menu_event(|app, ev| match ev.id().as_ref() {
        "open" => { if let Some(w) = app.get_webview_window("main") {
            let _ = w.unminimize(); let _ = w.show(); let _ = w.set_focus(); } }
        "quit" => app.exit(0),
        _ => {}
    })
    .build(app)?;
// más tarde, desde cualquier tarea:
// app.tray_by_id("main").unwrap().set_title(Some("Standup 10:30"))?;
```
(Patrón `on_menu_event`/`get_webview_window`/`show`/`set_focus` tomado de [migrate/from-tauri-1](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/start/migrate/from-tauri-1.mdx).) Si quieres sin AppIndicator ni extensión, evalúa la feature `ksni` de `tray-icon` (SNI directo por D-Bus).

## 2. Notificaciones

**`tauri-plugin-notification` en Linux**: el backend de escritorio instancia `notify_rust::Notification::new()` y mapea solo `summary` (title), `body`, `icon`/`auto_icon` y `sound_name` ([desktop.rs](https://raw.githubusercontent.com/tauri-apps/plugins-workspace/v2/plugins/notification/src/desktop.rs)). `notify-rust` habla D-Bus `org.freedesktop.Notifications` (backend `zbus` por defecto; `d` = dbus-rs) ([README](https://raw.githubusercontent.com/hoodie/notify-rust/main/README.md)). **Botones de acción: NO en desktop** — `init()` solo registra `notify`, `request_permission`, `is_permission_granted`; `registerActionTypes` falla en IPC; la doc marca Actions/Attachments/Channels como mobile-only ([lib.rs](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/notification/src/lib.rs), [plugin/notification](https://v2.tauri.app/plugin/notification/)).

Uso Rust del plugin (permiso `notification:default`):
```rust
use tauri_plugin_notification::NotificationExt;
app.notification().builder().title("Reunión en 10 min").body("Standup — Meet").show()?;
```

**Recomendación: `notify-rust` directo desde Rust** (el plugin no aporta nada en Linux y bloquea acciones). Ejemplo de acciones ([docs.rs notify-rust](https://docs.rs/notify-rust/latest/notify_rust/)):
```toml
notify-rust = "4"   # default = zbus
```
```rust
use notify_rust::{Notification, Hint, Timeout};
let handle = Notification::new()
    .appname("Unified Calendar")
    .summary("Standup en 10 min")
    .body("10:30 · Google Meet")
    .icon("x-office-calendar")
    .action("join", "Unirse")
    .action("snooze", "Posponer 5 min")
    .hint(Hint::Resident(true))
    .timeout(Timeout::Never)
    .show()?;
// no bloquear el hilo principal:
tauri::async_runtime::spawn(async move {
    handle.wait_for_action_async(|a| match a { "join" => {/*open_url*/}, "snooze" => {}, "__closed" => {}, _ => {} }).await;
});
```
`NotificationHandle::close()` / `update()` permiten reemplazar la misma notificación. GNOME muestra las acciones como botones en el banner/centro de notificaciones.

## 3. Cerrar a la bandeja + instancia única

`WindowEvent::CloseRequested { api: CloseRequestApi }` con `api.prevent_close()` ([docs.rs WindowEvent](https://docs.rs/tauri/latest/tauri/enum.WindowEvent.html)). En Tauri 2 `WebviewWindow::close()` dispara `CloseRequested`; `destroy()` fuerza el cierre ([blog tauri-2.0](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/blog/tauri-2.0.mdx)). Cuando se cierra la última ventana llega `RunEvent::ExitRequested { api, .. }` → `api.prevent_exit()` ([develop/Plugins](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/develop/Plugins/index.mdx)); como la ventana solo se oculta, no llega, pero conviene cubrirlo.

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {   // DEBE ser el primer plugin
        if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); }
    }))
    .on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window.hide();
        }
    })
    .build(tauri::generate_context!())?
    .run(|_app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event { api.prevent_exit(); }
    });
```
Single-instance: `cargo add tauri-plugin-single-instance --target 'cfg(any(target_os = "macos", windows, target_os = "linux"))'`; closure `|app, args, cwd|`; "must be the first one to be registered" ([plugin/single-instance](https://v2.tauri.app/plugin/single-instance/)). Desde JS: `getCurrentWindow().onCloseRequested(e => e.preventDefault())` ([api/window](https://tauri.app/reference/javascript/api/namespacewindow)). Para "salir de verdad" desde el menú del tray usa `app.exit(0)`.

## 4. OAuth loopback (Google Desktop)

Reglas de Google ([native-app](https://developers.google.com/identity/protocols/oauth2/native-app)): redirect `http://127.0.0.1:PORT` (o `[::1]`) en un puerto aleatorio libre (`localhost` funciona pero "may cause issues with client firewalls"); PKCE S256 recomendado (verifier 43–128 chars, challenge = base64url(SHA256)); auth en `https://accounts.google.com/o/oauth2/v2/auth` con `client_id, redirect_uri, response_type=code, scope, code_challenge, code_challenge_method=S256, state`; token en `https://oauth2.googleapis.com/token` con `client_id, code, code_verifier, grant_type=authorization_code, redirect_uri` (`client_secret` opcional para Desktop); "refresh tokens are always returned for installed applications".

**Opción A — `tauri-plugin-oauth` 2.1.0** ([docs.rs](https://docs.rs/tauri-plugin-oauth/latest/tauri_plugin_oauth/)): `start() -> port` en 127.0.0.1, `start_with_config(OauthConfig { ports: Option<Vec<u16>>, response: Option<Cow<str>>, redirect_uri: Option<Cow<str>> })`, `cancel(port)`; desde JS `invoke('plugin:oauth|start')` y evento `oauth://url`. Aviso del crate: "you must verify the URL received" (puerto sin proteger). Es comunitario, no oficial.

**Opción B — listener mínimo propio en Rust (recomendada)**: `std::net::TcpListener::bind("127.0.0.1:0")`, leer una petición HTTP, parsear `code`+`state`, responder HTML mínimo, cerrar; luego intercambiar el code con `reqwest` (feature `rustls-tls` para no cargar OpenSSL). Sin dependencia extra y controlas `state`/PKCE. Los tokens nunca pasan por el webview.

Abrir navegador: `tauri-plugin-opener` ([plugin/opener](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/plugin/opener.mdx)):
```rust
use tauri_plugin_opener::OpenerExt;
app.opener().open_url("https://accounts.google.com/o/oauth2/v2/auth?...", None::<&str>)?;
```
JS: `import { openUrl } from '@tauri-apps/plugin-opener'` con permiso `opener:allow-open-url` y scope de URLs (glob). Si la URL se construye en Rust no hace falta exponerlo a JS.

## 5. Base de datos local

- `tauri-plugin-sql` (`cargo add tauri-plugin-sql --features sqlite`): capa sobre **sqlx**, API JS `Database.load('sqlite:app.db')` (ruta relativa a `BaseDirectory::AppConfig`), `execute`/`select`, migraciones vía `Builder::add_migrations("sqlite:app.db", vec![Migration{version,description,sql,kind: MigrationKind::Up}])` y `plugins.sql.preload` en `tauri.conf.json`; permisos `sql:default` + `sql:allow-execute` ([plugin/sql](https://v2.tauri.app/plugin/sql/), [sql.mdx](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/plugin/sql.mdx)). Ventaja: cero código Rust. Contras para tu caso: cada query cruza IPC con JSON; la lógica de sync (diff de eventos, ETags, sync tokens) tendría que vivir en JS o duplicarse; arrastra sqlx + runtime async por cada engine.
- **`rusqlite` en comandos Rust (recomendado para app sync-heavy)** ([docs.rs](https://docs.rs/rusqlite/latest/rusqlite/)): `rusqlite = { version = "0.40", features = ["bundled"] }` (compila SQLite estáticamente, sin depender de libsqlite3 del sistema). `Connection` es `Send` pero no `Sync`: guárdala en `tauri::State<Mutex<Connection>>` o mejor un hilo dedicado con canal. El sync corre íntegro en Rust (tokio + reqwest), escribe en SQLite y emite eventos al frontend con el delta; el webview solo pide vistas (`#[tauri::command] fn events_between(from, to)`), lo que reduce el JS heap (menos RAM) y evita IPC por fila.
- Consejos: `PRAGMA journal_mode=WAL; synchronous=NORMAL`, transacciones por lote de sync, índices por `(calendar_id, start_ts)`.

## 6. Almacenamiento seguro de tokens

- `tauri-plugin-stronghold` ([plugin/stronghold](https://v2.tauri.app/plugin/stronghold/)): `Builder::with_argon2(&salt_path)` o `Builder::new(|password| Vec<u8> /*32 bytes exactos*/)`; API JS `Stronghold.load(vaultPath, password)` → `loadClient` → `getStore().insert/get` → `save()`. Requiere que el usuario/JS aporte una contraseña; añade la pila IOTA Stronghold (pesada, más RAM/binario) y hay bug conocido que obliga a `[profile.dev.package.scrypt] opt-level = 3`. No lo recomiendo para guardar un refresh token.
- **Recomendación: `keyring` 4.x (Secret Service = GNOME Keyring vía D-Bus) + opcionalmente AES-GCM para la caché** ([docs.rs keyring](https://docs.rs/keyring/latest/keyring/)):
```toml
[target.'cfg(target_os = "linux")'.dependencies]
keyring = { version = "4", default-features = false, features = ["sync-secret-service", "crypto-rust"] }
```
```rust
let e = keyring::Entry::new("unified-calendar", "google-refresh-token")?;
e.set_password(&refresh_token)?;  let t = e.get_password()?;  // e.delete_credential()?
```
`linux-native` (keyutils) NO persiste entre reinicios; usa `sync-secret-service`. GNOME lo desbloquea con la sesión, sin prompts.
- Si prefieres archivo cifrado: `aes-gcm` 0.11 (`Aes256Gcm`, nonce 96 bits **único por mensaje**) ([docs.rs](https://docs.rs/aes-gcm/latest/aes_gcm/)). Derivación de clave: genera 32 bytes aleatorios (`Key::<Aes256Gcm>::generate()`), guárdalos en `keyring`, y cifra el fichero `tokens.bin` (nonce || ciphertext) en `app_local_data_dir`. Con esto no hay contraseña de usuario ni Argon2; la clave vive en el llavero del sistema.

## 7. Bundling `.deb`

`tauri build --bundles deb` → salida `target/release/bundle/deb/<productName>_<version>_<arch>.deb` ([bundler debian.rs](https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-bundler/src/bundle/linux/debian.rs)). Layout dentro del paquete (mismo archivo): binario en `usr/bin/<mainBinaryName>`, `bundle.resources` en `usr/lib/<productName>/`, iconos en `usr/share/icons/hicolor/<size>/apps/`, `.desktop` en `usr/share/applications/`. Depends por defecto que añade tauri-cli: `libwebkit2gtk-4.1-0`, `libgtk-3-0` y, con feature `tray-icon`, `libappindicator3-1` o `libayatana-appindicator3-1` según `TAURI_LINUX_AYATANA_APPINDICATOR` ([rust.rs](https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-cli/src/interface/rust.rs)). Claves de `DebConfig` ([reference/config](https://v2.tauri.app/reference/config/)): `depends`, `recommends`, `provides`, `conflicts`, `replaces`, `files`, `section`, `priority` (default `optional`), `changelog`, `desktopTemplate`, `preInstallScript`, `postInstallScript`, `preRemoveScript`, `postRemoveScript`.

```json
{
  "productName": "Unified Calendar",
  "mainBinaryName": "unified-calendar",
  "identifier": "com.greelow.unifiedcalendar",
  "bundle": {
    "active": true,
    "targets": ["deb"],
    "category": "Productivity",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.png"],
    "resources": ["resources/sounds/*.oga"],
    "linux": {
      "deb": {
        "depends": ["libayatana-appindicator3-1", "gnome-shell-extension-appindicator"],
        "recommends": ["gnome-keyring"],
        "section": "utils",
        "desktopTemplate": "./unified-calendar.desktop",
        "files": { "/usr/share/metainfo/com.greelow.unifiedcalendar.metainfo.xml": "./metainfo.xml" }
      }
    }
  }
}
```
`depends` se **suma** a los defaults (webkit/gtk se añaden por código). `desktopTemplate` es una plantilla Handlebars propia (útil para `StartupWMClass`, `X-GNOME-UsesNotifications=true`, `Actions=`). Resolver recursos en runtime: `app.path().resolve("sounds/x.oga", BaseDirectory::Resource)?` / JS `resolveResource()` ([develop/resources](https://v2.tauri.app/develop/resources/)). Aviso de compatibilidad glibc: construye sobre la base más antigua que quieras soportar; Ubuntu 22.04/Debian 12 ya traen `libwebkit2gtk-4.1-dev` ([distribute/rpm](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/distribute/rpm.mdx)).

## 8. RAM

Datos reales en Linux ([tauri#5889](https://github.com/tauri-apps/tauri/issues/5889), Ubuntu 22.04): app por defecto USS 125 MB Tauri vs 118 MB Electron, PSS 185 MB vs 207 MB; con una web pesada (postman.com) Tauri 581 MB vs Electron 240 MB — WebKitGTK escala peor que Chromium con DOM/JS grandes, y el benchmark oficial infla a Electron por memoria compartida. Reportes de crecimiento de `WebKitWebProcess` en sesiones largas ([Handy#1279](https://github.com/cjpais/Handy/issues/1279)) y picos al redimensionar ([tauri#10102](https://github.com/tauri-apps/tauri/issues/10102)). Espera **~120–180 MB en reposo** (proceso Rust + WebKitWebProcess + WebKitNetworkProcess) para una UI modesta; el objetivo es mantener el DOM/JS pequeño.

Medidas concretas:
- Una sola `WebviewWindow`; sin ventanas secundarias (cada webview = otro WebKitWebProcess). Popups → `dialog`/notificaciones nativas.
- Sin `devtools` en release: el inspector solo está en debug salvo que actives la feature `devtools` en Cargo ([develop/Debug](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/develop/Debug/index.mdx)); no la actives.
- Perfil release ([concept/size](https://v2.tauri.app/concept/size/)):
```toml
[profile.release]
codegen-units = 1
lto = true
opt-level = "s"     # o "3" si prima velocidad
panic = "abort"
strip = true
```
  y `"build": { "removeUnusedCommands": true }` en `tauri.conf.json` (tauri ≥ 2.4).
- Lógica y caché de datos en Rust (SQLite), frontend con listas virtualizadas y sin estado duplicado: menos JS heap = menos RAM del WebProcess.
- Evitar timers/polling en JS; el sync vive en tokio y emite eventos (sección 10).
- Avanzado: WebKitGTK expone `MemoryPressureSettings` (`memory_limit`, `conservative_threshold`, `strict_threshold`, `kill_threshold`, `poll_interval`; se aplica al `WebKitWebContext` antes de crear web processes, desde 2.34) ([webkitgtk docs](https://webkitgtk.org/reference/webkit2gtk/stable/struct.MemoryPressureSettings.html)); accesible desde Tauri vía `WebviewWindow::with_webview` + `webkit2gtk` crate. "Badly defined parameters can greatly reduce the performance" — solo si mides un problema.
- Ocultar la ventana (close-to-tray) no libera el WebProcess; si el reposo en bandeja debe ser mínimo, la opción es destruir la ventana al ocultar y recrearla al abrir (coste: rehidratar la UI).

## 9. WebKitGTK en Wayland/Ubuntu — gotchas

Doc oficial [develop/debug/linux-graphics](https://v2.tauri.app/develop/debug/linux-graphics/):
- `WEBKIT_DISABLE_DMABUF_RENDERER=1` — ventana blanca/errores `AcceleratedSurfaceDMABuf` (sobre todo NVIDIA).
- `WEBKIT_DISABLE_COMPOSITING_MODE=1` — desactiva aceleración por completo; último recurso para cierres silenciosos al redimensionar.
- `__NV_DISABLE_EXPLICIT_SYNC=1` — "Error 71 (Protocol error)" en Wayland+NVIDIA.
- Ajuste programático: `#[cfg(target_os="linux")] std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER","1");` al inicio de `main()`, **solo tras confirmar** que tu máquina lo necesita ("disables a faster path for everyone").
- `GDK_BACKEND`: el hook del AppImage fuerza `x11` ([tauri#15781](https://github.com/tauri-apps/tauri/issues/15781)); con `.deb` corres Wayland nativo. Si necesitas fallback: `std::env::set_var("GDK_BACKEND", "wayland,x11")` antes de construir el Builder. Hay reportes de texto borroso tras resize en Wayland ([wry#1727](https://github.com/tauri-apps/wry/issues/1727)); si ocurre, `GDK_BACKEND=x11` es el workaround.
- Fuentes: WebKitGTK pasa por FreeType/Fontconfig; se reporta un offset de peso (texto más "bold" que en Chrome) ([tauri#14286](https://github.com/tauri-apps/tauri/issues/14286)) y no hay subpixel AA en texto compuesto por GPU. Mitiga con `font-weight` explícitos (400/600) y fuente empaquetada (Inter) en vez de `system-ui`.
- WebKitGTK 2.50 ([highlights](https://webkitgtk.org/2025/11/26/webkitgtk-2.50.html), [2.50.0](https://webkitgtk.org/2025/09/17/webkitgtk2.50.0-released.html)): rendering threaded con Skia, damage propagation al compositor, `font-variant-emoji`, `text-wrap-style: pretty`, `container-progress()`, CookieStore, `hidden="until-found"`, Sysprof. Es WebKit de mediados de 2025 (≈ Safari 26).
- CSS que sí puedes usar: `:has()` (Safari 15.4, [webkit.org](https://webkit.org/blog/12445/new-webkit-features-in-safari-15-4/)), container queries de tamaño + unidades `cq*` y subgrid (Safari 16.0, [webkit.org](https://webkit.org/blog/13152/webkit-features-in-safari-16-0/)), `color-mix()` (Safari 16.2, [mdn/bcd#19113](https://github.com/mdn/browser-compat-data/issues/19113)), `@layer`, viewport units `dvh`. Todo muy anterior a 2.50. Evita features Chrome-only (`scrollbar-*` de WebKit sí, pero `::-webkit-scrollbar` con estilos; `field-sizing`, `anchor-positioning`, View Transitions cross-document: verifica antes). Testea siempre en el propio webview, no en Chrome.

## 10. Trabajo en background y eventos

`tauri::async_runtime` **es tokio** (`spawn`, `spawn_blocking`, `block_on`, `handle`, `set`) ([docs.rs](https://docs.rs/tauri/latest/tauri/async_runtime/index.html)). Añade `tokio = { version = "1", features = ["time", "sync"] }` para `interval` ([tokio docs](https://docs.rs/tokio/latest/tokio/time/fn.interval.html)). Emitir: `app.emit("evt", payload)` con `use tauri::Emitter` ([develop/calling-frontend](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/develop/calling-frontend.mdx)); para streams ordenados dentro de un comando, `tauri::ipc::Channel<T>`.

```rust
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{interval, Duration};

pub fn start_sync_loop(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut tick = interval(Duration::from_secs(300));   // el primer tick es inmediato
        loop {
            tick.tick().await;
            match crate::sync::run(&app).await {          // reqwest + rusqlite (spawn_blocking)
                Ok(delta) => { let _ = app.emit("calendar:updated", &delta); }
                Err(e)    => { let _ = app.emit("calendar:sync-error", e.to_string()); }
            }
            if let Some(next) = crate::db::next_event(&app) {
                let _ = app.tray_by_id("main").unwrap().set_title(Some(next.label()));
            }
        }
    });
}
```
Frontend: `import { listen } from '@tauri-apps/api/event'; listen<Delta>('calendar:updated', e => …)`. Para rusqlite dentro de async usa `spawn_blocking`.

Autostart (opcional): `cargo add tauri-plugin-autostart --target 'cfg(any(target_os = "macos", windows, target_os = "linux"))'`; `app.handle().plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))`; `app.autolaunch().enable()/disable()/is_enabled()`; permisos `autostart:allow-enable|disable|is-enabled` ([plugin/autostart](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/plugin/autostart.mdx)). En Linux crea un `.desktop` en `~/.config/autostart`.

## 11. Dependencias apt y rustup (Ubuntu 25.04)

Oficial ([prerequisites](https://v2.tauri.app/start/prerequisites/)):
```sh
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
```
Node LTS solo para el frontend. Plugins exigen Rust ≥ 1.77.2 (notification/stronghold). Para el `.deb` con tray: `TAURI_LINUX_AYATANA_APPINDICATOR=1 npm run tauri build -- --bundles deb`. `libxdo` solo lo usan los items predefinidos Copy/Cut/Paste del menú ([tray-icon README](https://raw.githubusercontent.com/tauri-apps/tray-icon/dev/README.md)).

---

## Decisiones recomendadas (resumen)

1. Tauri 2 + `tray-icon` con libayatana; menú "Abrir/Salir" (no hay click izquierdo en GNOME); `set_title` para el próximo evento, tooltip no existe en Linux.
2. Notificaciones con `notify-rust` directo (D-Bus `org.freedesktop.Notifications`) para tener botones "Unirse/Posponer"; descartar `tauri-plugin-notification` (sin acciones en desktop).
3. Close-to-tray: `on_window_event` → `prevent_close()` + `hide()`; `RunEvent::ExitRequested` → `prevent_exit()`; `tauri-plugin-single-instance` como primer plugin.
4. OAuth: listener propio `TcpListener 127.0.0.1:0` + PKCE S256 en Rust; abrir navegador con `tauri-plugin-opener` desde Rust; tokens nunca en el webview.
5. DB: `rusqlite` (`bundled`, WAL) en comandos Rust; sync completo en tokio; frontend solo consulta vistas. No `tauri-plugin-sql`.
6. Secretos: refresh token en GNOME Keyring vía `keyring` (`sync-secret-service`); si hace falta fichero cifrado, clave aleatoria en keyring + `aes-gcm`. Sin Stronghold.
7. `.deb`: `bundle.targets=["deb"]`, `TAURI_LINUX_AYATANA_APPINDICATOR=1`, `depends` extra + `desktopTemplate`; binario en `/usr/bin`, recursos en `/usr/lib/<productName>`.
8. RAM: una sola ventana, sin feature `devtools`, `profile.release` con lto/opt-level s/panic abort/strip, `removeUnusedCommands`, estado y caché en Rust; esperar ~120–180 MB en reposo.
9. WebKitGTK 2.50: no forzar env vars salvo reproducir el fallo; `:has`, container queries y `color-mix` están soportados; validar CSS en el webview y fijar font-weights.
10. Background: `tauri::async_runtime::spawn` + `tokio::time::interval` + `app.emit`; autostart opcional con `tauri-plugin-autostart`.
