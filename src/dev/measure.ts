// Development-only bridge for the pixel-fidelity loop (docs/04 section 3 steps 8-10).
// Listens to the `dev:measure` event emitted by `POST /dev/measure`, performs the requested
// actions on the UI, runs scripts/measure/dumpRegion.js on the root element and hands the
// JSON to the `dev_dump` command, which writes `<name>-app.json`. Loaded only in dev builds.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import dumpRegionSource from '../../scripts/measure/dumpRegion.js?raw';
import { ipc } from '../ipc';
import { useUi, type ViewKind } from '../state/ui';

interface Action {
  type: 'click' | 'hover' | 'wait' | 'view' | 'date' | 'scroll' | 'unhover' | 'calendars' | 'key' | 'click_at' | 'type';
  key?: string;
  x?: number;
  text?: string;
  selector?: string;
  ms?: number;
  view?: ViewKind;
  ts?: number;
  y?: number;
  /** `calendars`: show only the calendars with these names (empty list: show all). */
  only?: string[];
}

interface MeasureRequest {
  name: string;
  root: string;
  actions?: Action[];
}

declare global {
  interface Window {
    dumpRegion?: (root: Element, opts?: object) => unknown;
    $0?: Element;
  }
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function fire(el: Element, types: string[]) {
  const r = el.getBoundingClientRect();
  for (const type of types) {
    el.dispatchEvent(
      new MouseEvent(type, { bubbles: true, cancelable: true, clientX: r.x + r.width / 2, clientY: r.y + r.height / 2 }),
    );
  }
}

async function run(req: MeasureRequest): Promise<void> {
  for (const a of req.actions ?? []) {
    const el = a.selector ? document.querySelector(a.selector) : null;
    switch (a.type) {
      case 'click':
        if (el instanceof HTMLElement) el.click();
        break;
      case 'hover':
        if (el) fire(el, ['pointerover', 'pointerenter', 'mouseover', 'mouseenter', 'mousemove']);
        break;
      case 'unhover':
        if (el) fire(el, ['pointerout', 'pointerleave', 'mouseout', 'mouseleave']);
        break;
      case 'view':
        if (a.view) useUi.getState().setView(a.view);
        break;
      case 'date':
        if (a.ts) useUi.getState().setDate(a.ts);
        break;
      case 'scroll':
        if (el && a.y !== undefined) el.scrollTop = a.y;
        break;
      case 'click_at': {
        // A synthetic click at viewport coordinates (the quick-create slot).
        const target = document.elementFromPoint(a.x ?? 0, a.y ?? 0);
        if (target) target.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, clientX: a.x ?? 0, clientY: a.y ?? 0 }));
        break;
      }
      case 'type': {
        const input = document.activeElement;
        if (input instanceof HTMLInputElement) {
          const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
          setter?.call(input, a.text ?? '');
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
        break;
      }
      case 'key':
        document.dispatchEvent(new KeyboardEvent('keydown', { key: a.key ?? 'Escape', bubbles: true }));
        break;
      case 'calendars': {
        // Local visibility only (docs/03): the reference shows the fixtures calendar alone.
        const { calendars, setCalendars } = useUi.getState();
        const only = a.only ?? [];
        const next = calendars.map((c) => ({ ...c, visible: only.length === 0 || only.includes(c.summary) }));
        setCalendars(next);
        await Promise.all(next.map((c) => ipc.setCalendarVisible(c.account_id, c.id, c.visible)));
        await sleep(800);
        break;
      }
      case 'wait':
        await sleep(a.ms ?? 300);
        break;
    }
    await sleep(50);
  }
  await sleep(250);
  if (!window.dumpRegion) {
    // The snippet is an IIFE that defines window.dumpRegion.
    window.$0 = document.body;
    new Function(dumpRegionSource)();
  }
  const root = document.querySelector(req.root);
  if (!root || !window.dumpRegion) {
    await invoke('dev_dump', { name: req.name, json: JSON.stringify({ error: `root not found: ${req.root}` }) });
    return;
  }
  const result = window.dumpRegion(root, { keepDefaults: false }) as Record<string, unknown>;
  // Which font faces actually loaded (docs/04 section 6: the proprietary families come from Google Fonts at runtime).
  result.fonts = [...document.fonts].map((f) => `${f.family} ${f.weight} ${f.status}`);
  await invoke('dev_dump', { name: req.name, json: JSON.stringify(result) });
}

export function installMeasureBridge(): void {
  listen<MeasureRequest>('dev:measure', (e) => {
    run(e.payload).catch((err: unknown) => {
      invoke('dev_dump', { name: e.payload.name, json: JSON.stringify({ error: String(err) }) }).catch(() => undefined);
    });
  }).catch(() => undefined);
}
