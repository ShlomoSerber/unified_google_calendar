---
paths:
  - "src/**"
  - "index.html"
  - "vite.config.ts"
  - "vitest.setup.ts"
---

# Frontend rules

- The frontend is a renderer. It never expands recurrences, never computes conflicts, never stores tokens, never talks to Google. It calls the commands in `docs/02-arquitectura.md` section 5 through `src/ipc/` and renders the payload.
- Visual system: Material 3 as specified in `docs/11-material3.md`. Controls are `@material/web` custom elements (`<md-*>`), registered once in `src/m3/register.ts` (one import per component used; never `all.js`) and typed in `src/m3/jsx.d.ts`. Non-string values are passed as properties, custom-element events are listened to with lowercase `on<event>` props (`onchange`, `onclosed`, `oncancel`), never React's `onClick`/`onChange` on a `<md-*>` element. Every `md-icon-button` has an `aria-label`; every field has a `label`.
- All CSS values come from `var(--md-sys-*)`, `var(--md-ref-*)`, `var(--ugc-*)`, `var(--md-<component>-*)` (component tokens of `@material/web`, always set to a `var()`), or `var(--data-*)` (runtime data such as a calendar's colour, set inline by the component). Never write a literal px, color, font-size, weight, radius, shadow or duration in a component stylesheet. Exception: `0`, `100%`, `auto`, `1fr`, and `calc()` that only combines tokens. Spacing only with `--ugc-space-<n>`, radii only with `--md-sys-shape-corner-*`, shadows only with `--md-sys-elevation-level<n>`, typography with `.md-typescale-<role>` or the `--md-sys-typescale-*` tokens.
- If a needed layout token does not exist, add it to `docs/design/m3/theme.json` on the 4 px grid with a `<name>_note`, run `node scripts/gen-m3-tokens.mjs`, and continue. Do not approximate in CSS.
- Fonts: only the families declared in `src/styles/fonts.css` (Roboto, Google Sans Flex, Material Symbols Outlined), all bundled. Nothing is loaded from the network. Icons only as `<md-icon>name</md-icon>` with names from `docs/design/m3/icons.txt`; no inline SVG paths.
- One component per file, PascalCase file name, a `.css` file with the same name next to it, class names prefixed with the component name in kebab-case. No empty nodes, no numbered wrapper classes, no hidden duplicates of visible text (use `aria-label`).
- The app's own interactive elements (grid days, day numbers, chips, list rows) carry `<md-ripple>` and `<md-focus-ring>` inside a `position: relative` container. No hand-drawn state layers, no `installRipples`, no `data-tooltip`; tooltips through the `Tooltip` component.
- State: zustand store in `src/state/` holds only UI state (current date, view, open dialog). Data comes from `get_view` and is refreshed on `calendar:updated`.
- Dialogs are `md-dialog` (native top layer). Popups (event detail, tooltip, date picker) are DOM elements positioned with JS (`popupPosition`, `columnRect`). Never open a second Tauri window.
- No timers in JS except UI animation. Periodic work lives in Rust.
- Keyboard shortcuts, drag and drop, search and Tasks are version 2. Do not add them. What `@material/web` brings for free (Escape, Tab, arrows in menus and radios, focus rings) is fine.
- Tests: `src/m3/register.ts` is mocked in `vitest.setup.ts`; component tests query the app's own roles and `aria-label`s, never the inside of a `<md-*>`. `npm run lint && npm run typecheck && npm test && node scripts/check-tokens.mjs` must pass before every commit.
