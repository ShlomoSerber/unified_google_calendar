#!/usr/bin/env node
// Fails if any var(--...) used under src/ is not defined in src/styles/tokens.css,
// or if a component stylesheet contains a literal px/color/font value.
// Usage: node scripts/check-tokens.mjs
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const tokensCss = resolve(root, 'src/styles/tokens.css');
if (!existsSync(tokensCss)) { console.error('src/styles/tokens.css missing: run node scripts/gen-tokens.mjs'); process.exit(1); }
const defined = new Set([...readFileSync(tokensCss, 'utf8').matchAll(/(--[a-z0-9-]+)\s*:/g)].map(m => m[1]));

const files = [];
const walk = d => { for (const e of readdirSync(d)) { const p = join(d, e); const s = statSync(p);
  if (s.isDirectory()) walk(p); else if (/\.(css|tsx?)$/.test(e)) files.push(p); } };
if (existsSync(resolve(root, 'src'))) walk(resolve(root, 'src'));

const problems = [];
const LITERAL = /(?<![\w-])(\d+(\.\d+)?px|#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\()/;
const ALLOWED_LITERAL_FILES = /src\/styles\/(tokens|fonts|base)\.css$/;
for (const f of files) {
  const src = readFileSync(f, 'utf8');
  for (const m of src.matchAll(/var\((--[a-z0-9-]+)/g)) if (!defined.has(m[1])) problems.push(`${f}: token ${m[1]} is not defined (null or missing in tokens.json)`);
  if (f.endsWith('.css') && !ALLOWED_LITERAL_FILES.test(f)) {
    src.split('\n').forEach((line, i) => {
      const code = line.replace(/\/\*.*?\*\//g, '');
      if (LITERAL.test(code) && !/^\s*\/\//.test(code) && !/\b0px\b/.test(code)) problems.push(`${f}:${i + 1}: literal value in stylesheet, use a token: ${code.trim()}`);
    });
  }
}
const nulls = [];
const scan = (o, p) => { for (const [k, v] of Object.entries(o)) { if (v === null) nulls.push(`${p}.${k}`); else if (typeof v === 'object' && !Array.isArray(v)) scan(v, `${p}.${k}`); } };
scan(JSON.parse(readFileSync(resolve(root, 'docs/design/tokens.json'), 'utf8')), 'tokens');

console.log(`${defined.size} tokens defined, ${files.length} source files scanned, ${nulls.length} tokens still null in tokens.json`);
for (const p of problems) console.log('ERROR ' + p);
process.exit(problems.length ? 1 : 0);
