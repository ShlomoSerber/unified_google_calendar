#!/usr/bin/env node
// Measure the modern event/calendar palettes of calendar.google.com (docs/04 section 7) in light
// and dark: the chip background and text colours for the 11 event colours ("Color N" fixtures,
// two weeks after the reference week) and for the 24 calendarList colours (the fixture
// calendar's colorId is cycled through 1..24 with the seed tool and restored at the end).
// Output: docs/design/measurements/palette.json. Usage: node scripts/measure/palette.mjs <account_email>
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { Cdp, launchChrome } from './cdp.mjs';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const email = process.argv[2];
if (!email) { console.error('usage: palette.mjs <account_email>'); process.exit(2); }
const COLOR_WEEK = 'https://calendar.google.com/calendar/u/0/r/week/2026/9/28';
const chipJs = (title) => `(() => { const t = ${JSON.stringify(title)}; return [...document.querySelectorAll('[role=main] [role=button]')].find(b => { const s = (b.innerText||'').replace(/\\n/g, ' '); return /^\\d\\d:\\d\\d to \\d\\d:\\d\\d, /.test(s) && s.slice(s.indexOf(', ') + 2).startsWith(t + ','); }); })()`;
const readJs = (title) => `(() => { const el = ${chipJs(title)}; if (!el) return null; const cs = getComputedStyle(el); const title = [...el.querySelectorAll('span')].find(s => (s.innerText||'').trim() === ${JSON.stringify(title)}); return { bg: cs.backgroundColor, fg: title ? getComputedStyle(title).color : null }; })()`;

function seed(...args) {
  const out = execFileSync('cargo', ['run', '-q', '--manifest-path', `${ROOT}src-tauri/Cargo.toml`, '-j', '2', '--example', 'seed_fixtures', '--', email, ...args], { env: { ...process.env, PATH: `${process.env.HOME}/.cargo/bin:${process.env.PATH}` } }).toString();
  return out.trim().split('\n').pop();
}

const chrome = await launchChrome();
const result = { measured_at: new Date().toISOString().slice(0, 10), source: 'calendar.google.com, Modern colour set (docs/04 section 7)', event: {}, calendar: {} };
try {
  const cdp = await Cdp.connect();
  await cdp.setViewport(1440, 900);
  for (const theme of ['light', 'dark']) {
    await cdp.emulateDark(theme === 'dark');
    await cdp.navigate(COLOR_WEEK, { waitMs: 8000 });
    for (let id = 1; id <= 11; id++) {
      const r = await cdp.eval(readJs(`Color ${id}`));
      result.event[id] ??= {};
      result.event[id][theme] = r;
      console.log(`event ${id} ${theme}: ${JSON.stringify(r)}`);
    }
  }
  const original = seed('--calendar-color', 'get').match(/colorId=(\d+)/)[1];
  console.log(`fixture calendar colorId ${original}`);
  try {
    for (let id = 1; id <= 24; id++) {
      const line = seed('--calendar-color', String(id));
      const classic = line.match(/backgroundColor=(#[0-9a-f]+)/i)?.[1];
      result.calendar[id] = { classic };
      for (const theme of ['light', 'dark']) {
        await cdp.emulateDark(theme === 'dark');
        await cdp.navigate(COLOR_WEEK, { waitMs: 7000 });
        // "Weekly repeat" recurs into this week and carries no colorId: it shows the calendar colour.
        const r = await cdp.eval(readJs('Weekly repeat'));
        result.calendar[id][theme] = r;
        console.log(`calendar ${id} (${classic}) ${theme}: ${JSON.stringify(r)}`);
      }
    }
  } finally {
    console.log(`restored: ${seed('--calendar-color', original)}`);
  }
  cdp.close();
} finally {
  chrome.kill();
}
writeFileSync(`${ROOT}docs/design/measurements/palette.json`, JSON.stringify(result, null, 1));
console.log('wrote docs/design/measurements/palette.json');
