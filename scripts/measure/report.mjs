#!/usr/bin/env node
// Summarise every Google/app pair in docs/design/measurements: diff-layout differences and the
// odiff pixel percentage, light and dark. Usage: node scripts/measure/report.mjs [--md]
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
const names = readdirSync(MEAS).filter((f) => /-(light|dark)-app\.json$/.test(f)).map((f) => f.replace(/-(light|dark)-app\.json$/, '')).filter((v, i, a) => a.indexOf(v) === i).sort();
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
    if (existsSync(`${MEAS}${name}-${theme}.png`) && existsSync(`${MEAS}${name}-${theme}-app.png`)) {
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
  for (const r of rows) console.log(`${r.name.padEnd(28)} ${r.theme.padEnd(5)} nodes=${r.compared} diffs=${r.diffs} px=${r.px}%`);
}
void execFileSync;
