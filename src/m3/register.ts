// Registers the @material/web custom elements the app uses, one import per component
// (docs/11-material3.md section 5). Imported by src/main.tsx before App; mocked away in
// vitest.setup.ts because jsdom has no ElementInternals. Never import all.js.
import '@material/web/button/filled-button.js';
import '@material/web/button/text-button.js';
import '@material/web/checkbox/checkbox.js';
import '@material/web/chips/chip-set.js';
import '@material/web/chips/filter-chip.js';
import '@material/web/dialog/dialog.js';
import '@material/web/divider/divider.js';
import '@material/web/focus/md-focus-ring.js';
import '@material/web/icon/icon.js';
import '@material/web/iconbutton/icon-button.js';
import '@material/web/list/list.js';
import '@material/web/list/list-item.js';
import '@material/web/menu/menu.js';
import '@material/web/menu/menu-item.js';
import '@material/web/progress/circular-progress.js';
import '@material/web/radio/radio.js';
import '@material/web/ripple/ripple.js';
import '@material/web/select/outlined-select.js';
import '@material/web/select/select-option.js';
import '@material/web/switch/switch.js';
import '@material/web/textfield/outlined-text-field.js';
