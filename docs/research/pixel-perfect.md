# Réplica pixel-perfect de Google Calendar web en Tauri 2 / WebKitGTK 2.50 — investigación

Fecha: 2026-09-14. Todas las afirmaciones citan una URL consultada durante esta investigación; lo que no se pudo verificar se marca como **[no verificado]**.

## 1. Fuentes tipográficas

### Qué usa Google Calendar web hoy
- Páginas guardadas desde `calendar.google.com/calendar/u/0/embed` y `.../companion` (código de Google, no de terceros) declaran `body, html {font-family:"Google Sans Text","Google Sans",Helvetica,Arial,sans-serif;}` e incrustan `Material Icons Extended` desde `fonts.gstatic.com` — [embed.html guardado](https://github.com/joaorodriguesz7/Portifolio-/blob/main/index_files/embed.html), [companion.html guardado](https://github.com/sadeenhmouda/TrainTrack/blob/main/public/static/logos/dana_files/companion.html).
- Un dataset de auditoría axe sobre `https://calendar.google.com/` muestra SVG de Google con `font-family="Google Sans Text" font-weight="500" fill="rgb(95,99,104)"` y contraste `#1a73e8` sobre `#5f6368` — [AccessGuruLLM dataset](https://github.com/NadeenAhmad/AccessGuruLLM/blob/main/data/baseline_two_dataset.csv).
- Una página de Google guardada con tokens GM3 usa el stack nuevo: `font-family: var(--gm3-tooltip-plain-supporting-text-font, "Google Sans Flex", "Google Sans Text", "Google Sans", Roboto, Arial, sans-serif)` y `var(--gm3-sys-color-inverse-on-surface, #f2f2f2)` — [app.html guardado](https://github.com/mostafa7html7-pixel/Mostafa_Abu/blob/main/app.html). Calendar Android adoptó Google Sans Flex con Material 3 Expressive en 2025 — [9to5google](https://9to5google.com/2025/08/07/google-calendar-material-3-expressive-redesign/). **Conclusión**: el stack real hoy es probablemente `"Google Sans Flex","Google Sans Text","Google Sans",Roboto,Arial,sans-serif` con la fuente efectiva dependiendo de la superficie; hay que confirmarlo con el snippet de la sección 2 (campo `fontFamily`).

### Licencias y obtención
| Fuente | Estado legal | Cómo obtener |
|---|---|---|
| Google Sans Flex | OFL 1.1, open source desde 2025 ([OMG! Ubuntu](https://www.omgubuntu.co.uk/2025/11/google-sans-flex-font-ubuntu), [Google Design](https://design.google/library/google-sans-flex-font)). Sin Reserved Font Name, pero "Google", "Google Sans" y "Google Sans Flex" son marcas: no usarlas en nombre de producto/empresa ([TRADEMARKS.md](https://raw.githubusercontent.com/google/fonts/main/ofl/googlesansflex/TRADEMARKS.md)). | `google/fonts/ofl/googlesansflex/GoogleSansFlex[GRAD,ROND,opsz,slnt,wdth,wght].ttf` + `OFL.txt` ([repo](https://github.com/google/fonts/tree/main/ofl/googlesansflex)); también en fonts.google.com. |
| Roboto | OFL ([google/fonts/ofl/roboto](https://github.com/google/fonts/tree/main/ofl/roboto)) | `Roboto[wdth,wght].ttf`, `Roboto-Italic[wdth,wght].ttf` del mismo directorio. |
| Google Sans / Google Sans Text | Propietarias; Google responde "This is not one of them. Please see fonts.google.com…" ([getButterfly](https://getbutterfly.com/google-sans/)); "Not available in Google Fonts or for public use" ([ideastoreach](https://www.ideastoreach.com/blog/font-used-by-google)). **No redistribuibles**: no empaquetar los TTF en la app. | Solo vía API en runtime (ver abajo), sin licencia explícita → zona gris. |
| Material Symbols Outlined | Apache 2.0 ([guía oficial](https://developers.google.com/fonts/docs/material_symbols), [repo](https://github.com/google/material-design-icons)). | Ver "Self-host" abajo. |

### ¿Sirve fonts.googleapis.com las fuentes propietarias a terceros?
Sí, técnicamente. Consultado hoy `https://fonts.googleapis.com/css?family=Google+Sans:400,500,700|Google+Sans+Text:400,500,700|Roboto:400,500,700` devuelve `@font-face` para las tres familias con `src` en `fonts.gstatic.com/s/googlesans/v70/…ttf`, `fonts.gstatic.com/s/googlesanstext/v29/…ttf` y `roboto/v51` (formato truetype). getButterfly documenta las mismas URLs (`css?family=Google+Sans:100,300,…` y `css2?family=Google+Sans:wght@100;…&display=swap`) y señala que la familia "no aparece en búsquedas pero es accesible por API" ([getButterfly](https://getbutterfly.com/google-sans/)). `css2?family=Google+Sans+Flex:opsz,wght@6..144,300..800` devuelve estáticas 300–800 desde `googlesansflex/v22` (sin eje opsz en el CSS servido, en esta consulta). La [guía de la API](https://developers.google.com/fonts/docs/getting_started) documenta `css?family=`, `display=`, `text=` sin límites de uso, pero no otorga licencia sobre Google Sans/Text.

### Recomendación
1. **Empaquetar localmente** (OFL): `GoogleSansFlex[…].ttf` (o instancias woff2 generadas con `pyftsubset --flavor=woff2`), `Roboto[wdth,wght].ttf`, y el subset de Material Symbols. Declararlas con `@font-face` bajo los **mismos nombres de familia** que usa Google (`"Google Sans Flex"`, `"Roboto"`) para que el CSS copiado resuelva igual.
2. Para "Google Sans Text" (si el snippet confirma que sigue siendo la fuente efectiva en algún componente): cargar en runtime desde `fonts.googleapis.com` con `display=swap`, con fallback a Google Sans Flex/Roboto. Es la opción que Google mismo usa, pero sin licencia escrita; aceptable para app personal no distribuida, no para distribuir.
3. **Material Symbols self-host**: descargar `variablefont/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf` del [repo](https://github.com/google/material-design-icons) y subsetear con fonttools: `pyftsubset MaterialSymbolsOutlined*.ttf --text="add,chevron_left,…" --layout-features='liga' --flavor=woff2` ([docs pyftsubset](https://fonttools.readthedocs.io/en/latest/subset/index.html); los iconos son ligaduras, verificar que el subset conserve GSUB). Alternativa: el parámetro `icon_names` de la API (`css2?family=Material+Symbols+Outlined&icon_names=home,palette,settings&display=block`, "295 KB → 1.7 KB") y guardar los TTF que devuelve, o el paquete `@fontsource-variable/material-symbols-outlined` ([fontsource](https://fontsource.org/fonts/material-symbols-outlined)). Ejes: FILL 0–1, wght 100–700, GRAD −50…200, opsz 20–48.

## 2. Metodología de medición

### (a) Snippet DevTools: volcado de layout + estilos computados de una región
Abrir calendar.google.com en Chrome, seleccionar la región en Elements (queda como `$0`) o pasar un selector, pegar en Console:

```js
// dumpRegion(rootEl | selector, opts) -> JSON con rect + estilos por elemento
(() => {
  const PROPS = ['display','position','fontFamily','fontSize','fontWeight','lineHeight',
    'letterSpacing','color','backgroundColor','borderTop','borderRight','borderBottom',
    'borderLeft','borderRadius','paddingTop','paddingRight','paddingBottom','paddingLeft',
    'marginTop','marginRight','marginBottom','marginLeft','boxShadow','opacity','width',
    'height','minWidth','minHeight','gap','flexDirection','alignItems','justifyContent',
    'textTransform','whiteSpace','overflow','zIndex','fill','outline','cursor'];
  const DEFAULTS = (() => { // valores por defecto de un <div> para omitir ruido
    const d = document.createElement('div'); document.body.appendChild(d);
    const cs = getComputedStyle(d); const o = {}; PROPS.forEach(p => o[p] = cs[p]);
    d.remove(); return o; })();
  window.dumpRegion = function (root = $0, { maxDepth = 60, keepDefaults = false, hidden = false } = {}) {
    if (typeof root === 'string') root = document.querySelector(root);
    const rootRect = root.getBoundingClientRect();
    const out = { url: location.href, dpr: devicePixelRatio, viewport: [innerWidth, innerHeight],
      theme: document.documentElement.getAttribute('data-theme') || null,
      colorScheme: getComputedStyle(document.documentElement).colorScheme,
      rootRect: [rootRect.x, rootRect.y, rootRect.width, rootRect.height], nodes: [] };
    const walk = (el, path, depth) => {
      if (depth > maxDepth) return;
      const cs = getComputedStyle(el); const r = el.getBoundingClientRect();
      if (!hidden && (cs.display === 'none' || cs.visibility === 'hidden' || (r.width === 0 && r.height === 0))) return;
      const style = {};
      for (const p of PROPS) if (keepDefaults || cs[p] !== DEFAULTS[p]) style[p] = cs[p];
      const text = [...el.childNodes].filter(n => n.nodeType === 3).map(n => n.textContent.trim()).filter(Boolean).join(' ');
      out.nodes.push({ path, tag: el.tagName.toLowerCase(), cls: el.className && String(el.className).trim(),
        role: el.getAttribute('role'), aria: el.getAttribute('aria-label'), text: text.slice(0, 80) || undefined,
        rect: [r.x - rootRect.x, r.y - rootRect.y, r.width, r.height].map(v => +v.toFixed(2)),
        abs: [r.x, r.y].map(v => +v.toFixed(2)), style });
      let i = 0; for (const c of el.children) walk(c, `${path}/${c.tagName.toLowerCase()}[${i++}]`, depth + 1);
    };
    walk(root, root.tagName.toLowerCase(), 0);
    const json = JSON.stringify(out, null, 1);
    try { copy(json); console.log('JSON copiado al portapapeles'); } catch {}
    const a = Object.assign(document.createElement('a'), { href: URL.createObjectURL(new Blob([json], { type: 'application/json' })),
      download: `gcal-${(root.getAttribute('aria-label') || root.tagName).replace(/\W+/g, '_')}-${Date.now()}.json` });
    a.click(); console.log(`${out.nodes.length} nodos`); return out;
  };
  console.log('Uso: dumpRegion($0) | dumpRegion("[role=main]") | dumpRegion(el,{keepDefaults:true})');
})();
```
Notas: `getBoundingClientRect` da valores fraccionarios; se redondean a centésimas. `rect` es relativo al root para comparar contra nuestra app aunque la ventana difiera. Ejecutar el mismo snippet dentro de la app Tauri (inspector con Ctrl+Shift+I, o `WebviewWindow::open_devtools`; en release requiere la feature `devtools` — [Tauri debug](https://v2.tauri.app/develop/debug/)) y **diffear los dos JSON** (p. ej. `jq` o un script Node) antes de comparar píxeles: detecta desvíos de layout/tipografía independientemente del rasterizado.

### (b) Exportar los tokens `--gm3-*` / `--gm-*`
`getComputedStyle()` no enumera custom properties; la técnica documentada es recorrer `document.styleSheets → cssRules → rule.style` filtrando nombres que empiezan por `--`, y solo funciona en hojas same-origin (las cross-origin lanzan `SecurityError`) — [CSS-Tricks](https://css-tricks.com/how-to-get-all-custom-properties-on-a-page-in-javascript/). Calendar usa `<style nonce>` inline (mismo origen, ver embed guardado arriba). Snippet:

```js
(() => {
  const names = new Set();
  for (const s of document.styleSheets) { let rules; try { rules = s.cssRules; } catch { continue; }
    for (const r of rules) for (const m of (r.cssText.match(/--gm3?-[\w-]+/g) || [])) names.add(m); }
  const cs = getComputedStyle(document.documentElement), out = {};
  [...names].sort().forEach(n => { const v = cs.getPropertyValue(n).trim(); if (v) out[n] = v; });
  copy(JSON.stringify(out, null, 1)); console.log(Object.keys(out).length, 'tokens copiados');
})();
```
Ejecutarlo dos veces: tema claro y tema oscuro (Configuración → Apariencia), guardando `tokens-light.json` / `tokens-dark.json`. Los tokens resueltos en `:root` son la base del `:root` / `[data-theme=dark]` de nuestra app.

### (c) Capturas a DPR 1 con tamaño fijo
- **DevTools (recomendado, sesión logueada)**: Ctrl+Shift+M (device toolbar) → modo "Responsive", escribir ancho/alto (p. ej. 1440×900) → menú ⋮ → "Add device pixel ratio" → elegir **1** ("DPR tells Chrome how many screen pixels to use to draw a CSS pixel") → ⋮ → "Capture screenshot" (viewport) o "Capture full size screenshot" — [device-mode docs](https://developer.chrome.com/docs/devtools/device-mode). También por Command Menu: Ctrl+Shift+P → "screenshot" → *Capture screenshot / full size / node / area* — [guía](https://www.capture-full-page.com/blog/chrome-devtools-screenshot-guide), [DevTools tips](https://developer.chrome.com/blog/devtools-tips-33). "Capture node screenshot" sobre `$0` sirve para recortar exactamente la misma región que `dumpRegion`.
- **Headless**: `chrome --headless --screenshot --window-size=1280,1696 URL` genera `screenshot.png` — [Headless Chrome](https://developer.chrome.com/blog/headless-chrome); pero sin perfil logueado no sirve para Calendar.
- **CDP con sesión**: desde Chrome 136 `--remote-debugging-port`/`--remote-debugging-pipe` se ignoran contra el user-data-dir por defecto; hay que usar `--user-data-dir` no estándar (usa otra clave de cifrado) — [Chrome blog](https://developer.chrome.com/blog/remote-debugging-port). Flujo práctico: `google-chrome --remote-debugging-port=9222 --user-data-dir="$HOME/.chrome-debug-profile"`, iniciar sesión en Calendar **una vez** y reutilizar ese directorio — [raf.dev](https://raf.dev/blog/chrome-debugging-profile-mcp/). Copiar el perfil real es frágil (locks, deriva) — [dassi](https://www.dassi.ai/blog/chrome-remote-debugging-port-browser-agents/). Luego `puppeteer.connect(options)` se adjunta a esa instancia — [pptr.dev](https://pptr.dev/api/puppeteer.puppeteer.connect) — y `page.screenshot()` captura ([pptr.dev](https://pptr.dev/api/puppeteer.page.screenshot)); fijar viewport 1440×900 y DPR 1 en ese script.
- Asegurar escala GNOME 100 % en ambos lados: Chromium fija el DPI web en 96 ([neugierig](https://neugierig.org/software/chromium/fonts/)).

### (c') Capturar nuestra app Tauri (WebKitGTK, Wayland)
En GNOME Wayland no hay D-Bus público para lanzar la herramienta de captura; el hilo oficial acaba simulando la tecla Print (`ydotool key 99:1 99:0`) — [GNOME Discourse](https://discourse.gnome.org/t/screenshot-from-the-command-line-on-wayland/25166). Opciones prácticas **[no verificado]**: lanzar la app con `GDK_BACKEND=x11` y usar `import -window`/`xdotool`, o capturar dentro del WebView (canvas/`html2canvas`) — el diff de JSON de (a) reduce la dependencia de esto.

### (d) Diff de capturas e iteración
- **pixelmatch**: `pixelmatch(img1, img2, output, width, height, {threshold: 0.1, includeAA})` devuelve nº de píxeles distintos; CLI `pixelmatch a.png b.png diff.png 0.1`; ejemplo Node con `pngjs` — [repo](https://github.com/mapbox/pixelmatch).
- **odiff**: `odiff a.png b.png diff.png --threshold … --antialiasing`; `npm i odiff-bin`; "6× más rápido que imagemagick y pixelmatch" — [repo](https://github.com/dmtrKovalenko/odiff).
- **ImageMagick**: `magick compare -metric AE -fuzz 5% a.png b.png diff.png` (AE = píxeles distintos, en stderr); `-compose Src -highlight-color White -lowlight-color Black` da máscara — [usage.imagemagick.org](https://usage.imagemagick.org/compare/).
- Flujo: (1) misma región vía "Capture node screenshot" en Chrome y en la app (mismo viewport, DPR 1, mismo día/eventos de prueba); (2) `dumpRegion` en ambos → diff JSON → corregir layout/tipografía hasta 0 diferencias de `rect`; (3) odiff con `--antialiasing` y umbral ~0.1 para tolerar rasterizado; (4) superponer `diff.png` en la app (overlay semitransparente) para localizar; repetir por vista (día/semana/mes/agenda, popup, quick-create, sidebar, claro/oscuro).

## 3. Constantes de layout publicadas (todas **no verificadas**, confirmar con `dumpRegion`)
- Sidebar principal: la extensión "Google Calendar Resize Sidebar" restablece al "default width (256px)" y permite 200–600 px — [README](https://github.com/ota2000/google_calendar_resize_sidebar).
- Réplica "pixel-faithful" de la vista Tareas (no DOM de Google): sidebar `256px`, botón Crear 14px/500, título 18px/24px, texto 14px/20px, stack `"Google Sans Flex","Google Sans Text","Google Sans",Roboto,Arial` — [praneethb7](https://github.com/praneethb7/google-calendar/blob/main/frontend-next/components/TasksView.jsx).
- Colores que repiten los clones: `#1a73e8`, hover `#1b66c9`, borde `#dadce0`, texto `#3c4043`, secundario `#5f6368` — [ravi-arnan](https://github.com/ravi-arnan/booking-system/blob/main/frontend/src/components/AdminCalendar.css); coinciden con `#1a73e8`/`#5f6368` y `rgb(95,99,104)` vistos en calendar.google.com (AccessGuru, sección 1).
- Embed antiguo (`embed?src=…`, 2022): chips de 1 h con `height: 38px` y `top` 239→279 px (≈40 px/hora); CSS legacy del embed: `.tg-markercell{height:42px}`, `.tg-markercell60{height:60px}`, fuentes Arial/Verdana — [CSSS embed guardado](https://github.com/CSSS/csss-site-events/blob/main/legacy/frosh/2022/frosh_files/embed.html), [CSS embed MIT](https://web.mit.edu/thairop/calendar_files/embed_data/cdbceecd4ab19bd2963ba1251f5bba20embedcompiled_fastui.css). No aplica a la UI principal.
- "48 px por hora", "cabecera 64 px": **no encontrado en ninguna fuente**; Untitled UI (kit Figma genérico, no Google) usa 96 px/hora y gutter 72 px — [Untitled UI](https://www.untitledui.com/components/calendars).

## 4. WebKitGTK vs Chrome: texto y CSS
- `-webkit-font-smoothing` es **solo macOS**; sin efecto en Linux/Windows/Android/iOS — [dbushell](https://dbushell.com/2024/11/05/webkit-font-smoothing/). No sirve para igualar.
- Chromium en Linux: la UI sigue XSETTINGS/Xft (gnome-settings-daemon) y el contenido web "intenta" seguir fontconfig; ambos se desincronizan; DPI web fijo 96 — [neugierig](https://neugierig.org/software/chromium/fonts/). Chromium hard-codea el `lcdfilter`; "webkit-gtk should follow all the settings" de fontconfig — [Arch forum](https://bbs.archlinux.org/viewtopic.php?id=152955). El flag `--disable-font-subpixel-positioning` en realidad **activa** el posicionamiento subpixel (bug conocido) — [gist pandasauce](https://gist.github.com/pandasauce/398c080f9054f05bee6e1c465416b53b).
- WebKitGTK usa Skia desde 2.46 ([webkitgtk.org](https://webkitgtk.org/2024/10/04/webkitgtk-2.46.html)); desde el PR "[GTK] Honor GtkSettings:gtk-font-rendering" (bug 281665, merge 2024-10-21) respeta hinting/antialias/subpixel del sistema cuando `gtk-font-rendering=manual` — [WebKit PR #35360](https://github.com/WebKit/WebKit/pull/35360); en modo manual GTK 4.16+ obedece `gtk-xft-antialias/hinting/hintstyle/rgba` y `gtk-hint-font-metrics` — [GTK docs](https://docs.gtk.org/gtk4/property.Settings.gtk-font-rendering.html). Con hinting activo GTK redondea la posición del glifo solo en Y y mantiene subpixel en X — [GTK blog](https://blogs.gnome.org/gtk/2024/03/07/on-fractional-scales-fonts-and-hinting/). El negrita sintético en Skia lo maneja Skia ("[Skia] Do not set synthetic bold offset", bug 270291) — [webkit-changes](https://www.mail-archive.com/webkit-changes@lists.webkit.org/msg211156.html).
- **Bug abierto relevante**: en Tauri con WebKitGTK 2.50.0 el texto se ve ~100 de peso más grueso que en Chrome, no reproducible en Epiphany, sin workaround — [tauri#14286](https://github.com/tauri-apps/tauri/issues/14286). (El PR "Make bold synthesis less aggressive" #37675 es solo CoreText/macOS — [WebKit PR](https://github.com/WebKit/WebKit/pull/37675).) Mitigación: usar pesos reales de la fuente variable, `font-synthesis: none`, y comprobar en la propia app con `dumpRegion` que `fontWeight` computado coincide.
- Para acercar el render: (1) mismas familias vía `@font-face` locales en ambos (Chrome puede cargar el mismo CSS de fuentes con una user-style o extensión) para evitar sustituciones de fontconfig; (2) `~/.config/fontconfig/fonts.conf` único con `hintstyle=hintslight`, `antialias=true`, `rgba=none`, `lcdfilter=lcddefault`, y `gsettings set org.gnome.desktop.interface font-hinting slight / font-antialiasing grayscale` + `gtk-font-rendering=manual` (GTK 4.16+) para que WebKitGTK use lo mismo **[combinación no verificada]**; (3) DPR 1 y escala 100 %; (4) aceptar que subpixel positioning/hinting no serán bit-idénticos → usar umbral en el diff. Env vars de Tauri para gráficos (`WEBKIT_DISABLE_DMABUF_RENDERER`, `WEBKIT_DISABLE_COMPOSITING_MODE`) no afectan a fuentes — [Tauri Linux graphics](https://v2.tauri.app/develop/debug/linux-graphics/).
- CSS que WebKitGTK 2.50 tiene: `container-progress()`, `text-wrap-style: pretty`, `font-family: math`, `font-variant-emoji`, `border-shape`, CookieStore — [2.50 highlights](https://webkitgtk.org/2025/11/26/webkitgtk-2.50.html). Lo que **falta o es parcial** en 2.50 (verificar con `CSS.supports()`): `field-sizing` (llega en 2.52 — [2.52 highlights](https://webkitgtk.org/2026/03/18/webkitgtk-2.52-highlights.html)); CSS Anchor Positioning (Safari 26 solo parcial, Chrome 125+ — [caniuse](https://caniuse.com/css-anchor-positioning)) → posicionar popups con JS/Floating UI; `scrollbar-color/width` completo solo desde Safari 26.2 ([caniuse](https://caniuse.com/css-scrollbar)) → usar `::-webkit-scrollbar`; `text-box-trim` sí (Safari 18.2+, Chrome 133+ — [caniuse](https://caniuse.com/css-text-box-trim)).

## 5. Nota legal (breve, no es asesoría)
- El "look and feel" de una UI tiene protección débil por copyright (interfaces como "método de operación", *Lotus v. Borland*); la vía viable es *trade dress*, que exige distintividad, no-funcionalidad y **probabilidad de confusión en el mercado** — [Harvard Berkman](https://cyber.harvard.edu/property/protection/resources/byerly_unedited.html). Una app personal, no distribuida ni comercializada, no genera confusión comercial; el riesgo aparece al distribuir.
- Marcas: no usar el logo de Google/Google Calendar ni versiones modificadas como logo de la app; nombres solo como "for Google Calendar™" con atribución "Google Calendar™ is a trademark of Google LLC" — [Marketplace branding](https://developers.google.com/workspace/marketplace/terms/branding); cualquier uso de logos/capturas exige permiso y aviso de marca — [Brand terms](https://partnermarketinghub.withgoogle.com/brands/google/trademarks-and-terms/terms-and-conditions/), [Brand guidance](https://about.google/brand-resource-center/guidance/). **Recomendación: icono propio, sin logo de Google.**
- Fuentes: no redistribuir Google Sans / Google Sans Text (propietarias); Google Sans Flex y Roboto sí (OFL), respetando TRADEMARKS.md; Material Symbols (Apache 2.0) sí.
