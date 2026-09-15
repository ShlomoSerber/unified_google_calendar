#!/usr/bin/env node
// Toggle the calendar checkboxes of the measurement profile (docs/04 section 2.2, docs/99 F4-T1):
//   node scripts/measure/reference-state.mjs on    → only "UGC Fixtures" visible (before capturing)
//   node scripts/measure/reference-state.mjs off   → every calendar visible again (the user's normal view)
// Nothing else of the account is touched.
import { Cdp, launchChrome } from './cdp.mjs';

const mode = process.argv[2];
if (mode !== 'on' && mode !== 'off') { console.error('usage: reference-state.mjs on|off'); process.exit(2); }
const SEL = `[...document.querySelectorAll('[role=complementary] input[type=checkbox], [role=complementary] [role=checkbox]')]`;
const chrome = await launchChrome();
try {
  const cdp = await Cdp.connect();
  await cdp.setViewport(1440, 900);
  await cdp.navigate('https://calendar.google.com/calendar/u/0/r/week', { waitMs: 8000 });
  const read = () => cdp.eval(`${SEL}.map(c => ({ label: c.getAttribute('aria-label'), checked: c.checked === true || c.getAttribute('aria-checked') === 'true' }))`);
  for (const b of await read()) {
    const want = mode === 'off' ? true : b.label === 'UGC Fixtures';
    if (b.checked !== want) {
      await cdp.clickElement(`${SEL}.find(c => c.getAttribute('aria-label') === ${JSON.stringify(b.label)})`);
      await cdp.sleep(1200);
    }
  }
  console.log(JSON.stringify(await read()));
  cdp.close();
} finally {
  chrome.kill();
}
