import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from 'react';
import { format } from 'date-fns';
import { ipc } from '../ipc';
import { chipBackground, useTheme } from '../lib/colors';
import { hhmm, inZone } from '../lib/dates';
import { useUi } from '../state/ui';
import { LAYOUT, layoutNumber } from '../styles/layout';
import type { CalendarInfo, CalendarKey, EventDraft } from '../types/ipc';
import { columnRect, popupPosition } from './EventPopup';
import './QuickCreate.css';

// Component 15 of docs/04 section 4 (docs/design/measurements/quick_create-light.json). The
// title, Meet and calendar are set here; the other rows open the full form with the draft, as
// does "More options". Task, out of office and appointment tabs are out of scope (docs/01).

const UNAVAILABLE = 'Not available in this version';
const ICON = {
  people: 'M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5c-1.66 0-3 1.34-3 3s1.34 3 3 3zm-8 0c1.66 0 2.99-1.34 2.99-3S9.66 5 8 5C6.34 5 5 6.34 5 8s1.34 3 3 3zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5c0-2.33-4.67-3.5-7-3.5zm8 0c-.29 0-.62.02-.97.05 1.16.84 1.97 1.97 1.97 3.45V19h6v-2.5c0-2.33-4.67-3.5-7-3.5z',
  place: 'M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7zm0 9.5c-1.38 0-2.5-1.12-2.5-2.5s1.12-2.5 2.5-2.5 2.5 1.12 2.5 2.5-1.12 2.5-2.5 2.5z',
};

/** Calendars the user can write to, the default calendar first. */
export function writableCalendars(calendars: CalendarInfo[], defaultKey: CalendarKey | null): CalendarInfo[] {
  const list = calendars.filter((c) => c.access_role === 'owner' || c.access_role === 'writer').sort((a, b) => a.sort_order - b.sort_order);
  if (!defaultKey) return list;
  return [...list].sort((a, b) => Number(!(b.account_id === defaultKey.account_id && b.id === defaultKey.calendar_id)) - Number(!(a.account_id === defaultKey.account_id && a.id === defaultKey.calendar_id)));
}

export interface QuickCreateProps {
  startTs: number;
  endTs: number;
  allDay: boolean;
  anchor: DOMRect | null;
}

export function QuickCreate({ startTs, endTs, allDay, anchor }: QuickCreateProps) {
  const tz = useUi((s) => s.tz);
  const calendars = useUi((s) => s.calendars);
  const accounts = useUi((s) => s.accounts);
  const settings = useUi((s) => s.settings);
  const theme = useTheme();
  const writable = writableCalendars(calendars, settings?.default_calendar ?? null);
  const [calendarKey, setCalendarKey] = useState<CalendarKey | null>(writable[0] ? { account_id: writable[0].account_id, calendar_id: writable[0].id } : null);
  const calendar = writable.find((c) => calendarKey && c.account_id === calendarKey.account_id && c.id === calendarKey.calendar_id) ?? writable[0];
  const [title, setTitle] = useState('');
  const [addMeet, setAddMeet] = useState(false);
  const [pick, setPick] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);
  const close = () => useUi.getState().closeDialog();
  const accountName = (id: string) => accounts.find((a) => a.id === id)?.display_name ?? '';

  useLayoutEffect(() => {
    const el = rootRef.current;
    if (!el || !anchor) return;
    const r = el.getBoundingClientRect();
    setPos(popupPosition(columnRect(anchor), { width: window.innerWidth, height: window.innerHeight }, { width: layoutNumber(LAYOUT.qc_width), height: r.height }, anchor.top));
  }, [anchor]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    const onDown = (e: MouseEvent) => {
      if (e.target instanceof Element && !e.target.closest('.qc-root')) close();
    };
    document.addEventListener('keydown', onKey);
    document.addEventListener('mousedown', onDown);
    return () => {
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('mousedown', onDown);
    };
  }, []);

  const draft = (): EventDraft => ({
    account_id: calendar?.account_id ?? '',
    calendar_id: calendar?.id ?? '',
    title: title.trim() || null,
    description: null,
    location: null,
    all_day: allDay,
    start: allDay ? null : startTs,
    end: allDay ? null : endTs,
    start_date: allDay ? format(inZone(startTs, tz), 'yyyy-MM-dd') : null,
    end_date: allDay ? format(inZone(endTs, tz), 'yyyy-MM-dd') : null,
    time_zone: tz,
    recurrence: [],
    attendees: [],
    reminders: null,
    color_id: null,
    add_meet: addMeet,
    transparency: null,
    visibility: null,
  });
  const save = async () => {
    if (!calendar) return;
    setBusy(true);
    try {
      await ipc.createEvent(draft());
      close();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };
  const more = () => useUi.getState().openDialog({ kind: 'full-form', occurrenceId: null, startTs, endTs, allDay, draft: draft() });

  const dateLabel = format(inZone(startTs, tz), 'EEEE, MMMM d');
  const startLabel = hhmm(startTs, tz);
  const endLabel = hhmm(endTs, tz);
  const dotStyle = { '--data-chip-bg': chipBackground(calendar?.color_bg ?? '', theme) } as CSSProperties;
  const style = { left: pos ? `${pos.left}px` : 'var(--layout-qc-left)', top: pos ? `${pos.top}px` : 'var(--layout-qc-top)' } as CSSProperties;

  return (
    <div className="qc-root" role="dialog" ref={rootRef} style={style} onMouseDown={(e) => e.stopPropagation()}>
      <div className="qc-side-l"></div>
      <span className="qc-frame">
        <div className="qc-col">
          <div className="qc-body">
            <div className="qc-heading-sr" role="heading">{"Create"}</div>
            <span className="qc-close-wrap">
              <button className="qc-close" aria-label="Close" type="button" onClick={close}>
                <span className="qc-close-ripple"></span>
                <span className="qc-close-icon-box">
                  <i className="qc-close-icon">{"close"}</i>
                </span>
                <div className="qc-close-overlay"></div>
              </button>
              <div className="qc-close-tooltip" role="tooltip">{"Close"}</div>
            </span>
            <div className="qc-scroll">
              <div className="qc-scroll-inner">
                <div className="qc-n15">
                  <div className="qc-n16"></div>
                </div>
                <div className="qc-title-row">
                  <div className="qc-title-box">
                    <div className="qc-title-field">
                      <div className="qc-title-line">
                        <span className="qc-title-span">
                          <input className="qc-title-input" aria-label="Add title" placeholder="Add title" value={title} onChange={(e) => setTitle(e.target.value)} onKeyDown={(e) => { if (e.key === 'Enter') void save(); }} autoFocus />
                        </span>
                        <div className="qc-title-underline"></div>
                      </div>
                    </div>
                  </div>
                </div>
                <div className="qc-panel">
                  <div className="qc-tabs" role="tablist">
                    <button className="qc-tab-event" role="tab" type="button" aria-selected="true">
                      <span className="qc-tab-event-ripple"></span>
                      <div className="qc-tab-event-label">{"Event"}</div>
                    </button>
                    <button className="qc-tab-task" role="tab" type="button" aria-selected="false" disabled title={UNAVAILABLE}>
                      <span className="qc-tab-task-ripple"></span>
                      <div className="qc-tab-task-label">{"Task"}</div>
                    </button>
                    <button className="qc-tab-ooo" role="tab" type="button" aria-selected="false" disabled title={UNAVAILABLE}>
                      <span className="qc-tab-ooo-ripple"></span>
                      <div className="qc-tab-ooo-label">{"Out of office"}</div>
                    </button>
                    <button className="qc-tab-appt" role="tab" type="button" aria-selected="false" disabled title={UNAVAILABLE}>
                      <span className="qc-tab-appt-ripple"></span>
                      <div className="qc-tab-appt-label">
                        <span className="qc-tab-appt-text">{"Appointment schedule"}</span>
                        <span className="qc-tab-appt-new" aria-label=" new">{"new"}</span>
                      </div>
                    </button>
                  </div>
                  <div className="qc-tabpanel" role="tabpanel">
                    <span className="qc-tabpanel-span">
                      <div className="qc-form">
                        <div className="qc-when-block">
                          <div className="qc-when-box">
                            <div className="qc-when-n45">
                              <div className="qc-when-n46">
                                <div className="qc-when-row">
                                  <div className="qc-when-icon-cell">
                                    <i className="qc-when-icon">{"access_time"}</i>
                                  </div>
                                  <div className="qc-when-cell">
                                    <div className="qc-when-cell-inner">
                                      <button className="qc-when-button" type="button" onClick={more}>
                                        <span className="qc-when-ripple"></span>
                                        <div className="qc-when-content">
                                          <div className="qc-when-line">
                                            <div className="qc-when-date-group">
                                              <span className="qc-when-date" aria-label={` ${dateLabel} `}>
                                                <span className="qc-when-date-text">{dateLabel}</span>
                                              </span>
                                              <div className="qc-when-times">
                                                <span className="qc-when-start" aria-label={` ${startLabel} `}>
                                                  <span className="qc-when-start-text">{startLabel}</span>
                                                </span>
                                                <span className="qc-when-to" aria-label=" to ">{"–"}</span>
                                                <span className="qc-when-end" aria-label={` ${endLabel} `}>
                                                  <span className="qc-when-end-text">{endLabel}</span>
                                                </span>
                                              </div>
                                            </div>
                                          </div>
                                          <div className="qc-when-meta">
                                            <div className="qc-when-meta-inner">
                                              <ul className="qc-when-meta-list">
                                                <li className="qc-when-tz">{"Time zone"}</li>
                                                <li className="qc-when-repeat">{"Does not repeat"}</li>
                                              </ul>
                                            </div>
                                          </div>
                                        </div>
                                      </button>
                                    </div>
                                  </div>
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                        <div className="qc-spacer"></div>
                        <div className="qc-guests-block">
                          <div className="qc-guests-box">
                            <div className="qc-guests-row">
                              <div className="qc-guests-icon-cell">
                                <span className="qc-guests-icon-span">
                                  <svg className="qc-guests-icon" viewBox="0 0 24 24" focusable="false">
                                    <path className="qc-guests-icon-path" d={ICON.people} />
                                  </svg>
                                </span>
                              </div>
                              <div className="qc-guests-cell">
                                <div className="qc-guests-cell-inner">
                                  <button className="qc-guests-button" type="button" onClick={more}>
                                    <span className="qc-guests-ripple"></span>
                                    <div className="qc-guests-label">{"Add guests"}</div>
                                  </button>
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                        <div className="qc-meet-block">
                          <div className="qc-meet-row">
                            <div className="qc-meet-icon-cell">
                              <div className="qc-meet-icon-box">
                                <i className="qc-meet-icon">videocam</i>
                              </div>
                            </div>
                            <div className="qc-meet-cell">
                              <div className="qc-meet-n89">
                                <div className="qc-meet-n90">
                                  <div className="qc-meet-n91">
                                    <div className="qc-meet-n92">
                                      <div className="qc-meet-n93">
                                        <button className="qc-meet-button" type="button" aria-pressed={addMeet} onClick={() => setAddMeet(!addMeet)}>
                                          <span className="qc-meet-ripple"></span>
                                          <span className="qc-meet-label">{addMeet ? 'Google Meet video conferencing added' : 'Add Google Meet video conferencing'}</span>
                                        </button>
                                      </div>
                                    </div>
                                    <div className="qc-meet-foot"></div>
                                  </div>
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                        <div className="qc-location-block">
                          <div className="qc-location-box">
                            <div className="qc-location-row">
                              <div className="qc-location-icon-cell">
                                <span className="qc-location-icon-span">
                                  <svg className="qc-location-icon">
                                    <path className="qc-location-icon-path" />
                                    <circle className="qc-location-icon-path2" />
                                  </svg>
                                </span>
                              </div>
                              <div className="qc-location-cell">
                                <div className="qc-location-cell-inner">
                                  <button className="qc-location-button" type="button" onClick={more}>
                                    <span className="qc-location-ripple"></span>
                                    <div className="qc-location-label">{"Add "}
                                      <span className="qc-location-word">{"location"}</span>
                                    </div>
                                  </button>
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                        <div className="qc-description-block">
                          <div className="qc-description-box">
                            <div className="qc-description-row">
                              <div className="qc-description-icon-cell">
                                <i className="qc-description-icon">{"notes"}</i>
                              </div>
                              <div className="qc-description-cell">
                                <div className="qc-description-cell-inner">
                                  <button className="qc-description-button" type="button" onClick={more}>
                                    <span className="qc-description-ripple"></span>
                                    <div className="qc-description-label">
                                      <span className="qc-description-text">
                                        {"Add "}
                                        <span className="qc-description-word">{"description"}</span>
                                        {" or "}
                                        <span className="qc-description-word2">{"a Google Drive attachment"}</span>
                                      </span>
                                    </div>
                                  </button>
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                        <div className="qc-calendar-block">
                          <div className="qc-calendar-box">
                            <div className="qc-calendar-row">
                              <div className="qc-calendar-icon-cell">
                                <i className="qc-calendar-icon">{"event"}</i>
                              </div>
                              <div className="qc-calendar-cell">
                                <div className="qc-calendar-cell-inner">
                                  <button className="qc-calendar-button" type="button" aria-haspopup="menu" aria-expanded={pick} onClick={() => setPick(!pick)}>
                                    <span className="qc-calendar-ripple"></span>
                                    <div className="qc-calendar-content">
                                      <div className="qc-calendar-line">
                                        <div className="qc-calendar-name-row">
                                          <div className="qc-calendar-name-box">
                                            <span className="qc-calendar-name">{calendar?.summary ?? 'No calendar'}</span>
                                          </div>
                                          <div className="qc-calendar-dot" style={dotStyle}></div>
                                        </div>
                                      </div>
                                      <div className="qc-calendar-meta">
                                        <ul className="qc-calendar-meta-list">
                                          <li className="qc-calendar-busy">{"Busy"}</li>
                                          <li className="qc-calendar-visibility">{"Default visibility"}</li>
                                          <li className="qc-calendar-notify">{"Do not notify"}</li>
                                        </ul>
                                      </div>
                                    </div>
                                  </button>
                                    {pick ? (
                                      <ul className="qc-calendar-menu create-menu-root" role="menu">
                                        {writable.map((c) => (
                                          <li className="create-menu-item" role="menuitem" key={`${c.account_id}/${c.id}`} tabIndex={0} onClick={() => { setCalendarKey({ account_id: c.account_id, calendar_id: c.id }); setPick(false); }}>
                                            <span className="create-menu-item-ripple"></span>
                                            <span className="create-menu-item-box">
                                              <span className="create-menu-item-label">{`${c.summary} · ${accountName(c.account_id)}`}</span>
                                            </span>
                                          </li>
                                        ))}
                                      </ul>
                                    ) : null}
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                        <div className="qc-panel-foot">
                          <div className="qc-panel-foot-bar">
                            <div className="qc-panel-foot-inner"></div>
                          </div>
                        </div>
                      </div>
                    </span>
                  </div>
                </div>
              </div>
              {error ? <div className="qc-error" role="alert">{error}</div> : null}
            <div className="qc-footer">
                <div className="qc-footer-inner">
                  <div className="qc-more-wrap">
                    <button className="qc-more" type="button" onClick={more}>
                      <span className="qc-more-ripple"></span>
                      <span className="qc-more-hit"></span>
                      <span className="qc-more-label">{"More options"}</span>
                    </button>
                  </div>
                  <div className="qc-save-n155"></div>
                  <div className="qc-save-n156"></div>
                  <div className="qc-save-wrap">
                    <button className="qc-save" type="button" onClick={() => void save()} disabled={!calendar || busy}>
                      <span className="qc-save-ripple">
                        <span className="qc-save-ripple-inner"></span>
                      </span>
                      <span className="qc-save-n161"></span>
                      <span className="qc-save-hit"></span>
                      <span className="qc-save-label">{"Save"}</span>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <div className="qc-head"></div>
          <span className="qc-dock-span">
            <button className="qc-dock" aria-label="Dock to sidebar" type="button" disabled title={UNAVAILABLE}>
              <span className="qc-dock-ripple"></span>
              <span className="qc-dock-icon-box">
                <i className="qc-dock-icon">{"drag_handle"}</i>
              </span>
              <div className="qc-dock-overlay"></div>
            </button>
            <div className="qc-dock-tooltip" role="tooltip">{"Dock to sidebar"}</div>
          </span>
          <span className="qc-undock-span">
            <div className="qc-undock-tooltip" role="tooltip">{"Undock sidebar"}</div>
          </span>
        </div>
      </span>
      <div className="qc-side-r"></div>
    </div>
  );
}
