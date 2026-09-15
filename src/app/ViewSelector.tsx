import { useEffect, useRef } from 'react';
import { useUi, type ViewKind } from '../state/ui';
import './ViewSelector.css';

// Component 17 of docs/04 section 4 (docs/design/measurements/view_selector-light.json): the
// menu under the Week button. Year and 4 days views, and the three toggles, are outside
// version 1 and stay inert; keyboard shortcuts are version 2 (the letters are Google's).

const UNAVAILABLE = 'Not available in this version';
const CHECK = 'M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z';
const VIEWS: { label: string; key: string; view: ViewKind | null }[] = [
  { label: 'Day', key: 'D', view: 'day' },
  { label: 'Week', key: 'W', view: 'week' },
  { label: 'Month', key: 'M', view: 'month' },
  { label: 'Year', key: 'Y', view: null },
  { label: 'Schedule', key: 'A', view: 'agenda' },
  { label: '4 days', key: 'X', view: null },
];
const CHECKS = [
  { label: 'Show weekends', checked: true },
  { label: 'Show declined events', checked: true },
  { label: 'Show completed tasks', checked: true },
];

export function ViewSelector() {
  const rootRef = useRef<HTMLDivElement>(null);
  const close = () => useUi.getState().closeDialog();
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
    <div className="vsel-root" ref={rootRef}>
      <span className="vsel-scrim">
        <span className="vsel-scrim-inner"></span>
      </span>
      <div className="vsel-body">
        <ul className="vsel-menu" role="menu">
          <div className="vsel-n5"></div>
          <div className="vsel-n6"></div>
          {VIEWS.map((v) => (
            <li className="vsel-item" role="menuitem" key={v.label} tabIndex={0} aria-disabled={!v.view} title={v.view ? undefined : UNAVAILABLE} onClick={() => { if (v.view) { useUi.getState().setView(v.view); close(); } }}>
              <span className="vsel-item-ripple"></span>
              <span className="vsel-item-box">
                <span className="vsel-item-label">{v.label}</span>
              </span>
              <span className="vsel-item-key">{v.key}</span>
            </li>
          ))}
          <li className="vsel-separator" role="separator"></li>
          <li className="vsel-group-li" role="none">
            <ul className="vsel-group" role="group" aria-label="View options">
              {CHECKS.map((c) => (
                <li className="vsel-check" role="menuitemcheckbox" aria-checked={c.checked} aria-disabled="true" title={UNAVAILABLE} key={c.label}>
                  <span className="vsel-check-ripple"></span>
                  <span className="vsel-check-icon-cell">
                    <span className="vsel-check-icon-wrap">
                      <span className="vsel-check-icon-box">
                        <svg className="vsel-check-icon" viewBox="0 0 24 24" focusable="false">
                          <path className="vsel-check-path" d={c.checked ? CHECK : ''} />
                        </svg>
                      </span>
                    </span>
                  </span>
                  <span className="vsel-check-box">
                    <span className="vsel-check-label">{c.label}</span>
                  </span>
                </li>
              ))}
            </ul>
          </li>
          <div className="vsel-foot1"></div>
          <div className="vsel-foot2"></div>
        </ul>
      </div>
    </div>
  );
}
