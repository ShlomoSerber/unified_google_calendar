import { useState, type CSSProperties, type ReactNode } from 'react';
import { ipc } from '../ipc';
import { chipBackground, useTheme } from '../lib/colors';
import { useUi } from '../state/ui';
import type { AccountInfo, CalendarInfo } from '../types/ipc';

// Component 5 of docs/04 section 4, grouped by account (docs/08 F4-T3). The DOM mirrors
// nodes 282-401 of docs/design/measurements/sidebar-light.json: every Google account gets a
// "My calendars"-style section titled with the account; iCal calendars share the
// "Other calendars" section, whose "+" opens the settings to add one (docs/99 entry F4-T3).

// Paths read from the SVGs calendar.google.com inlines (probed 2026-09-14): the checkbox tick is
// a stroked polyline (Material Components checkbox), the "+" is Material Design "add" (Apache 2.0).
const CHECK = 'M1.73,12.91 8.1,19.28 22.79,4.59';
const ADD_PATH = 'M20 13h-7v7h-2v-7H4v-2h7V4h2v7h7v2z';

interface RowProps {
  calendar: CalendarInfo;
  label: string;
  typeLabel: string | null;
}

function CalendarRow({ calendar, label, typeLabel }: RowProps) {
  const theme = useTheme();
  const on = calendar.visible;
  const suffix = on ? '-on' : '';
  const color = { '--data-calendar-color': chipBackground(calendar.color_bg, theme) } as CSSProperties;
  const toggle = () => {
    const next = !calendar.visible;
    const { calendars, setCalendars } = useUi.getState();
    setCalendars(calendars.map((c) => (c.account_id === calendar.account_id && c.id === calendar.id ? { ...c, visible: next } : c)));
    ipc.setCalendarVisible(calendar.account_id, calendar.id, next).catch(() => {
      const state = useUi.getState();
      state.setCalendars(state.calendars.map((c) => (c.account_id === calendar.account_id && c.id === calendar.id ? { ...c, visible: !next } : c)));
    });
  };
  return (
    <div className="sidebar-list-row-wrap" role="presentation">
      <li className="sidebar-list-row" role="listitem">
        <div className="sidebar-list-row-inner">
          <div className="sidebar-list-check-area">
            <div className="sidebar-list-check-box">
              <div className="sidebar-list-check" style={color}>
                <input className="sidebar-list-check-input" type="checkbox" aria-label={label} checked={on} onChange={toggle} />
                <div className={`sidebar-list-check-mark${suffix}`}>
                  <svg className={`sidebar-list-check-svg${suffix}`} viewBox="0 0 24 24" focusable="false">
                    <path className={`sidebar-list-check-path${suffix}`} d={CHECK} fill="none" />
                  </svg>
                  <div className="sidebar-list-check-mixed"></div>
                </div>
                <span className="sidebar-list-check-ripple ugc-state ugc-state-calendar"></span>
              </div>
            </div>
          </div>
          <div className="sidebar-list-label-box">
            <span className="sidebar-list-label">{label}</span>
            {typeLabel ? <span className="sidebar-list-type">{typeLabel}</span> : null}
          </div>
        </div>
      </li>
    </div>
  );
}

interface SectionProps {
  id: string;
  title: string;
  avatar: string | null;
  tooltip: string | null;
  addButton: boolean;
  children: ReactNode;
}

function Section({ id, title, avatar, tooltip, addButton, children }: SectionProps) {
  const [open, setOpen] = useState(true);
  const openAddCalendar = () => useUi.getState().openDialog({ kind: 'add-calendar' });
  const n = addButton ? '2' : '';
  const header = (
    <button className={`sidebar-list-header${n}`} type="button" aria-expanded={open} onClick={() => setOpen(!open)} title={tooltip ?? undefined}>
      <span className={`sidebar-list-header${n}-ripple ugc-state ugc-state-primary`}></span>
      <div className={`sidebar-list-header${n}-box`}>
        <div className={`sidebar-list-header${n}-row`}>
          {avatar ? (
            <span className="sidebar-list-avatar" aria-hidden="true">
              {avatar}
            </span>
          ) : null}
          <div className={`sidebar-list-header${n}-label`}>{title}</div>
          <i className={`sidebar-list-header${n}-arrow`}>{open ? 'keyboard_arrow_up' : 'keyboard_arrow_down'}</i>
        </div>
      </div>
    </button>
  );
  return (
    <>
      {addButton ? (
        <div className="sidebar-list-header2-wrap">
          {header}
          <div className="sidebar-list-add-wrap">
            <div className="sidebar-list-add-box">
              <div className="sidebar-list-add-inner">
                <span className="sidebar-list-add-span">
                  <button className="sidebar-list-add" aria-label="Add other calendars" type="button" onClick={openAddCalendar} data-tooltip="Add other calendars">
                    <span className="sidebar-list-add-ripple ugc-state ugc-state-icon"></span>
                    <span className="sidebar-list-add-icon-box">
                      <svg className="sidebar-list-add-icon" viewBox="0 0 24 24" focusable="false">
                        <path className="sidebar-list-add-path" d={ADD_PATH} />
                      </svg>
                    </span>
                    <div className="sidebar-list-add-overlay"></div>
                  </button>
                </span>
                <div className="sidebar-list-add-foot"></div>
              </div>
            </div>
          </div>
        </div>
      ) : (
        header
      )}
      {open ? (
        <div className="sidebar-list-body">
          <div className="sidebar-list" role="list" aria-label={title} data-section={id}>
            {children}
          </div>
        </div>
      ) : null}
    </>
  );
}

const byOrder = <T extends { sort_order: number }>(a: T, b: T) => a.sort_order - b.sort_order;

export function accountTooltip(a: AccountInfo): string {
  if (a.kind === 'google') return a.email ? `Google account · ${a.email}` : 'Google account';
  return 'iCal calendar · read-only';
}

export function CalendarList() {
  const accounts = useUi((s) => s.accounts);
  const calendars = useUi((s) => s.calendars);
  const google = accounts.filter((a) => a.kind === 'google').sort(byOrder);
  const others = accounts.filter((a) => a.kind !== 'google').sort(byOrder);
  const calendarsOf = (a: AccountInfo) => calendars.filter((c) => c.account_id === a.id && !c.hidden_remote).sort(byOrder);

  return (
    <div className="sidebar-lists">
      <h2 className="sidebar-lists-sr">Calendar list</h2>
      <div className="sidebar-lists-box">
        <div className="sidebar-lists-inner">
          <div className="sidebar-list-panel" role="complementary">
            <div className="sidebar-list-wrap">
              {google.map((a, i) => (
                <div key={a.id}>
                  {i > 0 ? <div className="sidebar-list-gap"></div> : null}
                  <Section id={a.id} title={a.display_name} avatar={(a.email ?? a.display_name).charAt(0).toUpperCase()} tooltip={accountTooltip(a)} addButton={false}>
                    {calendarsOf(a).map((c) => (
                      <CalendarRow key={c.id} calendar={c} label={c.summary} typeLabel={null} />
                    ))}
                  </Section>
                </div>
              ))}
              {google.length > 0 ? <div className="sidebar-list-gap"></div> : null}
              <Section id="other" title="Other calendars" avatar={null} tooltip={null} addButton={true}>
                {others.flatMap((a) =>
                  calendarsOf(a).map((c) => <CalendarRow key={`${a.id}/${c.id}`} calendar={c} label={c.summary} typeLabel="iCal" />),
                )}
              </Section>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
