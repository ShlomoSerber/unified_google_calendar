import { useEffect, useState, type ReactElement } from 'react';
import { ms } from '../lib/motion';
import { MOTION } from '../styles/legacy-motion';
import type { ViewKind } from '../state/ui';

// The main area's motion (docs/04 section 10, measurements/animations.md): a date change slides
// the new grid in from the side of travel (nav_next: +56px → 0 with opacity 0 → 1, 200 ms); a
// view change cross-fades the old view out while the new one fades in (200 ms), so the old view
// stays mounted, inert, for that long. Each view renders in an absolute layer over .app-main.

export interface ViewStageProps {
  view: ViewKind;
  /** Key that changes whenever the visible range changes (view plus anchor day). */
  rangeKey: string;
  navDir: 1 | -1;
  children: ReactElement;
}

interface Track {
  key: string;
  view: ViewKind;
  enter: '' | 'fade' | 'forward' | 'back';
  leaving: { key: string; element: ReactElement } | null;
}

export function ViewStage({ view, rangeKey, navDir, children }: ViewStageProps) {
  const [track, setTrack] = useState<Track>({ key: rangeKey, view, enter: '', leaving: null });
  // The element rendered last, so a view change can keep rendering it while it fades out.
  const [last, setLast] = useState(children);
  // Derived state during render: the new layer must be committed with its animation class, or
  // the first frame would show it fully opaque before an effect could add the class.
  if (track.key !== rangeKey) {
    const viewChanged = track.view !== view;
    setTrack({
      key: rangeKey,
      view,
      enter: viewChanged ? 'fade' : navDir > 0 ? 'forward' : 'back',
      leaving: viewChanged ? { key: track.key, element: last } : null,
    });
  }
  if (last !== children) setLast(children);
  const leavingKey = track.leaving?.key;
  useEffect(() => {
    if (!leavingKey) return undefined;
    const t = window.setTimeout(() => setTrack((s) => (s.leaving?.key === leavingKey ? { ...s, leaving: null } : s)), ms(MOTION.view_fade_duration));
    return () => window.clearTimeout(t);
  }, [leavingKey]);
  return (
    <>
      {track.leaving ? (
        <div className="app-view-layer app-view-layer-leaving" key={track.leaving.key} aria-hidden="true">
          {track.leaving.element}
        </div>
      ) : null}
      <div className={`app-view-layer${track.enter ? ` app-view-layer-${track.enter}` : ''}`} key={track.key}>
        {children}
      </div>
    </>
  );
}
