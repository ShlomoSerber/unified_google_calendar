#!/usr/bin/env node
// Fill docs/design/tokens.json from the measurement dumps, driven by docs/design/token-spec.json.
// Nothing is typed by hand: every value is read from a dumpRegion node of calendar.google.com
// (docs/04 section 5, .claude/rules/design-tokens.md). Light and dark dumps are both read; a
// value that differs between themes becomes { light, dark } (gen-tokens emits the dark override).
//
// Spec:
// {
//   "component": {
//     "<comp>": {
//       "source": "<dump base name>",           // measurements/<base>-light.json and -dark.json
//       "names": { "<index>": "<alias>" },      // node index in the dump -> readable key (else n<index>)
//       "size": ["<key>", ...],                 // keys whose width/height are exported even if they have children
//       "skip": ["<key>", ...],                 // keys not exported
//       "only_named": true,                     // export only the nodes listed in "names" (repetitive DOM)
//       "extra": { "<key>": ["<dump>", <index>] } // one node from another dump (hover/selected states)
//     }
//   },
//   "font":   { "<role>": { "<prop>": ["<dump>", "<node index>", "<style prop>"] } },
//   "color":  { "<name>": ["<dump>", "<node index>", "<style prop>"] },
//   "layout": { "<name>": ["<dump>", "<node index>", "rect.w|rect.h|<style prop>"] },
//   "set":    { "<dotted.path>": <literal copied from a measurement file, with a note> }
// }
// Every exported node key gets one token per non-default style property of the node, named
// component.<comp>.<key>.<prop>, which scripts/gen-measured-css.mjs turns into the rule
// .<comp>-<key> { <css prop>: var(--<comp>-<key>-<prop>) }.
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const MEAS = `${ROOT}docs/design/measurements/`;
const spec = JSON.parse(readFileSync(`${ROOT}docs/design/token-spec.json`, 'utf8'));
const tokens = JSON.parse(readFileSync(`${ROOT}docs/design/tokens.json`, 'utf8'));

// Style properties exported per node (dumpRegion omits the ones equal to a plain div's).
const EXPORTED = ['display', 'position', 'fontFamily', 'fontSize', 'fontWeight', 'lineHeight', 'letterSpacing', 'color',
  'backgroundColor', 'borderTop', 'borderRight', 'borderBottom', 'borderLeft', 'borderRadius', 'paddingTop', 'paddingRight',
  'paddingBottom', 'paddingLeft', 'marginTop', 'marginRight', 'marginBottom', 'marginLeft', 'boxShadow', 'opacity', 'minWidth',
  'minHeight', 'gap', 'flexDirection', 'alignItems', 'justifyContent', 'textTransform', 'whiteSpace', 'overflow', 'zIndex', 'cursor', 'fill',
  'backgroundImage', 'textDecorationLine', 'textAlign', 'boxSizing', 'flex', 'verticalAlign', 'textOverflow', 'borderSpacing', 'borderCollapse', 'float', 'order', 'alignSelf'];

// Inherited properties are emitted for every node, default or not: a node that shows the page
// default in Google may sit under a parent that does not, and the omitted value would otherwise
// leak from our own parent (a <span> at 14px inside a 13.33px <button>).
const INHERITED = ['fontFamily', 'fontSize', 'fontWeight', 'lineHeight', 'letterSpacing', 'color', 'whiteSpace', 'textAlign', 'textTransform', 'cursor', 'fill'];
// Tags whose user-agent stylesheet differs from a <div>'s: the dump omits values equal to a div's
// default, so the UA value would apply in our DOM unless the default is emitted explicitly.
const UA_RESET = {
  ul: ['marginTop', 'marginBottom', 'paddingLeft'], ol: ['marginTop', 'marginBottom', 'paddingLeft'],
  h1: ['marginTop', 'marginBottom', 'fontSize', 'fontWeight'], h2: ['marginTop', 'marginBottom', 'fontSize', 'fontWeight'],
  h3: ['marginTop', 'marginBottom', 'fontSize', 'fontWeight'], p: ['marginTop', 'marginBottom'],
  button: ['paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft', 'borderTop', 'borderRight', 'borderBottom', 'borderLeft', 'backgroundColor', 'textAlign', 'letterSpacing'],
  input: ['paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft', 'borderTop', 'borderRight', 'borderBottom', 'borderLeft', 'backgroundColor', 'letterSpacing'],
  table: ['borderSpacing', 'borderCollapse'], th: ['fontWeight', 'textAlign', 'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft'],
  td: ['paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft'],
  fieldset: ['paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft', 'borderTop', 'borderRight', 'borderBottom', 'borderLeft', 'marginLeft', 'marginRight', 'minWidth'],
  legend: ['paddingLeft', 'paddingRight'],
  select: ['paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft', 'borderTop', 'borderRight', 'borderBottom', 'borderLeft', 'backgroundColor'],
};

// Values dumpRegion omits because they equal a plain div's computed style on calendar.google.com
// (body: "Google Sans Text" 14px/normal, color rgb(0, 0, 0); see gm3-light.json roles.body).
const DEFAULTS = {
  display: 'block', position: 'static',
  fontFamily: '"Google Sans Text", "Google Sans", Helvetica, Arial, sans-serif',
  fontSize: '14px', fontWeight: '400', lineHeight: 'normal', letterSpacing: 'normal', color: 'rgb(0, 0, 0)',
  backgroundColor: 'rgba(0, 0, 0, 0)', borderTop: '0px none rgb(0, 0, 0)', borderRight: '0px none rgb(0, 0, 0)',
  borderBottom: '0px none rgb(0, 0, 0)', borderLeft: '0px none rgb(0, 0, 0)', borderRadius: '0px',
  paddingTop: '0px', paddingRight: '0px', paddingBottom: '0px', paddingLeft: '0px',
  marginTop: '0px', marginRight: '0px', marginBottom: '0px', marginLeft: '0px',
  boxShadow: 'none', opacity: '1', minWidth: '0px', minHeight: '0px', gap: 'normal', flexDirection: 'row',
  alignItems: 'normal', justifyContent: 'normal', textTransform: 'none', whiteSpace: 'normal', overflow: 'visible',
  zIndex: 'auto', cursor: 'auto', fill: 'rgb(0, 0, 0)',
  backgroundImage: 'none', textDecorationLine: 'none', textAlign: 'start', boxSizing: 'content-box', flex: '0 1 auto',
  verticalAlign: 'baseline', textOverflow: 'clip', borderSpacing: '0px 0px', borderCollapse: 'separate', float: 'none', order: '0', alignSelf: 'auto',
};

const cache = new Map();
const missing = new Set();
function dump(name) {
  if (!cache.has(name)) {
    if (!existsSync(`${MEAS}${name}.json`)) {
      // A missing dark dump is reported once; the light value is used until it is captured.
      if (!missing.has(name)) { missing.add(name); console.warn(`missing dump ${name}.json (light values used)`); }
      cache.set(name, undefined);
      return undefined;
    }
    const d = JSON.parse(readFileSync(`${MEAS}${name}.json`, 'utf8'));
    for (const n of d.nodes || []) n._root = d.rootRect;
    cache.set(name, d);
  }
  return cache.get(name);
}

function nodeProp(n, p) {
  if (p === 'rect.w') return `${+n.rect[2].toFixed(2)}px`;
  if (p === 'rect.h') return `${+n.rect[3].toFixed(2)}px`;
  if (p === 'rect.x') return `${+n.rect[0].toFixed(2)}px`;
  if (p === 'rect.y') return `${+n.rect[1].toFixed(2)}px`;
  if (p === 'root.x') return `${+n._root[0].toFixed(2)}px`; // page position of the dump root
  if (p === 'root.y') return `${+n._root[1].toFixed(2)}px`;
  return n.style[p] === undefined ? undefined : String(n.style[p]);
}

/** Same node in the light and dark dump (dumps share the DOM, so the index is the same).
 *  `base` "gm3" reads a top-level key of gm3-<theme>.json instead (e.g. bodyBg). */
function themedNode(base, index, p) {
  if (base === 'gm3') {
    const l = dump('gm3-light')[p];
    const dk = dump('gm3-dark')[p];
    return l === dk ? l : { light: l, dark: dk };
  }
  const light = nodeProp(dump(`${base}-light`).nodes[index], p);
  let dark = light;
  const dn = dump(`${base}-dark`)?.nodes[index];
  if (dn) dark = nodeProp(dn, p);
  if (light === undefined && dark === undefined) return undefined;
  const l = light ?? DEFAULTS[p] ?? null;
  const dk = dark ?? DEFAULTS[p] ?? null;
  return l === dk ? l : { light: l, dark: dk };
}

function hasElementChildren(d, i) {
  const path = d.nodes[i].path + '/';
  return d.nodes.some((n) => n.path.startsWith(path));
}

let count = 0;
for (const [comp, def] of Object.entries(spec.component || {})) {
  const d = dump(`${def.source}-light`);
  const names = def.names || {};
  const size = new Set(def.size || []);
  const fluid = new Set(def.fluid || []); // never sized: they follow the window
  const fluidW = new Set(def.fluid_w || []); // width follows the window, height is measured
  const fluidH = new Set(def.fluid_h || []); // height follows the content, width is measured
  const skip = new Set(def.skip || []);
  const group = { source: `measurements/${def.source}-light.json`, _nodes: {} };
  const exportNode = (base, dd, i, key) => {
    const n = dd.nodes[i];
    const out = {};
    const always = new Set([...INHERITED, ...(UA_RESET[n.tag] || [])]);
    for (const p of EXPORTED) {
      const v = themedNode(base, i, p);
      if (v !== undefined) out[p] = v;
      else if (always.has(p)) out[p] = DEFAULTS[p];
    }
    // Relative offsets (top/left) come straight from the dump; absolute ones are computed below.
    if (n.style.position === 'relative') {
      for (const p of ['top', 'left']) if (n.style[p] !== undefined && n.style[p] !== 'auto') out[p] = themedNode(base, i, p);
    }
    const leaf = !hasElementChildren(dd, i);
    if (!fluid.has(key) && (leaf || size.has(key) || size.has('*') || n.style.position === 'absolute')) {
      if (n.style.width !== undefined && !fluidW.has(key)) out.width = themedNode(base, i, 'width');
      if (n.style.height !== undefined && !fluidH.has(key)) out.height = themedNode(base, i, 'height');
    }
    out.rect_w = `${+n.rect[2].toFixed(2)}px`;
    out.rect_h = `${+n.rect[3].toFixed(2)}px`;
    // Absolutely positioned nodes: offset from the nearest positioned ancestor (or the root),
    // so the generated CSS can place them exactly (tooltips parked at -10000px, ripples, ...).
    if (n.style.position === 'absolute' || n.style.position === 'fixed') {
      const ancestors = dd.nodes.filter((a) => n.path.startsWith(a.path + '/') && (a.style.position === 'relative' || a.style.position === 'absolute' || a.style.position === 'fixed' || a.style.position === 'sticky'));
      const anc = ancestors[ancestors.length - 1] || dd.nodes[0];
      // The rect includes the margins (a screen-reader <h1> keeps its UA margin); left/top do not.
      const ml = parseFloat(n.style.marginLeft || '0') || 0;
      const mt = parseFloat(n.style.marginTop || '0') || 0;
      out.left = `${+(n.rect[0] - anc.rect[0] - ml).toFixed(2)}px`;
      out.top = `${+(n.rect[1] - anc.rect[1] - mt).toFixed(2)}px`;
    }
    group[key] = out;
    group._nodes[key] = { source: base, index: i, path: n.path, tag: n.tag, role: n.role || null, aria: n.aria || null, text: n.text || null };
    count += Object.keys(out).length;
  };
  d.nodes.forEach((n, i) => {
    const named = names[String(i)];
    if (def.only_named && !named) return;
    const key = named || `n${i}`;
    if (skip.has(key)) return;
    exportNode(def.source, d, i, key);
  });
  for (const [key, [base, i]] of Object.entries(def.extra || {})) exportNode(base, dump(`${base}-light`), Number(i), key);
  tokens.component[comp] = group;
}
for (const [role, props] of Object.entries(spec.font || {})) {
  tokens.font[role] ??= {};
  for (const [k, [base, i, p]] of Object.entries(props)) { tokens.font[role][k] = themedNode(base, Number(i), p) ?? DEFAULTS[p] ?? null; count++; }
}
for (const [name, [base, i, p]] of Object.entries(spec.color || {})) {
  const v = themedNode(base, Number(i), p);
  tokens.color.light[name] = v && typeof v === 'object' ? v.light : v ?? null;
  tokens.color.dark[name] = v && typeof v === 'object' ? v.dark : v ?? null;
  count += 2;
}
for (const [name, [base, i, p]] of Object.entries(spec.layout || {})) { tokens.layout[name] = themedNode(base, Number(i), p) ?? DEFAULTS[p] ?? null; count++; }
for (const [k, v] of Object.entries(spec.set || {})) { let o = tokens; const ks = k.split('.'); for (const kk of ks.slice(0, -1)) o = o[kk] ??= {}; o[ks[ks.length - 1]] = v; count++; }

// Event and calendar palettes (docs/04 section 7) from scripts/measure/palette.mjs.
const paletteFile = `${MEAS}palette.json`;
if (existsSync(paletteFile)) {
  const pal = JSON.parse(readFileSync(paletteFile, 'utf8'));
  const pair = (l, d) => (l === d ? l : { light: l, dark: d });
  tokens.color.event_palette.source = 'measurements/palette.json (chip background and title colour per colorId, light and dark)';
  for (const [id, v] of Object.entries(pal.event)) {
    const cur = tokens.color.event_palette[id] || {};
    tokens.color.event_palette[id] = { name: cur.name || null, bg: pair(v.light.bg, v.dark.bg), fg: pair(v.light.fg, v.dark.fg), hover: pair(v.light.bg, v.dark.bg) };
    count += 3;
  }
  tokens.color.calendar_palette = { source: 'measurements/palette.json (fixture calendar colorId 1..24 cycled through calendarList.patch; classic = API backgroundColor)', _note: 'The API reports the classic hex; the UI paints the measured tone.' };
  for (const [id, v] of Object.entries(pal.calendar)) {
    tokens.color.calendar_palette[id] = { classic: v.classic, bg: pair(v.light.bg, v.dark.bg), fg: pair(v.light.fg, v.dark.fg) };
    count += 2;
  }
}

tokens.meta.measured_at = new Date().toISOString().slice(0, 10);
tokens.meta.chrome_version = dump('gm3-light').chrome || null;
writeFileSync(`${ROOT}docs/design/tokens.json`, JSON.stringify(tokens, null, 2) + '\n');
console.log(`wrote ${count} tokens into docs/design/tokens.json`);
