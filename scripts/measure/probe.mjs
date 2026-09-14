#!/usr/bin/env node
// Smoke test: is the measurement profile signed in? Prints the page title and URL.
import { writeFileSync } from 'node:fs';
import { Cdp, launchChrome } from './cdp.mjs';

const chrome = await launchChrome();
try {
  const cdp = await Cdp.connect();
  await cdp.setViewport(1440, 900);
  await cdp.navigate('https://calendar.google.com/calendar/u/0/r/week', { waitMs: 8000 });
  const info = await cdp.eval('({ title: document.title, url: location.href, main: !!document.querySelector("[role=main]") })');
  console.log(JSON.stringify(info));
  writeFileSync('/tmp/claude-1000/probe.png', await cdp.screenshot());
  cdp.close();
} finally {
  chrome.kill();
}
