#!/usr/bin/env node
// Print a JSX skeleton of a measured component: same tags, roles, aria-labels and text as the
// Google dump, with class names <comp>-<key> from docs/design/token-spec.json. A starting point
// that is then edited by hand for dynamic content. Usage: node scripts/measure/scaffold-jsx.mjs <comp>
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const comp = process.argv[2];
const spec = JSON.parse(readFileSync(`${ROOT}docs/design/token-spec.json`, 'utf8')).component[comp];
const d = JSON.parse(readFileSync(`${ROOT}docs/design/measurements/${spec.source}-light.json`, 'utf8'));
const names = spec.names || {};
const kebab = (s) => s.replace(/_/g, '-');
const nodes = d.nodes.map((n, i) => ({ ...n, i, key: names[String(i)] || `n${i}`, depth: n.path.split('/').length - 1 }));
const VOID = new Set(['img', 'input', 'br', 'hr', 'circle', 'path', 'image']);
for (let i = 0; i < nodes.length; i++) {
  const n = nodes[i];
  const next = nodes[i + 1];
  const attrs = [`className="${kebab(comp)}-${kebab(n.key)}"`];
  if (n.role) attrs.push(`role="${n.role}"`);
  if (n.aria) attrs.push(`aria-label=${JSON.stringify(n.aria)}`);
  const hasChildren = next && next.depth > n.depth;
  const pad = '  '.repeat(n.depth);
  const text = n.text ? `{${JSON.stringify(n.text)}}` : '';
  if (!hasChildren) {
    console.log(`${pad}<${n.tag} ${attrs.join(' ')}${VOID.has(n.tag) ? ' />' : `>${text}</${n.tag}>`}`);
  } else {
    console.log(`${pad}<${n.tag} ${attrs.join(' ')}>${text}`);
  }
  // close parents when depth decreases
  const nextDepth = next ? next.depth : 0;
  if (hasChildren) {
    // closing handled when we walk back up
  }
  if (!next || next.depth <= n.depth) {
    // close ancestors from n.depth-1 down to nextDepth
    let depth = n.depth - 1;
    while (depth >= nextDepth) {
      const parent = [...nodes.slice(0, i)].reverse().find((p) => p.depth === depth);
      if (parent && !VOID.has(parent.tag)) console.log(`${'  '.repeat(depth)}</${parent.tag}>`);
      depth--;
    }
  }
}
