#!/usr/bin/env node
// Interactive-ish helper: keeps one headless Chrome open and runs JS snippets against Google
// Calendar. Usage: node scripts/measure/session.mjs <url> <file-with-js-expression> [waitMs]
import { readFileSync, writeFileSync } from 'node:fs';
import { Cdp, launchChrome } from './cdp.mjs';

const [url, file, waitMs = '6000', shot] = process.argv.slice(2);
const chrome = await launchChrome({ dark: process.env.DARK === '1' });
try {
  const cdp = await Cdp.connect();
  await cdp.setViewport(1440, 900);
  await cdp.navigate(url, { waitMs: Number(waitMs) });
  const expr = readFileSync(file, 'utf8');
  const result = await cdp.eval(expr);
  console.log(JSON.stringify(result, null, 1));
  if (shot) writeFileSync(shot, await cdp.screenshot());
  cdp.close();
} finally {
  chrome.kill();
}
