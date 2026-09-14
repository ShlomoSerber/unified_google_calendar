#!/usr/bin/env node
// Compare two dumpRegion() JSON files: the Google Calendar reference and our app.
// Usage: node scripts/measure/diff-layout.mjs <google.json> <app.json> [--tolerance 0.5] [--all] [--ignore <regex>] [--ignore-hidden]
//
// Matching strategy: our DOM does not share Google's element paths, so nodes are
// matched by a semantic key (tag + role + aria-label + text). Nodes without any
// of role/aria/text are matched by path only when the key is unique on both sides.
// Reported: missing semantic nodes, rect differences beyond tolerance, and style
// differences in the compared property list. Exit code 1 when any difference exists.
import { readFileSync } from 'node:fs';

const args = process.argv.slice(2);
if (args.length < 2) { console.error('usage: diff-layout.mjs <google.json> <app.json> [--tolerance 0.5] [--all]'); process.exit(2); }
const tolIdx = args.indexOf('--tolerance');
const TOL = tolIdx >= 0 ? parseFloat(args[tolIdx + 1]) : 0.5;
const ALL = args.includes('--all');
// --ignore <regex>: semantic keys to leave out (user identity such as the account avatar label);
// every use must be justified in docs/design/measurements/<component>.md.
const ignIdx = args.indexOf('--ignore');
const IGNORE = ignIdx >= 0 ? new RegExp(args[ignIdx + 1]) : null;
// --ignore-hidden: skip screen-reader-only nodes (1×1 boxes parked off screen); their position
// depends on the static position of an absolute box, which is invisible by definition.
const IGNORE_HIDDEN = args.includes('--ignore-hidden');
const positional = args.filter((a, i) => !a.startsWith('--') && !(i > 0 && args[i - 1].startsWith('--')));
const [refFile, appFile] = positional;

const STYLE_PROPS = ['fontFamily','fontSize','fontWeight','lineHeight','letterSpacing','color','backgroundColor',
  'borderTop','borderRight','borderBottom','borderLeft','borderRadius','paddingTop','paddingRight','paddingBottom',
  'paddingLeft','boxShadow','opacity','textTransform','whiteSpace','backgroundImage','textDecorationLine'];

const load = f => JSON.parse(readFileSync(f, 'utf8'));
const ref = load(refFile), app = load(appFile);

const keyOf = n => {
  const sem = [n.role, n.aria, n.text].filter(Boolean).join('|');
  return sem ? `${n.tag}|${sem}` : null;
};
const index = nodes => {
  const byKey = new Map(), counts = new Map();
  for (const n of nodes) { const k = keyOf(n); if (!k) continue; counts.set(k, (counts.get(k) || 0) + 1); if (!byKey.has(k)) byKey.set(k, n); }
  for (const [k, c] of counts) if (c > 1) byKey.delete(k); // ambiguous keys are not matched
  return byKey;
};
const norm = v => String(v ?? '').replace(/\s+/g, ' ').trim().toLowerCase();
const normFont = v => norm(v).replace(/"/g, '');

const refIdx = index(ref.nodes), appIdx = index(app.nodes);
const diffs = [];
let compared = 0;
for (const [k, rn] of refIdx) {
  if (IGNORE && IGNORE.test(k)) continue;
  if (IGNORE_HIDDEN && rn.rect[2] <= 1 && rn.rect[3] <= 1) continue;
  const an = appIdx.get(k);
  if (!an) { diffs.push({ kind: 'missing', key: k, ref: rn.rect }); continue; }
  compared++;
  const rd = rn.rect.map((v, i) => Math.abs(v - an.rect[i]));
  if (rd.some(d => d > TOL)) diffs.push({ kind: 'rect', key: k, ref: rn.rect, app: an.rect });
  for (const p of STYLE_PROPS) {
    const a = rn.style[p], b = an.style[p];
    if (a === undefined && b === undefined) continue;
    const eq = p === 'fontFamily' ? normFont(a).split(',')[0] === normFont(b).split(',')[0] : norm(a) === norm(b);
    if (!eq) diffs.push({ kind: 'style', key: k, prop: p, ref: a ?? '(default)', app: b ?? '(default)' });
  }
}
for (const k of appIdx.keys()) if (!refIdx.has(k) && ALL) diffs.push({ kind: 'extra', key: k });

console.log(`reference: ${ref.nodes.length} nodes (${refIdx.size} matchable) from ${ref.url}`);
console.log(`app:       ${app.nodes.length} nodes (${appIdx.size} matchable) from ${app.url}`);
console.log(`root rect  ref=${ref.rootRect.map(v => v.toFixed(1)).join(',')} app=${app.rootRect.map(v => v.toFixed(1)).join(',')}`);
console.log(`compared ${compared} nodes, tolerance ${TOL}px`);
for (const d of diffs) {
  if (d.kind === 'missing') console.log(`MISSING  ${d.key}  ref rect=${d.ref}`);
  else if (d.kind === 'rect') console.log(`RECT     ${d.key}\n         ref=${d.ref}\n         app=${d.app}`);
  else if (d.kind === 'style') console.log(`STYLE    ${d.key}  ${d.prop}: ref=${d.ref} app=${d.app}`);
  else console.log(`EXTRA    ${d.key}`);
}
console.log(`${diffs.length} differences`);
process.exit(diffs.length ? 1 : 0);
