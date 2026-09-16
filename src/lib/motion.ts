// Motion helpers (docs/11 section 10): keep an element mounted while its exit animation plays.
// Timings come from src/styles/motion.ts; nothing here is typed by hand.
import { useEffect, useState } from 'react';

/** "150ms" → 150. */
export const ms = (v: string): number => parseFloat(v);

/**
 * Presence with an exit delay: while `isOpen(value)` the current value is shown at once; when it
 * closes, the last open value stays rendered for `exitMs(last)` with `exiting` true, so the CSS
 * exit animation can play before the element unmounts.
 */
export function usePresence<T>(value: T, isOpen: (v: T) => boolean, exitMs: (last: T) => number): { shown: T; exiting: boolean } {
  const [state, setState] = useState<{ shown: T; exiting: boolean }>({ shown: value, exiting: false });
  // Derived during render so the exit class is committed in the same frame the dialog closes.
  if (isOpen(value)) {
    if (state.shown !== value || state.exiting) setState({ shown: value, exiting: false });
  } else if (state.shown !== value && !state.exiting) {
    setState(isOpen(state.shown) ? { shown: state.shown, exiting: true } : { shown: value, exiting: false });
  }
  const { shown, exiting } = state;
  useEffect(() => {
    if (!exiting) return undefined;
    const t = window.setTimeout(() => setState({ shown: value, exiting: false }), exitMs(shown));
    return () => window.clearTimeout(t);
    // `value` is the closed value while exiting; exitMs is a module-level constant function.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [exiting, shown]);
  return state;
}
