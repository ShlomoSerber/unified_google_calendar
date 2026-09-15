#!/usr/bin/env node
// Measure the motion of calendar.google.com (docs/04 section 10): for every scenario the page is
// put in a known state, the recorder of animRecorder.js is started, the action is triggered and
// every Web Animation (target, keyframes, duration, easing) plus the frame-by-frame box of the
// watched elements is saved. Hover scenarios also diff the computed styles at rest and hovered
// in both themes. Read-only on the user's account: chips are opened and closed, the quick create
// bubble and the full form are opened and discarded, one calendar checkbox is toggled and
// restored. Output: docs/design/measurements/animations-<light|dark>.json.
// Usage: node scripts/measure/animations.mjs [scenario ...]   (default: all)
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { Cdp, launchChrome } from './cdp.mjs';

const OUT = fileURLToPath(new URL('../../docs/design/measurements/', import.meta.url));
const WEEK = 'https://calendar.google.com/calendar/u/0/r/week';
const RECORD_MS = 1600;

const recorder = readFileSync(new URL('./animRecorder.js', import.meta.url), 'utf8');

// JS expressions returning elements on calendar.google.com (same probes as capture.mjs).
// The view button reads "Week\narrow_drop_down": only the first line is matched.
const headerButton = (re) => `[...document.querySelectorAll('header button')].find(b => ${re}.test((b.innerText||'').trim().split('\\n')[0]) && b.getBoundingClientRect().width > 0)`;
const byLabel = (label) => `[...document.querySelectorAll('button,[role=button]')].find(b => (b.getAttribute('aria-label')||'') === ${JSON.stringify(label)} && b.getBoundingClientRect().width > 0)`;
const byLabelPrefix = (prefix) => `[...document.querySelectorAll('button,[role=button]')].find(b => (b.getAttribute('aria-label')||'').startsWith(${JSON.stringify(prefix)}) && b.getBoundingClientRect().width > 0)`;
const menuItem = (re) => `[...document.querySelectorAll('[role=menuitem],[role=menuitemradio],[role=option]')].find(m => ${re}.test((m.innerText||'').trim()) && m.getBoundingClientRect().width > 0)`;
const createButton = `[...document.querySelectorAll('button')].find(b => /Create/.test(b.innerText||'') && b.getBoundingClientRect().x < 200 && b.getBoundingClientRect().width > 0)`;
// First timed chip of the visible week ("HH:MM to HH:MM, title, …").
const firstChip = `[...document.querySelectorAll('[role=main] [data-eventid][role=button]')].find(b => /^\\d\\d:\\d\\d to \\d\\d:\\d\\d, /.test((b.innerText||'').replace(/\\n/g, ' ')) && b.getBoundingClientRect().width > 0 && b.getBoundingClientRect().y > 200 && b.getBoundingClientRect().bottom < 880)`;
const firstCalendarRow = `[...document.querySelectorAll('[role=list][aria-label="My calendars"] [role=presentation]')].find(r => r.getBoundingClientRect().width > 0)`;
const firstCalendarCheckbox = `(() => { const r = ${firstCalendarRow}; return r && r.querySelector('[role=checkbox],input[type=checkbox]'); })()`;
// Mini calendar days are <button aria-label="August 31, Monday"|"15, Tuesday, today"> inside <td>.
const minicalDays = `[...document.querySelectorAll('td button[aria-label]')].filter(b => /^([A-Z][a-z]+ )?\\d{1,2}, [A-Z][a-z]+day(, today)?$/.test(b.getAttribute('aria-label')) && b.getBoundingClientRect().width > 0)`;
const minicalDay = `${minicalDays}[10]`;
const dialogClose = `[...document.querySelectorAll('[role=dialog] button')].find(b => (b.getAttribute('aria-label')||'') === 'Close' && b.getBoundingClientRect().width > 0)`;
const emptySlot = `(() => { const g = document.querySelector('[role=main] [role=grid]'); const cols = [...g.querySelectorAll('[role=gridcell]')].filter(c => c.getBoundingClientRect().height > 500); const c = cols[3] || cols[0]; const r = c.getBoundingClientRect(); return { x: r.x + r.width / 2, y: Math.min(r.bottom - 40, r.y + 60 * 16 - (c.closest('[role=presentation]')?.scrollTop || 0)) }; })()`;

const WATCH = {
  main: '[role=main]',
  mainChildren: '[role=main] > *',
  grid: '[role=main] [role=grid]',
  dialog: '[role=dialog]',
  menu: '[role=menu], [role=listbox]',
  drawer: '[role=complementary]',
  tooltip: '[role=tooltip]',
  minical: '[role=complementary] [role=grid]',
  scrim: '.gb_id, [aria-hidden="true"][style*="opacity"]',
};

/** Scenario: prepare (state before), trigger (the action), reset (back to base). */
export const SCENARIOS = {
  drawer_close: { trigger: async (c) => c.clickElement(byLabel('Main drawer')), reset: async (c) => c.clickElement(byLabel('Main drawer')) },
  drawer_open: { prepare: async (c) => { await c.clickElement(byLabel('Main drawer')); await c.sleep(800); }, trigger: async (c) => c.clickElement(byLabel('Main drawer')) },
  nav_next: { trigger: async (c) => c.clickElement(byLabelPrefix('Next ')), reset: async (c) => c.clickElement(byLabelPrefix('Previous ')) },
  nav_prev: { trigger: async (c) => c.clickElement(byLabelPrefix('Previous ')), reset: async (c) => c.clickElement(byLabelPrefix('Next ')) },
  today: { prepare: async (c) => { await c.clickElement(byLabelPrefix('Next ')); await c.sleep(800); await c.clickElement(byLabelPrefix('Next ')); await c.sleep(800); }, trigger: async (c) => c.clickElement(headerButton('/^Today$/')) },
  view_menu_open: { trigger: async (c) => c.clickElement(headerButton('/^(Day|Week|Month|Schedule|Year|4 days)$/')), reset: async (c) => c.pressKey('Escape') },
  view_menu_close: { prepare: async (c) => { await c.clickElement(headerButton('/^(Day|Week|Month|Schedule|Year|4 days)$/')); await c.sleep(800); }, trigger: async (c) => c.pressKey('Escape') },
  view_to_month: { prepare: async (c) => { await c.clickElement(headerButton('/^Week$/')); await c.sleep(800); }, trigger: async (c) => c.clickElement(menuItem('/^Month/')), reset: async (c) => { await c.clickElement(headerButton('/^Month$/')); await c.sleep(800); await c.clickElement(menuItem('/^Week/')); } },
  view_to_day: { prepare: async (c) => { await c.clickElement(headerButton('/^Week$/')); await c.sleep(800); }, trigger: async (c) => c.clickElement(menuItem('/^Day/')), reset: async (c) => { await c.clickElement(headerButton('/^Day$/')); await c.sleep(800); await c.clickElement(menuItem('/^Week/')); } },
  view_to_schedule: { prepare: async (c) => { await c.clickElement(headerButton('/^Week$/')); await c.sleep(800); }, trigger: async (c) => c.clickElement(menuItem('/^Schedule/')), reset: async (c) => { await c.clickElement(headerButton('/^Schedule$/')); await c.sleep(800); await c.clickElement(menuItem('/^Week/')); } },
  view_to_week: { prepare: async (c) => { await c.clickElement(headerButton('/^Week$/')); await c.sleep(800); await c.clickElement(menuItem('/^Month/')); await c.sleep(1500); await c.clickElement(headerButton('/^Month$/')); await c.sleep(800); }, trigger: async (c) => c.clickElement(menuItem('/^Week/')) },
  nav_next_month: { prepare: async (c) => { await c.clickElement(headerButton('/^Week$/')); await c.sleep(800); await c.clickElement(menuItem('/^Month/')); await c.sleep(1500); }, trigger: async (c) => c.clickElement(byLabelPrefix('Next ')), reset: async (c) => { await c.clickElement(byLabelPrefix('Previous ')); await c.sleep(800); await c.clickElement(headerButton('/^Month$/')); await c.sleep(800); await c.clickElement(menuItem('/^Week/')); } },
  event_popup_open: { trigger: async (c) => c.clickElement(firstChip), reset: async (c) => c.pressKey('Escape') },
  // Escape does not reach the popup in headless Chrome; the Close button is what the user clicks anyway.
  event_popup_close: { prepare: async (c) => { await c.clickElement(firstChip); await c.sleep(1200); }, trigger: async (c) => c.clickElement(dialogClose) },
  quick_create_open: { trigger: async (c) => { const p = await c.eval(emptySlot); await c.clickAt(p.x, p.y); }, reset: async (c) => c.pressKey('Escape') },
  quick_create_close: { prepare: async (c) => { const p = await c.eval(emptySlot); await c.clickAt(p.x, p.y); await c.sleep(1200); }, trigger: async (c) => c.clickElement(dialogClose) },
  full_form_open: {
    prepare: async (c) => { const p = await c.eval(emptySlot); await c.clickAt(p.x, p.y); await c.sleep(1200); },
    trigger: async (c) => c.clickElement(`[...document.querySelectorAll('[role=dialog] button')].find(b => /More options/.test(b.innerText||''))`),
    settle: 3000,
    reset: async (c) => { await closeFullForm(c); },
    navigates: true,
  },
  full_form_close: {
    prepare: async (c) => { const p = await c.eval(emptySlot); await c.clickAt(p.x, p.y); await c.sleep(1200); await c.clickElement(`[...document.querySelectorAll('[role=dialog] button')].find(b => /More options/.test(b.innerText||''))`); await c.sleep(3500); },
    trigger: async (c) => closeFullForm(c),
    settle: 3000,
    navigates: true,
  },
  create_menu_open: { trigger: async (c) => c.clickElement(createButton), reset: async (c) => c.pressKey('Escape') },
  create_menu_close: { prepare: async (c) => { await c.clickElement(createButton); await c.sleep(800); }, trigger: async (c) => c.pressKey('Escape') },
  settings_menu_open: { trigger: async (c) => c.clickElement(byLabel('Settings menu')), reset: async (c) => c.pressKey('Escape') },
  settings_menu_close: { prepare: async (c) => { await c.clickElement(byLabel('Settings menu')); await c.sleep(800); }, trigger: async (c) => c.pressKey('Escape') },
  settings_page_open: { prepare: async (c) => { await c.clickElement(byLabel('Settings menu')); await c.sleep(800); }, trigger: async (c) => c.clickElement(menuItem('/^Settings$/')), settle: 3000, navigates: true },
  minical_next: { trigger: async (c) => c.clickElement(byLabel('Next month')), reset: async (c) => c.clickElement(byLabel('Previous month')) },
  minical_prev: { trigger: async (c) => c.clickElement(byLabel('Previous month')), reset: async (c) => c.clickElement(byLabel('Next month')) },
  minical_pick_day: { trigger: async (c) => c.clickElement(`${minicalDays}[24]`), reset: async (c) => c.clickElement(headerButton('/^Today$/')) },
  calendar_toggle_off: { trigger: async (c) => c.clickElement(firstCalendarCheckbox), reset: async (c) => c.clickElement(firstCalendarCheckbox) },
  calendar_toggle_on: { prepare: async (c) => { await c.clickElement(firstCalendarCheckbox); await c.sleep(1500); }, trigger: async (c) => c.clickElement(firstCalendarCheckbox) },
  calendar_options_open: {
    prepare: async (c) => { await hoverElement(c, firstCalendarRow); await c.sleep(500); },
    trigger: async (c) => c.clickElement(byLabelPrefix('Options for ')),
    reset: async (c) => c.pressKey('Escape'),
  },
  // Hover and press states: transitions plus the computed-style diff at rest and hovered.
  hover_today: { hover: headerButton('/^Today$/') },
  hover_nav_next: { hover: byLabelPrefix('Next ') },
  hover_settings: { hover: byLabel('Settings menu') },
  hover_view_button: { hover: headerButton('/^(Day|Week|Month|Schedule|Year|4 days)$/') },
  hover_drawer_button: { hover: byLabel('Main drawer') },
  hover_create: { hover: createButton },
  hover_chip: { hover: firstChip },
  hover_calendar_row: { hover: firstCalendarRow },
  hover_minical_day: { hover: minicalDay },
  hover_minical_next: { hover: byLabel('Next month') },
  press_today: { hover: headerButton('/^Today$/'), press: true },
  // The press toggles the calendar; the reset clicks it back on.
  press_calendar_row: { hover: firstCalendarRow, press: true, reset: async (c) => c.clickElement(firstCalendarCheckbox) },
  press_create: { hover: createButton, press: true, reset: async (c) => c.pressKey('Escape') },
  press_chip: { hover: firstChip, press: true, reset: async (c) => c.pressKey('Escape') },
  hover_list_header: { hover: `[...document.querySelectorAll('[role=complementary] [role=button], [role=complementary] button')].find(b => /^My calendars/.test((b.innerText||'').trim()) && b.getBoundingClientRect().width > 0)` },
  hover_list_add: { hover: byLabel('Add other calendars') },
  hover_menu_item: { prepare: async (c) => { await c.clickElement(headerButton('/^(Day|Week|Month|Schedule|Year|4 days)$/')); await c.sleep(800); }, hover: menuItem('/^Month/'), reset: async (c) => c.pressKey('Escape') },
  hover_chip_selected: { prepare: async (c) => { await c.clickElement(firstChip); await c.sleep(1200); }, hover: firstChip, reset: async (c) => c.clickElement(dialogClose) },
  tooltip_nav_next: { hover: byLabelPrefix('Next '), hoverMs: 2500 },
  tooltip_today: { hover: headerButton('/^Today$/'), hoverMs: 2500 },
};

async function closeFullForm(c) {
  const close = `[...document.querySelectorAll('button, [role=button]')].find(b => /^(Close|Cancel event creation|Cancel)$/.test((b.getAttribute('aria-label')||'').trim()) && b.getBoundingClientRect().width > 0)`;
  await c.clickElement(close);
  await c.sleep(1000);
  // A discard prompt appears when the form has content; the empty form closes without one.
  const discard = `[...document.querySelectorAll('[role=dialog] button, [role=alertdialog] button')].find(b => /Discard/.test(b.innerText||'') && b.getBoundingClientRect().width > 0)`;
  if (await c.eval(`!!(${discard})`)) { await c.clickElement(discard); await c.sleep(1000); }
}

async function hoverElement(c, js) {
  const box = await c.eval(`(() => { const el = ${js}; if (!el) return null; const r = el.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`);
  if (!box) throw new Error(`element not found: ${js}`);
  await c.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: box.x, y: box.y });
  return box;
}

const parkMouse = (c) => c.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 1430, y: 890 });

async function runScenario(c, name, s) {
  await parkMouse(c);
  await c.sleep(300);
  if (s.prepare) await s.prepare(c);
  await c.eval(recorder);
  const result = { name };
  if (s.hover) {
    const rest = await c.eval(`(() => { const el = ${s.hover}; return el ? window.__animRec.styles(el) : null; })()`);
    await c.eval(`window.__animRec.start(${JSON.stringify(WATCH)})`);
    const box = await hoverElement(c, s.hover);
    await c.sleep(s.hoverMs || 700);
    const hovered = await c.eval(`(() => { const el = ${s.hover}; return el ? window.__animRec.styles(el) : null; })()`);
    result.styleDiff = diffStyles(rest, hovered);
    result.tooltip = await c.eval(`(() => { const t = [...document.querySelectorAll('[role=tooltip]')].find(t => t.getBoundingClientRect().width > 0); if (!t) return null; const cs = getComputedStyle(t); return { cls: t.className, text: (t.innerText||'').trim(), rect: [t.getBoundingClientRect().x, t.getBoundingClientRect().y, t.getBoundingClientRect().width, t.getBoundingClientRect().height], opacity: cs.opacity, transform: cs.transform, transition: cs.transition, animation: cs.animation }; })()`);
    if (s.press) {
      await c.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: box.x, y: box.y, button: 'left', clickCount: 1 });
      await c.sleep(400);
      const pressed = await c.eval(`(() => { const el = ${s.hover}; return el ? window.__animRec.styles(el) : null; })()`);
      result.pressDiff = diffStyles(hovered, pressed);
      await c.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: box.x, y: box.y, button: 'left', clickCount: 1 });
      await c.sleep(600);
    }
    Object.assign(result, await c.eval(`window.__animRec.stop()`));
    await parkMouse(c);
    await c.sleep(500);
  } else {
    await c.eval(`window.__animRec.start(${JSON.stringify(WATCH)})`);
    await s.trigger(c);
    await c.sleep(s.settle || RECORD_MS);
    Object.assign(result, await c.eval(`window.__animRec.stop()`));
  }
  if (s.reset) {
    try { await s.reset(c); } catch (e) { result.resetError = e.message; }
    await c.sleep(1000);
  }
  return result;
}

/** Entries whose hover-sensitive properties changed between two `styles()` snapshots. */
function diffStyles(a, b) {
  if (!a || !b) return null;
  const out = {};
  for (const key of Object.keys(b)) {
    const before = a[key];
    const after = b[key];
    if (!before) continue;
    const changed = {};
    for (const p of Object.keys(after)) {
      if (p === 'rect' || p === 'tag') continue;
      if (JSON.stringify(before[p]) !== JSON.stringify(after[p])) changed[p] = { from: before[p], to: after[p] };
    }
    const transition = after['transition-property'] !== 'all' || after['transition-duration'] !== '0s' ? { property: after['transition-property'], duration: after['transition-duration'], timing: after['transition-timing-function'], delay: after['transition-delay'] } : null;
    if (Object.keys(changed).length || (transition && transition.duration !== '0s')) out[key] = { cls: after.cls, rect: after.rect, changed, transition };
  }
  return out;
}

export async function measureAll(names, themes) {
  const chrome = await launchChrome();
  const report = [];
  try {
    const c = await Cdp.connect();
    await c.send('Browser.setDownloadBehavior', { behavior: 'deny' }).catch(() => {});
    await c.setViewport(1440, 900);
    const version = (await c.send('Browser.getVersion')).product;
    for (const theme of themes) {
      await c.emulateDark(theme === 'dark');
      const results = {};
      let fresh = false;
      for (const name of names) {
        const s = SCENARIOS[name];
        if (!s) { report.push(`${name}: unknown scenario`); continue; }
        // Motion timings are theme independent; only the styled states are diffed twice.
        if (theme === 'dark' && !s.hover) continue;
        if (!fresh) { await c.navigate(WEEK, { waitMs: 8000 }); fresh = true; }
        try {
          results[name] = await runScenario(c, name, s);
          const anims = results[name].found || [];
          report.push(`${name}-${theme}: ${anims.length} animations, ${(results[name].frames || []).length} frames with changes${results[name].styleDiff ? `, ${Object.keys(results[name].styleDiff).length} styled nodes` : ''}`);
        } catch (e) {
          report.push(`${name}-${theme}: FAILED ${e.message}`);
          fresh = false;
        }
        if (s.navigates) fresh = false;
      }
      if (Object.keys(results).length) {
        const file = `${OUT}animations-${theme}.json`;
        let previous = {};
        try { previous = JSON.parse(readFileSync(file, 'utf8')).scenarios || {}; } catch { /* first run */ }
        const out = { meta: { measured_at: new Date().toISOString().slice(0, 10), chrome: version, viewport: [1440, 900], url: WEEK, theme, record_ms: RECORD_MS, source: 'scripts/measure/animations.mjs' }, scenarios: { ...previous, ...results } };
        writeFileSync(file, JSON.stringify(out, null, 1));
      }
    }
    // Safety: every checkbox the scenarios touched must be back on.
    const off = await c.eval(`[...document.querySelectorAll('[role=list][aria-label="My calendars"] [role=checkbox],[role=list][aria-label="My calendars"] input[type=checkbox]')].filter(x => !(x.getAttribute('aria-checked') === 'true' || x.checked)).map(x => x.getAttribute('aria-label'))`);
    if (off.length) report.push(`WARNING: calendars left unchecked on Google: ${off.join(', ')} — re-check them by hand`);
    c.close();
  } finally {
    chrome.kill();
  }
  return report;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const names = process.argv.slice(2).length ? process.argv.slice(2) : Object.keys(SCENARIOS);
  const report = await measureAll(names, ['light', 'dark']);
  console.log(report.join('\n'));
}
