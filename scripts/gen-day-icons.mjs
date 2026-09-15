#!/usr/bin/env node
// Renders the day-of-month icons the tray and the window show (tray.rs `day_icon`): the same
// orange rounded square as src-tauri/icons/app-icon.svg with the day number as the glyph, white
// with a black outline. Output: src-tauri/icons/day/NN.rgba (raw RGBA, SIZE×SIZE, no header,
// embedded with include_bytes! — no PNG decoder in the binary), NN-<size>.png for each hicolor
// size the .deb installs (tray.rs copies them into the user's icon theme so GNOME's dock shows
// the day too; a directory's icons must have its declared pixel size) and a preview sheet.
// Rendering uses the headless Chrome of the measurement pipeline (scripts/measure/cdp.mjs).
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Cdp, launchChrome } from './measure/cdp.mjs';

const SIZE = 64;
const PNG_SIZES = [32, 128, 512];
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const out = resolve(root, 'src-tauri/icons/day');
mkdirSync(out, { recursive: true });

const svg = (day) => `<svg xmlns="http://www.w3.org/2000/svg" width="${SIZE}" height="${SIZE}" viewBox="0 0 512 512">
  <rect x="16" y="16" width="480" height="480" rx="112" fill="#D97757"/>
  <text x="256" y="256" text-anchor="middle" dominant-baseline="central"
        font-family="'Noto Sans', 'Roboto', 'DejaVu Sans', sans-serif" font-weight="700" font-size="330"
        fill="#ffffff" stroke="#000000" stroke-width="12" stroke-linejoin="round" paint-order="stroke">${day}</text>
</svg>`;

const chrome = await launchChrome({ width: 200, height: 200 });
try {
  const cdp = await Cdp.connect();
  await cdp.navigate('about:blank', { waitMs: 300 });
  const sheet = [];
  for (let day = 1; day <= 31; day++) {
    const data = await cdp.eval(`(async () => {
      const img = new Image();
      img.src = 'data:image/svg+xml;base64,' + btoa(${JSON.stringify(svg(day))});
      await img.decode();
      const c = document.createElement('canvas');
      c.width = ${SIZE}; c.height = ${SIZE};
      c.getContext('2d').drawImage(img, 0, 0, ${SIZE}, ${SIZE});
      return Array.from(c.getContext('2d').getImageData(0, 0, ${SIZE}, ${SIZE}).data);
    })()`);
    if (data.length !== SIZE * SIZE * 4) throw new Error(`day ${day}: ${data.length} bytes`);
    writeFileSync(resolve(out, `${String(day).padStart(2, '0')}.rgba`), Buffer.from(data));
    for (const px of PNG_SIZES) {
      const png = await cdp.eval(`(async () => {
        const img = new Image();
        img.src = 'data:image/svg+xml;base64,' + btoa(${JSON.stringify(svg(day))});
        await img.decode();
        const c = document.createElement('canvas');
        c.width = ${px}; c.height = ${px};
        c.getContext('2d').drawImage(img, 0, 0, ${px}, ${px});
        return c.toDataURL('image/png').split(',')[1];
      })()`);
      writeFileSync(resolve(out, `${String(day).padStart(2, '0')}-${px}.png`), Buffer.from(png, 'base64'));
    }
    sheet.push(day);
  }
  // Preview sheet (PNG) for a human check; not used by the app.
  const png = await cdp.eval(`(async () => {
    const days = ${JSON.stringify(sheet)};
    const c = document.createElement('canvas');
    c.width = ${SIZE} * 8; c.height = ${SIZE} * 4;
    const ctx = c.getContext('2d');
    for (const [i, d] of days.entries()) {
      const img = new Image();
      img.src = 'data:image/svg+xml;base64,' + btoa(${JSON.stringify(svg('DAY'))}.replace('DAY', d));
      await img.decode();
      ctx.drawImage(img, (i % 8) * ${SIZE}, Math.floor(i / 8) * ${SIZE}, ${SIZE}, ${SIZE});
    }
    return c.toDataURL('image/png').split(',')[1];
  })()`);
  writeFileSync(resolve(out, 'preview.png'), Buffer.from(png, 'base64'));
  cdp.close();
  console.log(`wrote 31 icons (${SIZE}×${SIZE} RGBA, PNG at ${PNG_SIZES.join('/')} px) and preview.png to src-tauri/icons/day/`);
} finally {
  chrome.kill();
}
