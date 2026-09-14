#!/usr/bin/env node
// Minimal Chrome DevTools Protocol client used by the measurement scripts (docs/04 section 3).
// Launches headless Chrome on the measurement profile (~/.chrome-measure) with remote
// debugging, or attaches to a running one, and exposes evaluate/screenshot helpers.
import { spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import WebSocket from 'ws';

const PORT = Number(process.env.CDP_PORT || 9333);

export async function launchChrome({ width = 1440, height = 900, headless = true, dark = false } = {}) {
  const args = [
    `--user-data-dir=${homedir()}/.chrome-measure`,
    `--remote-debugging-port=${PORT}`,
    `--window-size=${width},${height}`,
    '--hide-scrollbars=false',
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-features=TranslateUI',
    '--force-device-scale-factor=1',
    '--lang=en-US',
  ];
  if (headless) args.push('--headless=new');
  if (dark) args.push('--force-dark-mode');
  args.push('about:blank');
  const child = spawn('google-chrome', args, { stdio: ['ignore', 'ignore', 'pipe'] });
  child.stderr.on('data', () => {});
  for (let i = 0; i < 100; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${PORT}/json/version`);
      if (r.ok) return child;
    } catch {
      // not up yet
    }
    await new Promise((res) => setTimeout(res, 200));
  }
  child.kill();
  throw new Error('chrome did not start');
}

export class Cdp {
  constructor(ws) {
    this.ws = ws;
    this.id = 0;
    this.pending = new Map();
    this.listeners = new Map();
    ws.on('message', (data) => {
      const msg = JSON.parse(data.toString());
      if (msg.id && this.pending.has(msg.id)) {
        const { resolve, reject } = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        msg.error ? reject(new Error(msg.error.message)) : resolve(msg.result);
      } else if (msg.method && this.listeners.has(msg.method)) {
        for (const fn of this.listeners.get(msg.method)) fn(msg.params);
      }
    });
  }

  static async connect() {
    const targets = await (await fetch(`http://127.0.0.1:${PORT}/json`)).json();
    const page = targets.find((t) => t.type === 'page');
    if (!page) throw new Error('no page target');
    const ws = new WebSocket(page.webSocketDebuggerUrl, { perMessageDeflate: false, maxPayload: 256 * 1024 * 1024 });
    await new Promise((res, rej) => { ws.on('open', res); ws.on('error', rej); });
    const cdp = new Cdp(ws);
    await cdp.send('Page.enable');
    await cdp.send('Runtime.enable');
    return cdp;
  }

  send(method, params = {}) {
    const id = ++this.id;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  on(method, fn) {
    if (!this.listeners.has(method)) this.listeners.set(method, []);
    this.listeners.get(method).push(fn);
  }

  async navigate(url, { waitMs = 6000 } = {}) {
    await this.send('Page.navigate', { url });
    await new Promise((res) => setTimeout(res, waitMs));
  }

  /** Evaluate an expression and return its JSON value. */
  async eval(expression) {
    const r = await this.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description || r.exceptionDetails.text);
    return r.result.value;
  }

  async emulateDark(dark) {
    await this.send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: dark ? 'dark' : 'light' }] });
  }

  async setViewport(width, height) {
    await this.send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
  }

  /** PNG (base64) of the viewport or of a clip {x,y,width,height} in CSS px. */
  async screenshot(clip) {
    const params = { format: 'png', captureBeyondViewport: false };
    if (clip) params.clip = { ...clip, scale: 1 };
    const r = await this.send('Page.captureScreenshot', params);
    return Buffer.from(r.data, 'base64');
  }

  /** Real mouse click at CSS coordinates. */
  async clickAt(x, y) {
    for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) {
      await this.send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
    }
  }

  /** Real click on the center of the first element matching `selectorJs` (a JS expression returning an element). */
  async clickElement(selectorJs) {
    const box = await this.eval(`(() => { const el = ${selectorJs}; if (!el) return null; el.scrollIntoView({ block: 'center' }); const r = el.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`);
    if (!box) throw new Error(`element not found: ${selectorJs}`);
    await this.clickAt(box.x, box.y);
    return box;
  }

  /** Type text with real key events into the focused element. */
  async typeText(text) {
    await this.send('Input.insertText', { text });
  }

  async pressKey(key) {
    const codes = { Enter: 13, Escape: 27, ArrowDown: 40, ArrowUp: 38, Tab: 9 };
    const code = codes[key] ?? 0;
    await this.send('Input.dispatchKeyEvent', { type: 'keyDown', key, code: key, windowsVirtualKeyCode: code, nativeVirtualKeyCode: code });
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', key, code: key, windowsVirtualKeyCode: code, nativeVirtualKeyCode: code });
  }

  sleep(ms) {
    return new Promise((res) => setTimeout(res, ms));
  }

  close() {
    this.ws.close();
  }
}

export function dumpRegionSource() {
  return readFileSync(new URL('./dumpRegion.js', import.meta.url), 'utf8');
}
