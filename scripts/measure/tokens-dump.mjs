#!/usr/bin/env node
// Component 1 of docs/04 section 4: export the --gm3-* / --gm-* custom properties resolved on
// :root, plus the effective font families per text role, for light and dark.
// Writes docs/design/measurements/gm3-<theme>.json.
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { Cdp, launchChrome } from './cdp.mjs';

const OUT = fileURLToPath(new URL('../../docs/design/measurements/', import.meta.url));
const SNIPPET = `(() => {
  const names = new Set();
  for (const s of document.styleSheets) { let rules; try { rules = s.cssRules; } catch { continue; }
    for (const r of rules) for (const m of (r.cssText.match(/--gm3?-[\\w-]+/g) || [])) names.add(m); }
  const cs = getComputedStyle(document.documentElement), out = {};
  [...names].sort().forEach(n => { const v = cs.getPropertyValue(n).trim(); if (v) out[n] = v; });
  const roles = {};
  const pick = (label, el) => { if (!el) return; const c = getComputedStyle(el); roles[label] = { fontFamily: c.fontFamily, fontSize: c.fontSize, fontWeight: c.fontWeight, lineHeight: c.lineHeight, letterSpacing: c.letterSpacing, color: c.color }; };
  pick('body', document.body);
  pick('topbar_title', [...document.querySelectorAll('header [role=heading]')].find(h => /Calendar/.test(h.innerText)));
  pick('topbar_month', [...document.querySelectorAll('header *')].find(e => e.children.length === 0 && /September 2026/.test(e.innerText||'')));
  pick('topbar_button', [...document.querySelectorAll('header button')].find(b => /Today/.test(b.innerText))?.querySelector('span') || [...document.querySelectorAll('header button')].find(b => /Today/.test(b.innerText)));
  pick('sidebar_item', [...document.querySelectorAll('[role=list][aria-label="My calendars"] *')].find(e => e.children.length === 0 && /UGC Fixtures/.test(e.innerText||'')));
  pick('mini_cal_day', [...document.querySelectorAll('table[role=grid] td *')].find(e => e.children.length === 0 && (e.innerText||'').trim() === '15'));
  pick('day_header_name', [...document.querySelectorAll('[role=main] [role=columnheader] *')].find(e => e.children.length === 0 && /^MON$|^Mon$/.test(e.innerText.trim())));
  pick('day_header_number', [...document.querySelectorAll('[role=main] [role=columnheader] *')].find(e => e.children.length === 0 && (e.innerText||'').trim() === '15'));
  pick('hour_label', [...document.querySelectorAll('[role=main] *')].find(e => e.children.length === 0 && /^10:00$/.test((e.innerText||'').trim())));
  const chip = [...document.querySelectorAll('[role=main] [role=button]')].find(b => /Sixty/.test(b.innerText||''));
  if (chip) { const leaves = [...chip.querySelectorAll('*')].filter(e => e.children.length === 0 && (e.innerText||'').trim()); pick('chip_title', leaves[0]); pick('chip_time', leaves[1]); }
  const fonts = [...document.fonts].filter(f => f.status === 'loaded').map(f => f.family + ' ' + f.weight + ' ' + f.style);
  return { tokens: out, count: Object.keys(out).length, roles, fonts: [...new Set(fonts)].sort(), colorScheme: cs.colorScheme, bodyBg: getComputedStyle(document.body).backgroundColor };
})()`;

const chrome = await launchChrome();
try {
  const cdp = await Cdp.connect();
  await cdp.setViewport(1440, 900);
  for (const theme of ['light', 'dark']) {
    await cdp.emulateDark(theme === 'dark');
    await cdp.navigate('https://calendar.google.com/calendar/u/0/r/week/2026/9/14', { waitMs: 9000 });
    const data = await cdp.eval(SNIPPET);
    data.theme = theme;
    data.measured_at = new Date().toISOString();
    data.chrome = (await cdp.send('Browser.getVersion')).product;
    writeFileSync(`${OUT}gm3-${theme}.json`, JSON.stringify(data, null, 1));
    console.log(theme, data.count, 'tokens;', 'body bg', data.bodyBg, '; fonts:', data.fonts.slice(0, 12).join(' | '));
    console.log(JSON.stringify(data.roles, null, 0).slice(0, 1500));
  }
  cdp.close();
} finally {
  chrome.kill();
}
