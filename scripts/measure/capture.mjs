#!/usr/bin/env node
// Measure Google Calendar components with the signed-in ~/.chrome-measure profile
// (docs/04 section 3, automated). For every component and theme it saves
// docs/design/measurements/<name>-<light|dark>.json (dumpRegion output) and .png (node shot).
// Usage: node scripts/measure/capture.mjs [component[:state] ...]   (default: all)
import { mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { Cdp, dumpRegionSource, launchChrome } from './cdp.mjs';

const OUT = fileURLToPath(new URL('../../docs/design/measurements/', import.meta.url));
const WEEK = 'https://calendar.google.com/calendar/u/0/r/week/2026/9/14';
mkdirSync(OUT, { recursive: true });

// Timed chips read "HH:MM to HH:MM, <title>, Calendar: …"; all-day chips start with the title.
const chipJs = (title) => `(() => { const t = ${JSON.stringify(title)}; return [...document.querySelectorAll('[role=main] [role=button]')].find(b => { const s = (b.innerText||'').replace(/\\n/g, ' '); return /^\\d\\d:\\d\\d to \\d\\d:\\d\\d, /.test(s) && s.slice(s.indexOf(', ') + 2).startsWith(t + ','); }); })()`;
const allDayChipJs = (title) => `[...document.querySelectorAll('[role=main] [role=button]')].find(b => (b.innerText||'').startsWith(${JSON.stringify(title)}))`;

/** Component registry. `root` is a JS expression returning the root element. */
export const COMPONENTS = {
  topbar: { url: WEEK, root: `document.querySelector('header[role=banner]')` },
  // The whole drawer (Create button, mini calendar, people search, calendar lists): the only dump
  // that records the offsets between the sidebar components and the drawer's own box.
  sidebar: {
    url: WEEK,
    root: `(() => { const l = [...document.querySelectorAll('[role=complementary]')].find(c => /My calendars/.test(c.innerText||'')); let e = l; while (e.parentElement && e.parentElement.getBoundingClientRect().width < 400) e = e.parentElement; return e; })()`,
  },
  create_button: {
    url: WEEK,
    root: `[...document.querySelectorAll('button')].find(b => /Create/.test(b.innerText||'') && b.getBoundingClientRect().x < 200)`,
    states: {
      open: { prepare: async (cdp) => { await cdp.clickElement(`[...document.querySelectorAll('button')].find(b => /Create/.test(b.innerText||'') && b.getBoundingClientRect().x < 200)`); await cdp.sleep(1200); }, root: `[...document.querySelectorAll('[role=menu]')].find(m => m.getBoundingClientRect().width > 0).parentElement.parentElement` },
    },
  },
  mini_calendar: {
    url: WEEK,
    root: `document.querySelector('table[role=grid]').closest('[role=grid]').parentElement.parentElement`,
    states: {
      other_month: { prepare: async (cdp) => { await cdp.clickElement(`document.querySelector('button[aria-label="Next month"]')`); await cdp.sleep(1200); } },
      // Another week displayed: how the selected (non-today) day and the visible week are marked.
      next_week: { url: 'https://calendar.google.com/calendar/u/0/r/week/2026/9/23' },
    },
  },
  calendar_list: {
    url: WEEK,
    root: `[...document.querySelectorAll('[role=complementary]')].find(c => /My calendars/.test(c.innerText||''))`,
    states: {
      hover: { prepare: async (cdp) => { const b = await cdp.eval(`(() => { const r = [...document.querySelectorAll('[role=list][aria-label="My calendars"] [role=presentation]')][0].getBoundingClientRect(); return { x: r.x + r.width/2, y: r.y + r.height/2 }; })()`); await cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: b.x, y: b.y }); await cdp.sleep(800); } },
    },
  },
  week_header: { url: WEEK, root: `document.querySelector('[role=main] [role=columnheader]').closest('[role=row]')` },
  allday_row: { url: WEEK, root: `[...document.querySelectorAll('[role=main] [role=row]')].find(r => /All-day one/.test(r.innerText||''))` },
  hour_grid: { url: WEEK, grid: true, root: `document.querySelector('[role=main] [role=grid]')` },
  now_line: {
    url: WEEK,
    grid: true,
    // The line is a 2px top border; the dot is its 12px rounded sibling.
    root: `(() => { const g = document.querySelector('[role=main] [role=grid]'); return [...g.querySelectorAll('div')].find(e => { const cs = getComputedStyle(e); const r = e.getBoundingClientRect(); return cs.borderTopWidth === '2px' && cs.borderTopStyle === 'solid' && r.height <= 3 && r.width > 100 && e.nextElementSibling && getComputedStyle(e.nextElementSibling).borderRadius === '9999px'; }); })()`,
    states: {
      dot: { root: `(() => { const g = document.querySelector('[role=main] [role=grid]'); return [...g.querySelectorAll('div')].find(e => { const cs = getComputedStyle(e); const r = e.getBoundingClientRect(); return cs.borderRadius === '9999px' && Math.round(r.width) === 12 && Math.round(r.height) === 12; }); })()` },
    },
  },
  event_chip: {
    url: WEEK,
    grid: true,
    root: chipJs('Weekend'),
    states: {
      sixty: { root: chipJs('Sixty') },
      fifteen: { root: chipJs('Fifteen') },
      thirty: { root: chipJs('Thirty') },
      forty_five: { root: chipJs('Forty-five') },
      ninety: { root: chipJs('Ninety') },
      overlap_a: { root: chipJs('Overlap A') },
      overlap_b: { root: chipJs('Overlap B') },
      triple_a: { root: chipJs('Triple A') },
      triple_b: { root: chipJs('Triple B') },
      triple_c: { root: chipJs('Triple C') },
      all_day: { root: allDayChipJs('All-day one') },
      // Three lines: title (no wrap), time, location.
      with_location: { root: chipJs('With Meet') },
      hover: { prepare: async (cdp) => { const b = await cdp.eval(`(() => { const r = ${chipJs('Weekend')}.getBoundingClientRect(); return { x: r.x + r.width/2, y: r.y + r.height/2 }; })()`); await cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: b.x, y: b.y }); await cdp.sleep(800); }, root: chipJs('Weekend') },
      // "Reduce the brightness of past events" is off in the reference settings, so a past chip
      // (Sixty, Monday 13:00) renders like a future one; measured to record that fact.
      past: { root: chipJs('Sixty') },
    },
  },
  day_view: { url: 'https://calendar.google.com/calendar/u/0/r/day/2026/9/14', root: `document.querySelector('[role=main]')` },
  month_view: { url: 'https://calendar.google.com/calendar/u/0/r/month/2026/9/14', root: `document.querySelector('[role=main]')` },
  agenda_view: { url: 'https://calendar.google.com/calendar/u/0/r/agenda/2026/9/14', root: `document.querySelector('[role=main]')` },
  view_selector: {
    url: WEEK,
    prepare: async (cdp) => { await cdp.clickElement(`[...document.querySelectorAll('header [role=button]')].find(b => /^Week/.test((b.innerText||'').trim()))`); await cdp.sleep(1200); },
    root: `[...document.querySelectorAll('[role=menu]')].find(m => m.getBoundingClientRect().width > 0)`,
  },
};

// Tentative/declined chips live on the primary calendar (invitation copies). Measured with it shown.
COMPONENTS.event_chip.states.tentative = {
  prepare: async (cdp) => { await toggleCalendar(cdp, 'Shlomo Serber', true); },
  // The attendee's copy on the primary calendar, not the organiser's copy on UGC Fixtures.
  root: `[...document.querySelectorAll('[role=main] [role=button]')].find(b => { const s = (b.innerText||'').replace(/\\n/g,' '); return /, Tentative, Shlomo Serber, Tentative, /.test(s); })`,
  cleanup: async (cdp) => { await toggleCalendar(cdp, 'Shlomo Serber', false); },
};
COMPONENTS.event_chip.states.declined = {
  prepare: async (cdp) => { await toggleCalendar(cdp, 'Shlomo Serber', true); },
  root: `[...document.querySelectorAll('[role=main] [role=button]')].find(b => { const s = (b.innerText||'').replace(/\\n/g,' '); return /, Declined, Shlomo Serber, Declined, /.test(s); })`,
  cleanup: async (cdp) => { await toggleCalendar(cdp, 'Shlomo Serber', false); },
};

async function toggleCalendar(cdp, label, on) {
  const sel = `[...document.querySelectorAll('[role=checkbox],input[type=checkbox]')].find(c => (c.getAttribute('aria-label')||'') === ${JSON.stringify(label)})`;
  const checked = await cdp.eval(`(() => { const c = ${sel}; return c ? (c.getAttribute('aria-checked') === 'true' || c.checked === true) : null; })()`);
  if (checked !== null && checked !== on) { await cdp.clickElement(sel); await cdp.sleep(1500); }
}

// Google's initial scroll depends on the clock (tokens.json layout.week_initial_scroll_note); the
// grid components are dumped at the 07:00 position so every run and the app compare alike.
const GRID_SCROLL = 420;
async function scrollGrid(cdp) {
  await cdp.eval(`(() => { const g = document.querySelector('[role=main] [role=grid]'); const row = [...g.querySelectorAll('[role=row]')].find(r => r.getBoundingClientRect().height > 1000); row.parentElement.scrollTop = ${GRID_SCROLL}; })()`);
  await cdp.sleep(600);
}

async function measure(cdp, name, root) {
  await cdp.eval(dumpRegionSource());
  const json = await cdp.eval(`(() => { const el = ${root}; if (!el) return null; return window.dumpRegion(el, { keepDefaults: false }); })()`);
  if (!json) throw new Error(`${name}: root not found`);
  const r = json.rootRect;
  const png = await cdp.screenshot({ x: Math.max(0, Math.floor(r[0])), y: Math.max(0, Math.floor(r[1])), width: Math.max(1, Math.ceil(r[2])), height: Math.max(1, Math.ceil(r[3])) });
  return { json, png };
}

export async function captureAll(names, themes = ['light', 'dark']) {
  const chrome = await launchChrome();
  const report = [];
  try {
    const cdp = await Cdp.connect();
    await cdp.send('Browser.setDownloadBehavior', { behavior: 'deny' }).catch(() => {});
    await cdp.setViewport(1440, 900);
    for (const theme of themes) {
      await cdp.emulateDark(theme === 'dark');
      let current = null;
      for (const spec of names) {
        // "component" runs every state; "component:state" (or "component:base") runs one.
        const [name, only] = spec.split(':');
        const comp = COMPONENTS[name];
        if (!comp) { report.push(`${name}: unknown component`); continue; }
        let variants = [{ key: '', ...comp }, ...Object.entries(comp.states || {}).map(([k, v]) => ({ key: k, ...v, url: v.url || comp.url }))];
        if (only) variants = variants.filter((v) => (only === 'base' ? v.key === '' : v.key === only));
        for (const v of variants) {
          if (current !== v.url) { await cdp.navigate(v.url, { waitMs: 8000 }); current = v.url; }
          else { await cdp.send('Page.reload'); await cdp.sleep(7000); }
          // Park the mouse so no hover state leaks in.
          await cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 1430, y: 890 });
          await cdp.sleep(300);
          try {
            if (comp.grid) await scrollGrid(cdp);
            if (v.prepare) await v.prepare(cdp);
            const root = v.root || comp.root;
            const { json, png } = await measure(cdp, name, root);
            const file = v.key ? `${name}-${v.key}-${theme}` : `${name}-${theme}`;
            writeFileSync(`${OUT}${file}.json`, JSON.stringify(json, null, 1));
            writeFileSync(`${OUT}${file}.png`, png);
            report.push(`${file}: ${json.nodes.length} nodes, rect ${json.rootRect.map(Math.round).join(',')}`);
            if (v.cleanup) await v.cleanup(cdp);
          } catch (e) {
            report.push(`${name}${v.key ? '-' + v.key : ''}-${theme}: FAILED ${e.message}`);
            try { if (v.cleanup) await v.cleanup(cdp); } catch { /* best effort */ }
          }
        }
      }
    }
    cdp.close();
  } finally {
    chrome.kill();
  }
  return report;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const args = process.argv.slice(2);
  const names = args.length ? args : Object.keys(COMPONENTS);
  const report = await captureAll(names);
  for (const line of report) console.log(line);
}
