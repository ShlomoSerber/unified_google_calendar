import { useEffect, useRef, useState, type CSSProperties } from 'react';
import { useUi, type ViewKind } from '../state/ui';
import { LAYOUT, layoutNumber } from '../styles/legacy-layout';
import './ViewSelector.css';

// Component 17 of docs/04 section 4 (docs/design/measurements/view_selector-light.json): the
// menu under the view button.

// Only the views that exist: no "4 days", no shortcut letters (keyboard shortcuts are version
// 2) and no toggles (docs/99, 2026-09-15).
const VIEWS: { label: string; view: ViewKind }[] = [
  { label: 'Day', view: 'day' },
  { label: 'Week', view: 'week' },
  { label: 'Month', view: 'month' },
  { label: 'Year', view: 'year' },
  { label: 'Schedule', view: 'agenda' },
];

export interface ViewSelectorProps {
  /** Closing: keep rendering with the exit animation (App.tsx presence). */
  exiting?: boolean;
}

export function ViewSelector({ exiting = false }: ViewSelectorProps) {
  const rootRef = useRef<HTMLDivElement>(null);
  // Under the view button, left edges aligned (tokens.json layout.vsel_gap_note). The button is
  // already laid out when the menu mounts, so its rect is read once during the first render.
  const [pos] = useState<{ left: number; top: number } | null>(() => {
    const r = document.querySelector('.topbar-view-button')?.getBoundingClientRect();
    return r ? { left: r.left, top: r.bottom + layoutNumber(LAYOUT.vsel_gap) } : null;
  });
  const close = () => useUi.getState().closeDialog();
  const style = pos ? ({ left: `${pos.left}px`, top: `${pos.top}px` } as CSSProperties) : undefined;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    const onDown = (e: MouseEvent) => {
      if (e.target instanceof Element && !e.target.closest('.vsel-root, .topbar-view-button')) close();
    };
    document.addEventListener('keydown', onKey);
    document.addEventListener('mousedown', onDown);
    return () => {
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('mousedown', onDown);
    };
  }, []);
  return (
    <div className={exiting ? 'vsel-root motion-menu-exit' : 'vsel-root motion-menu'} ref={rootRef} style={style}>
      <span className="vsel-scrim">
        <span className="vsel-scrim-inner"></span>
      </span>
      <div className="vsel-body">
        <ul className="vsel-menu" role="menu">
          <div className="vsel-n5"></div>
          <div className="vsel-n6"></div>
          {VIEWS.map((v) => (
            <li className="vsel-item" role="menuitem" key={v.label} tabIndex={0} onClick={() => { useUi.getState().setView(v.view); close(); }}>
              <span className="vsel-item-ripple ugc-state"></span>
              <span className="vsel-item-box">
                <span className="vsel-item-label">{v.label}</span>
              </span>
            </li>
          ))}
          <div className="vsel-foot1"></div>
          <div className="vsel-foot2"></div>
        </ul>
      </div>
    </div>
  );
}
