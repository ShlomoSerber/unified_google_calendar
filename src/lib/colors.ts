// Calendar and event colours (docs/11 section 6): the backend stores the hex the API reports;
// the UI shows the Material 3 custom-color roles derived from it in src/styles/palette.ts.
import { useSyncExternalStore } from 'react';
import { COLOR_MAP, type ChipColors } from '../styles/palette';
import { COLOR_MAP as LEGACY_COLOR_MAP } from '../styles/legacy-palette';

export type Theme = 'light' | 'dark';
export type { ChipColors };

const query = typeof window !== 'undefined' && typeof window.matchMedia === 'function' ? window.matchMedia('(prefers-color-scheme: dark)') : null;

/** The system theme (`prefers-color-scheme`, no manual selector in version 1). */
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

/**
 * Chip roles (border and dot, container, text) for a stored colour. A hex outside the palette
 * (a colour Google adds later) keeps its own tone for the border and a 24 % mix over the
 * surface for the container, with the surface's text colour on it.
 */
export function chipColors(hex: string, theme: Theme): ChipColors {
  const known = COLOR_MAP[hex.toLowerCase()];
  if (known) return known[theme];
  return { color: hex, container: `color-mix(in srgb, ${hex} 24%, var(--md-sys-color-surface))`, onContainer: 'var(--md-sys-color-on-surface)' };
}

/** Until M3-T2 of the transition: the Google-measured chip background the unmigrated
 *  components still paint (docs/12). Deleted with legacy-palette.ts. */
export function chipBackground(hex: string, theme: Theme): string {
  return LEGACY_COLOR_MAP[hex.toLowerCase()]?.[theme].bg ?? hex;
}
