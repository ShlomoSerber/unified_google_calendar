#!/usr/bin/env node
// Print a dumpRegion JSON as an indexed tree: index, path, tag/role/aria/text, rect, display.
// Usage: node scripts/measure/tree.mjs <file.json> [maxDepth]
import { readFileSync } from 'node:fs';
const [file, depthArg] = process.argv.slice(2);
const maxDepth = Number(depthArg || 99);
const d = JSON.parse(readFileSync(file, 'utf8'));
d.nodes.forEach((n, i) => {
  const depth = n.path.split('/').length - 1;
  if (depth > maxDepth) return;
  const sem = [n.role && `role=${n.role}`, n.aria && `aria="${n.aria.slice(0, 30)}"`, n.text && `"${n.text.slice(0, 30)}"`].filter(Boolean).join(' ');
  const s = n.style;
  const box = [s.display, s.position, s.flexDirection, s.width && `w=${s.width}`, s.height && `h=${s.height}`].filter(Boolean).join(' ');
  console.log(`${String(i).padStart(3)} ${'  '.repeat(depth)}<${n.tag}> ${sem}  [${n.rect.map(v => +v.toFixed(1)).join(',')}]  ${box}`);
});
