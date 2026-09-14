#!/usr/bin/env node
// App side of docs/04 section 3 (steps 8-10), automated: starts vite and the dev binary on a
// private Xvfb display, asks the webview to run dumpRegion on each component through
// POST /dev/measure (dev-only), saves docs/design/measurements/<name>-app.json and takes a
// node screenshot (<name>-app.png) from the X display. Runs light and dark (GTK_THEME).
// Usage: node scripts/measure/app-capture.mjs [component[:state]...]   (see APP_COMPONENTS)
import { spawn, execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, unlinkSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { APP_COMPONENTS, RESTORE } from './app-components.mjs';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const MEAS = `${ROOT}docs/design/measurements/`;
const DISPLAY = process.env.MEASURE_DISPLAY || ':150';
const BIN = `${ROOT}src-tauri/target/debug/unified-google-calendar`;
mkdirSync(MEAS, { recursive: true });

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitFor(fn, timeoutMs, every = 300) {
  const t0 = Date.now();
  while (Date.now() - t0 < timeoutMs) {
    try { const v = await fn(); if (v) return v; } catch { /* retry */ }
    await sleep(every);
  }
  throw new Error('timeout');
}

function startXvfb() {
  const x = spawn('Xvfb', [DISPLAY, '-screen', '0', '1440x900x24', '-nolisten', 'tcp'], { stdio: 'ignore' });
  return x;
}

function startVite() {
  return spawn('npx', ['vite', '--port', '5173', '--strictPort'], { cwd: ROOT, stdio: 'ignore' });
}

function startApp(theme) {
  const env = {
    ...process.env,
    DISPLAY,
    GDK_BACKEND: 'x11',
    GDK_SCALE: '1',
    UGC_MEASURE_DIR: MEAS.replace(/\/$/, ''),
    GTK_THEME: theme === 'dark' ? 'Adwaita:dark' : 'Adwaita',
    WEBKIT_DISABLE_DMABUF_RENDERER: '1',
  };
  return spawn(BIN, [], { env, stdio: 'ignore' });
}

async function windowGeometry() {
  const out = execFileSync('xdotool', ['search', '--sync', '--onlyvisible', '--name', 'Unified Google Calendar', 'getwindowgeometry', '--shell'], { env: { ...process.env, DISPLAY } }).toString();
  const g = Object.fromEntries(out.trim().split('\n').map((l) => l.split('=')));
  return { x: Number(g.X), y: Number(g.Y), w: Number(g.WIDTH), h: Number(g.HEIGHT) };
}

function screenshot(file) {
  execFileSync('scrot', ['-o', file], { env: { ...process.env, DISPLAY } });
}

function crop(src, dst, x, y, w, h) {
  execFileSync('python3', ['-c', `from PIL import Image; im = Image.open(${JSON.stringify(src)}); im.crop((${x}, ${y}, ${x + w}, ${y + h})).save(${JSON.stringify(dst)})`]);
}

async function measure(name, spec, theme, geom) {
  const file = `${MEAS}${name}-${theme}-app.json`;
  if (existsSync(file)) unlinkSync(file);
  // An action value may differ per theme ({ light, dark }) when the reference dumps do.
  const actions = (spec.actions || []).map((a) => Object.fromEntries(Object.entries(a).map(([k, v]) => [k, v && typeof v === 'object' && !Array.isArray(v) && 'light' in v ? v[theme] : v])));
  const res = await fetch('http://127.0.0.1:8080/dev/measure', { method: 'POST', body: JSON.stringify({ name: `${name}-${theme}`, root: spec.root, actions }) });
  if (res.status !== 202) throw new Error(`dev/measure answered ${res.status}`);
  const json = await waitFor(() => existsSync(file) && JSON.parse(readFileSync(file, 'utf8')), 20000);
  if (json.error) throw new Error(json.error);
  const [x, y, w, h] = json.rootRect;
  const shot = `/tmp/claude-1000/app-${theme}.png`;
  screenshot(shot);
  crop(shot, `${MEAS}${name}-${theme}-app.png`, Math.round(geom.x + x), Math.round(geom.y + y), Math.round(w), Math.round(h));
  // Undo the actions' UI state for the next component.
  if (spec.reset) await fetch('http://127.0.0.1:8080/dev/measure', { method: 'POST', body: JSON.stringify({ name: `_reset`, root: 'body', actions: spec.reset }) });
  await sleep(400);
  return `${name}-${theme}-app: ${json.nodes.length} nodes, rect ${json.rootRect.map(Math.round).join(',')}`;
}

const names = process.argv.slice(2).length ? process.argv.slice(2) : Object.keys(APP_COMPONENTS);
const themes = (process.env.MEASURE_THEMES || 'light,dark').split(',');
const xvfb = startXvfb();
await sleep(800);
const vite = startVite();
const report = [];
try {
  await waitFor(async () => (await fetch('http://127.0.0.1:5173/')).ok, 30000);
  for (const theme of themes) {
    const app = startApp(theme);
    try {
      await waitFor(async () => (await fetch('http://127.0.0.1:8080/healthz')).ok, 60000);
      await sleep(4000);
      const geom = await windowGeometry();
      console.error(`window geometry (${theme}): ${JSON.stringify(geom)}`);
      for (const spec of names) {
        // "component" runs every state; "component:state" (or "component:base") runs one.
        const [name, only] = spec.split(':');
        const comp = APP_COMPONENTS[name];
        if (!comp) { report.push(`${name}: unknown`); continue; }
        let variants = [{ key: '', ...comp }, ...Object.entries(comp.states || {}).map(([k, v]) => ({ key: k, root: comp.root, actions: comp.actions, ...v }))];
        if (only) variants = variants.filter((v) => (only === 'base' ? v.key === '' : v.key === only));
        for (const v of variants) {
          const full = v.key ? `${name}-${v.key}` : name;
          try { report.push(await measure(full, v, theme, geom)); } catch (e) { report.push(`${full}-${theme}-app: FAILED ${e.message}`); }
        }
      }
      await fetch('http://127.0.0.1:8080/dev/measure', { method: 'POST', body: JSON.stringify({ name: '_restore', root: 'body', actions: RESTORE }) }).catch(() => undefined);
      await sleep(1500);
    } finally {
      app.kill('SIGTERM');
      await sleep(1500);
      try { app.kill('SIGKILL'); } catch { /* gone */ }
    }
  }
} finally {
  vite.kill();
  xvfb.kill();
}
for (const l of report) console.log(l);
