import { useEffect } from 'react';
import { dayStart } from '../lib/dates';
import { useUi } from '../state/ui';

// The menu of component 3 (docs/design/measurements/create_button-open-light.json). Only
// "Event" exists in version 1 (docs/01 section 3: tasks, out of office and appointment schedules
// are out of scope); the other items keep the measured layout and say so.
const ITEMS = ['Event', 'Task', 'Out of office'] as const;
const UNAVAILABLE = 'Not available in this version';

export function CreateMenu() {
  const date = useUi((s) => s.date);
  const tz = useUi((s) => s.tz);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') useUi.getState().closeDialog();
    };
    const onClick = (e: MouseEvent) => {
      if (!(e.target instanceof Element) || !e.target.closest('.create-menu-root, .create-button-root')) useUi.getState().closeDialog();
    };
    document.addEventListener('keydown', onKey);
    document.addEventListener('mousedown', onClick);
    return () => {
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('mousedown', onClick);
    };
  }, []);
  const createEvent = () => {
    // Same default as Google's menu: the next full hour of the anchor day, one hour long.
    const now = Math.floor(Date.now() / 1000);
    const start = dayStart(date, tz) + (Math.floor((now - dayStart(now, tz)) / 3600) + 1) * 3600;
    useUi.getState().openDialog({ kind: 'full-form', occurrenceId: null, startTs: start, endTs: start + 3600, allDay: false });
  };
  return (
    <div className="create-menu-root">
      <div className="create-menu-n1"></div>
      <div className="create-menu-n2"></div>
      <span className="create-menu-scrim">
        <span className="create-menu-scrim-inner"></span>
      </span>
      <div className="create-menu-body">
        <ul className="create-menu-list" role="menu">
          {ITEMS.map((label) => {
            const enabled = label === 'Event';
            return (
              <li
                className="create-menu-item"
                role="menuitem"
                key={label}
                tabIndex={enabled ? 0 : -1}
                aria-disabled={!enabled}
                title={enabled ? undefined : UNAVAILABLE}
                onClick={enabled ? createEvent : undefined}
              >
                <span className="create-menu-item-ripple"></span>
                <span className="create-menu-item-box">
                  <span className="create-menu-item-label">{label}</span>
                </span>
              </li>
            );
          })}
          <li className="create-menu-item-last" role="menuitem" aria-label="Appointment schedule" aria-disabled="true" title={UNAVAILABLE}>
            <div className="create-menu-item-last-box">
              <div className="create-menu-item-last-label">{'Appointment schedule'}</div>
            </div>
            <span className="create-menu-item-last-ripple"></span>
          </li>
        </ul>
      </div>
      <div className="create-menu-n23"></div>
      <div className="create-menu-n24"></div>
    </div>
  );
}
