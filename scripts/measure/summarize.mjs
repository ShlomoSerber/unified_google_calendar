#!/usr/bin/env node
// Print a compact tree of a dumpRegion JSON: depth, tag/role/aria/text, rect and the styles
// that differ from a plain div. Usage: node scripts/measure/summarize.mjs <file.json> [--all]
import { readFileSync } from 'node:fs';
const [file, flag] = process.argv.slice(2);
const d = JSON.parse(readFileSync(file, 'utf8'));
const all = flag === '--all';
console.log(`${file}: ${d.nodes.length} nodes, root ${d.rootRect.map((v) => +v.toFixed(1)).join(',')}`);
for (const n of d.nodes) {
  const depth = n.path.split('/').length - 1;
  const sem = [n.role && `role=${n.role}`, n.aria && `aria="${n.aria.slice(0, 40)}"`, n.text && `"${n.text.slice(0, 40)}"`].filter(Boolean).join(' ');
  if (!all && !sem) continue;
  const st = Object.entries(n.style)
    .filter(([k]) => !['display', 'position', 'width', 'height', 'minWidth', 'minHeight', 'overflow', 'zIndex', 'cursor', 'outline', 'fill', 'flexDirection', 'alignItems', 'justifyContent', 'gap'].includes(k))
    .map(([k, v]) => `${k}=${String(v).replace(/\s+/g, ' ').slice(0, 60)}`)
    .join('; ');
  console.log(`${'  '.repeat(Math.min(depth, 12))}<${n.tag}> ${sem}  [${n.rect.join(',')}]  ${st}`);
}
