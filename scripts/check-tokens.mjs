#!/usr/bin/env node
// Fails if any var(--...) used under src/ is not defined in src/styles/tokens.css, or if a
// component stylesheet contains a literal px/color/font value (docs/11 section 3).
// Usage: node scripts/check-tokens.mjs
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const tokensCss = resolve(root, 'src/styles/tokens.css');
if (!existsSync(tokensCss)) { console.error('src/styles/tokens.css missing: run node scripts/gen-m3-tokens.mjs'); process.exit(1); }
const defined = new Set([...readFileSync(tokensCss, 'utf8').matchAll(/(--[a-z0-9-]+)\s*:/g)].map(m => m[1]));

const files = [];
const walk = d => { for (const e of readdirSync(d)) { const p = join(d, e); const s = statSync(p);
  if (s.isDirectory()) walk(p); else if (/\.(css|tsx?)$/.test(e)) files.push(p); } };
if (existsSync(resolve(root, 'src'))) walk(resolve(root, 'src'));

const problems = [];
const isComponentToken = (name) => name.startsWith('--md-') && !name.startsWith('--md-sys-') && !name.startsWith('--md-ref-');
const LITERAL = /(?<![\w-])(\d+(\.\d+)?px|#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\()/;
const ALLOWED_LITERAL_FILES = /src\/styles\/(tokens|typescale|fonts|base)\.css$/;
for (const f of files) {
  const src = readFileSync(f, 'utf8');
  // --data-* variables carry values that come from the calendar data at runtime (a calendar's
  // colour), set inline by the component; they are not design tokens.
  // --md-<component>-* are @material/web component tokens (docs/11 section 3, rule 2): set by the
  // app's stylesheets, read by the components' shadow DOM, never defined in tokens.css.
  for (const m of src.matchAll(/var\((--[a-z0-9-]+)/g)) if (!defined.has(m[1]) && !m[1].startsWith('--data-') && !isComponentToken(m[1])) problems.push(`${f}: token ${m[1]} is not defined (missing in theme.json)`);
  if (f.endsWith('.css') && !ALLOWED_LITERAL_FILES.test(f)) {
    src.split('\n').forEach((line, i) => {
      const code = line.replace(/\/\*.*?\*\//g, '');
      if (LITERAL.test(code) && !/^\s*\/\//.test(code) && !/\b0px\b/.test(code)) problems.push(`${f}:${i + 1}: literal value in stylesheet, use a token: ${code.trim()}`);
    });
  }
}
console.log(`${defined.size} tokens defined, ${files.length} source files scanned`);
for (const p of problems) console.log('ERROR ' + p);
process.exit(problems.length ? 1 : 0);
