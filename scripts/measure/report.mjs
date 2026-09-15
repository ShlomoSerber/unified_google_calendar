#!/usr/bin/env node
// Summarise every Google/app pair in docs/design/measurements: diff-layout differences and the
// odiff pixel percentage, light and dark. Usage: node scripts/measure/report.mjs [--md] [--gate <phase>]
// With --gate the exit code is 1 when a component of that phase has more layout differences than
// ALLOWED (each allowance is justified in docs/design/measurements/<component>.md).
import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const MEAS = fileURLToPath(new URL('../../docs/design/measurements/', import.meta.url));
const DIFF = fileURLToPath(new URL('./diff-layout.mjs', import.meta.url));
// Per-component options, justified in docs/design/measurements/<component>.md.
const OPTIONS = {
  topbar: ['--ignore', 'Google Account', '--ignore-hidden'],
  calendar_list: ['--ignore', 'My calendars|Shlomo Serber|Birthdays|RappiCard|Tasks|Other calendars|Holidays in Argentina|complementary', '--ignore-hidden'],
  sidebar: ['--ignore', 'My calendars|Shlomo Serber|Birthdays|RappiCard|Tasks|Other calendars|Holidays in Argentina|complementary', '--ignore-hidden'],
};
// Justified residual differences (see the .md of each component); everything else must be 0.
const ALLOWED = {
  'calendar_list': 3, 'calendar_list-hover': 4, 'sidebar': 3, // sections and rows per account (docs/99 F4-T3)
  'hour_grid': 10, // 2 text widths at ±1 px + the deduplicated "Declined" chip (docs/99 F4-T4)
  'event_chip-fifteen': 3, 'event_chip-ninety': 1, // 11-12 px text widths (font metrics)
  'event_chip-tentative': 3, 'event_chip-declined': 4, // deduplicated copy: width and calendar colour
  'day_view': 4, 'month_view': 3, 'agenda_view': 13, // text widths of titles and date labels (see .md)
  'event_popup': 6, 'event_popup-meet': 12, 'event_popup-guests': 12, 'event_popup-recurring': 6, // text widths, creator/guest names the app cannot resolve (see .md)
  'quick_create': 13, 'quick_create-with_title': 13, // text widths and the default calendar (see .md)
  'full_form': 14, 'recurrence_dialog': 12, 'edit_scope_dialog': 5, // text widths; the third scope option (see .md)
};
const PHASE = {
  4: ['topbar', 'sidebar', 'create_button', 'mini_calendar', 'calendar_list', 'week_header', 'allday_row', 'hour_grid', 'now_line', 'event_chip'],
  7: ['day_view', 'month_view', 'agenda_view', 'event_popup', 'quick_create', 'full_form', 'view_selector', 'recurrence_dialog', 'edit_scope_dialog'],
};
const gateIdx = process.argv.indexOf('--gate');
const gate = gateIdx >= 0 ? PHASE[process.argv[gateIdx + 1]] : null;
const names = readdirSync(MEAS).filter((f) => /-(light|dark)-app\.json$/.test(f)).map((f) => f.replace(/-(light|dark)-app\.json$/, '')).filter((v, i, a) => a.indexOf(v) === i).filter((n) => !gate || gate.includes(n.split('-')[0])).sort();
const md = process.argv.includes('--md');
const rows = [];
for (const name of names) {
  const comp = name.split('-')[0];
  for (const theme of ['light', 'dark']) {
    const ref = `${MEAS}${name}-${theme}.json`, app = `${MEAS}${name}-${theme}-app.json`;
    if (!existsSync(ref) || !existsSync(app)) continue;
    const r = spawnSync('node', [DIFF, ref, app, '--tolerance', '1', ...(OPTIONS[comp] || ['--ignore-hidden'])], { encoding: 'utf8' });
    const m = r.stdout.match(/(\d+) differences/);
    const compared = r.stdout.match(/compared (\d+) nodes/)?.[1] ?? '?';
    let px = 'n/a';
    if (!gate && existsSync(`${MEAS}${name}-${theme}.png`) && existsSync(`${MEAS}${name}-${theme}-app.png`)) {
      const o = spawnSync('npx', ['--yes', 'odiff-bin', '--antialiasing', '--threshold', '0.1', `${MEAS}${name}-${theme}.png`, `${MEAS}${name}-${theme}-app.png`, `/tmp/claude-1000/odiff-${name}-${theme}.png`], { encoding: 'utf8' });
      const out = `${o.stdout}\n${o.stderr}`;
      px = out.match(/\(([\d.]+)%\)/)?.[1] ?? (o.status === 0 ? '0' : out.trim().split('\n').pop());
    }
    rows.push({ name, theme, compared, diffs: m ? m[1] : 'ERR', px });
  }
}
if (md) {
  console.log('| Componente | Tema | Nodos comparados | Diferencias de layout | odiff |');
  console.log('|---|---|---|---|---|');
  for (const r of rows) console.log(`| ${r.name} | ${r.theme} | ${r.compared} | ${r.diffs} | ${r.px}% |`);
} else {
  for (const r of rows) console.log(`${r.name.padEnd(28)} ${r.theme.padEnd(5)} nodes=${r.compared} diffs=${r.diffs}${gate ? ` (allowed ${ALLOWED[r.name] ?? 0})` : ` px=${r.px}%`}`);
}
if (gate) {
  const missing = gate.filter((c) => !rows.some((r) => r.name === c));
  const bad = rows.filter((r) => r.diffs === 'ERR' || Number(r.diffs) > (ALLOWED[r.name] ?? 0));
  for (const m of missing) console.log(`MISSING dumps for ${m}`);
  for (const b of bad) console.log(`FAIL ${b.name} ${b.theme}: ${b.diffs} differences (allowed ${ALLOWED[b.name] ?? 0})`);
  process.exit(missing.length || bad.length ? 1 : 0);
}
void execFileSync;
