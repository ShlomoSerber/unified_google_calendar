// Calendar and event colours as Google paints them (docs/04 section 7): the backend stores the
// hex the API reports; the UI shows the measured light/dark tone from docs/design/tokens.json.
import { useSyncExternalStore } from 'react';
import { COLOR_MAP } from '../styles/palette';

export type Theme = 'light' | 'dark';

const query = typeof window !== 'undefined' && typeof window.matchMedia === 'function' ? window.matchMedia('(prefers-color-scheme: dark)') : null;

/** The system theme (docs/08 F4-T5: `prefers-color-scheme`, no manual selector in version 1). */
export function useTheme(): Theme {
  return useSyncExternalStore(
    (cb) => {
      query?.addEventListener('change', cb);
      return () => query?.removeEventListener('change', cb);
    },
    () => (query?.matches ? 'dark' : 'light'),
    () => 'light',
  );
}

/** Measured chip background for a stored colour, or the colour itself when it is not in the palette. */
export function chipBackground(hex: string, theme: Theme): string {
  return COLOR_MAP[hex.toLowerCase()]?.[theme].bg ?? hex;
}
