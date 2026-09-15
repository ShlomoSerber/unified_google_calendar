// The snackbar of component 24 (docs/design/measurements/snackbar.md): Google shows "Saving..." /
// "Deleting..." the moment a request starts and "Event saved" / "Event deleted" when it returns.
// `withNotice` wraps a write so every caller gets the same feedback (docs/99, 2026-09-15).
import { useUi } from '../state/ui';
import { MOTION } from '../styles/motion';
import { ms } from './motion';

let holdTimer = 0;
let serial = 0;

/** Runs `work` with the busy text on screen, then shows `done` for snackbar_hold_duration. */
export async function withNotice<T>(busy: string, done: string, work: () => Promise<T>): Promise<T> {
  const id = ++serial;
  window.clearTimeout(holdTimer);
  useUi.getState().setNotice({ text: busy, busy: true });
  try {
    const result = await work();
    useUi.getState().setNotice({ text: done, busy: false });
    holdTimer = window.setTimeout(() => {
      if (id === serial) useUi.getState().setNotice(null);
    }, ms(MOTION.snackbar_hold_duration));
    return result;
  } catch (e) {
    if (id === serial) useUi.getState().setNotice(null);
    throw e;
  }
}

export function dismissNotice(): void {
  window.clearTimeout(holdTimer);
  useUi.getState().setNotice(null);
}
