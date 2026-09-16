import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from 'react';
import { format } from 'date-fns';
import { ipc } from '../ipc';
import { withNotice } from '../lib/notice';
import { chipBackground, useTheme } from '../lib/colors';
import { hhmm, inZone } from '../lib/dates';
import { useUi } from '../state/ui';
import { LAYOUT, layoutNumber } from '../styles/legacy-layout';
import type { AttendeeInfo, EventDetail } from '../types/ipc';
import './EventPopup.css';

// Component 14 of docs/04 section 4 (docs/design/measurements/event_popup-*-light.json). Rows
// appear in Google's order: header actions, title/date/recurrence, Meet, meeting notes (inert,
// Workspace), guests, location, description, calendar/organizer. Positioned by JS following
// the measured anchor rule (docs/04 section 8, tokens.json layout.popup_anchor_note).

// Material Design paths (Apache 2.0) read from the SVGs Google inlines in the popup.
const ICON = {
  close: 'M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z',
  edit: 'M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z',
  delete: 'M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z',
  more: 'M12 8c1.1 0 2-.9 2-2s-.9-2-2-2-2 .9-2 2 .9 2 2 2zm0 2c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm0 6c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2z',
  people: 'M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5c-1.66 0-3 1.34-3 3s1.34 3 3 3zm-8 0c1.66 0 2.99-1.34 2.99-3S9.66 5 8 5C6.34 5 5 6.34 5 8s1.34 3 3 3zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5c0-2.33-4.67-3.5-7-3.5zm8 0c-.29 0-.62.02-.97.05 1.16.84 1.97 1.97 1.97 3.45V19h6v-2.5c0-2.33-4.67-3.5-7-3.5z',
  place: 'M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7zm0 9.5c-1.38 0-2.5-1.12-2.5-2.5s1.12-2.5 2.5-2.5 2.5 1.12 2.5 2.5-1.12 2.5-2.5 2.5z',
  check: 'M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z',
  question: 'M11 18h2v-2h-2v2zm1-16C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm0-14c-2.21 0-4 1.79-4 4h2c0-1.1.9-2 2-2s2 .9 2 2c0 2-3 1.75-3 5h2c0-2.25 3-2.5 3-5 0-2.21-1.79-4-4-4z',
  phone: 'M6.62 10.79c1.44 2.83 3.76 5.14 6.59 6.59l2.2-2.2c.27-.27.67-.36 1.02-.24 1.12.37 2.33.57 3.57.57.55 0 1 .45 1 1V20c0 .55-.45 1-1 1-9.39 0-17-7.61-17-17 0-.55.45-1 1-1h3.5c.55 0 1 .45 1 1 0 1.25.2 2.45.57 3.57.11.35.03.74-.25 1.02l-2.2 2.2z',
  person: 'M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z',
};


/** Where a popup goes for the day column of its chip or slot (tokens.json layout.popup_anchor_note):
 *  right of the column when it fits, else left of it. `slotTop` (quick create) pushes the dialog up
 *  so its bottom meets a slot that starts below the centred position; the event popup is centred. */
export function popupPosition(
  column: { left: number; right: number },
  viewport: { width: number; height: number },
  size: { width: number; height: number },
  slotTop?: number,
): { left: number; top: number; side: 'right' | 'left' } {
  const gap = layoutNumber(LAYOUT.popup_gap);
  const gapRight = layoutNumber(LAYOUT.popup_gap_right);
  let left = column.right + gapRight;
  let side: 'right' | 'left' = 'right';
  if (left + size.width > viewport.width) {
    left = column.left - gap - size.width;
    side = 'left';
  }
  const centred = (viewport.height - size.height) / 2;
  let top = slotTop !== undefined && slotTop - size.height - 1 > centred ? slotTop - size.height - 1 : centred;
  top = Math.max(0, Math.min(top, viewport.height - size.height));
  return { left, top, side };
}

/** "2 yes", "1 awaiting", "1 no", "1 maybe" summary lines. */
export function attendeeSummary(attendees: AttendeeInfo[]): string[] {
  const count = (s: string) => attendees.filter((a) => a.response_status === s).length;
  const lines: string[] = [];
  if (count('accepted')) lines.push(`${count('accepted')} yes`);
  if (count('declined')) lines.push(`${count('declined')} no`);
  if (count('tentative')) lines.push(`${count('tentative')} maybe`);
  if (count('needsAction')) lines.push(`${count('needsAction')} awaiting`);
  return lines;
}

function IconButton({ prefix, label, path, onClick, disabled, title }: { prefix: string; label: string; path: string; onClick?: () => void; disabled?: boolean; title?: string }) {
  return (
    <div className={`popup-${prefix}-cell`}>
      <span className={`popup-${prefix}-span`}>
        <button className={`popup-${prefix}`} aria-label={label} type="button" onClick={onClick} disabled={disabled} title={title} data-tooltip={label}>
          <span className={`popup-${prefix}-ripple`}></span>
          <span className={`popup-${prefix}-icon-box`}>
            <svg className={`popup-${prefix}-icon`} viewBox="0 0 24 24" focusable="false">
              <path className={`popup-${prefix}-path`} d={path} />
            </svg>
          </span>
          <div className={`popup-${prefix}-overlay`}></div>
        </button>
      </span>
    </div>
  );
}

/** The day column that contains an anchor rect (the chip's `.week-daycol`, or the rect itself). */
export function columnRect(anchor: DOMRect): { left: number; right: number } {
  const x = anchor.left + anchor.width / 2;
  const y = anchor.top + anchor.height / 2;
  const col = document.elementsFromPoint(x, y).find((e) => e.classList.contains('week-daycol') || e.classList.contains('month-cell') || e.classList.contains('allday-cell'));
  if (col) {
    const r = col.getBoundingClientRect();
    return { left: r.left, right: r.right };
  }
  return { left: anchor.left, right: anchor.right };
}

export interface EventPopupProps {
  occurrenceId: string;
  anchor: DOMRect | null;
  /** Closing: keep rendering with the exit animation (App.tsx presence). */
  exiting?: boolean;
}

export function EventPopup({ occurrenceId, anchor, exiting = false }: EventPopupProps) {
  const tz = useUi((s) => s.tz);
  const accounts = useUi((s) => s.accounts);
  const theme = useTheme();
  const [detail, setDetail] = useState<EventDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [menu, setMenu] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ left: number; top: number; side: 'right' | 'left' } | null>(null);
  const close = () => useUi.getState().closeDialog();
  // Motion (docs/04 section 10): slides away from the chip once the content is there; fades out.
  const motion = exiting ? ' motion-popup-exit' : detail && pos ? ` motion-popup-${pos.side}` : '';

  useEffect(() => {
    let cancelled = false;
    ipc
      .getEvent(occurrenceId)
      .then((d) => {
        if (!cancelled) setDetail(d);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [occurrenceId]);

  useLayoutEffect(() => {
    const el = rootRef.current;
    if (!el || !anchor) return;
    const column = columnRect(anchor);
    setPos(popupPosition(column, { width: window.innerWidth, height: window.innerHeight }, { width: layoutNumber(LAYOUT.popup_width), height: el.getBoundingClientRect().height }));
  }, [anchor, detail]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && useUi.getState().overlay.kind === 'none') close();
    };
    const onDown = (e: MouseEvent) => {
      if (e.target instanceof Element && !e.target.closest('.popup-root, .scope-scrim-page')) close();
    };
    document.addEventListener('keydown', onKey);
    document.addEventListener('mousedown', onDown);
    return () => {
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('mousedown', onDown);
    };
  }, []);

  const style = {
    left: pos ? `${pos.left}px` : `var(--layout-popup-left)`,
    top: pos ? `${pos.top}px` : `var(--layout-popup-top)`,
  } as CSSProperties;

  if (error) {
    return (
      <div className={`popup-root popup-error${motion}`} role="dialog" ref={rootRef} style={style}>
        {`The event could not be loaded: ${error}`}
      </div>
    );
  }
  if (!detail) return <div className="popup-root" role="dialog" ref={rootRef} style={style}></div>;

  const d = detail;
  const start = inZone(d.start, tz);
  const dateLabel = format(start, 'EEEE, MMMM d');
  const timeLabel = d.all_day ? null : `${hhmm(d.start, tz)} – ${hhmm(d.end, tz)}`;
  const otherAccounts = accounts.filter((a) => a.kind === 'google' && a.id !== d.account_id);
  const edit = () => useUi.getState().openDialog({ kind: 'full-form', occurrenceId: d.occurrence_id, startTs: d.start, endTs: d.end, allDay: d.all_day });
  const remove = () => {
    if (d.is_recurring) {
      useUi.getState().openOverlay({ kind: 'edit-scope', occurrenceId: d.occurrence_id, action: 'delete' });
      return;
    }
    withNotice('Deleting...', 'Event deleted', () => ipc.deleteEvent(d.occurrence_id, 'this')).then(close).catch((e: unknown) => setError(String(e)));
  };
  const openMaps = () => ipc.openUrl(`https://www.google.com/maps/search/?api=1&query=${encodeURIComponent(d.location ?? '')}`).catch(() => undefined);
  const dotStyle = { '--data-chip-bg': chipBackground(d.color_bg, theme) } as CSSProperties;
  const meetCode = (d.meet_link ?? '').replace(/^https?:\/\/[^/]+\//, '');
  // Google resolves attendee names from contacts; the app knows its own accounts' names.
  const nameOf = (a: AttendeeInfo) => a.display_name ?? accounts.find((x) => x.email?.toLowerCase() === a.email.toLowerCase())?.display_name ?? a.email;
  const order = (a: AttendeeInfo) => (a.organizer ? 0 : a.response_status === 'accepted' ? 1 : a.response_status === 'tentative' ? 2 : a.response_status === 'declined' ? 3 : 4);
  const attendees = [...d.attendees].sort((x, y) => order(x) - order(y) || nameOf(x).localeCompare(nameOf(y)));
  const summary = attendeeSummary(d.attendees);
  const organizerEmail = d.organizer?.email ?? '';
  const creator = d.organizer?.display_name ?? (organizerEmail.endsWith('@group.calendar.google.com') || !organizerEmail ? d.account_name : organizerEmail);

  return (
    <div className={`popup-root${motion}`} role="dialog" ref={rootRef} style={style} onMouseDown={(e) => e.stopPropagation()}>
      <div className="popup-side-l1"></div>
      <div className="popup-side-l2"></div>
      <span className="popup-frame">
        <div className="popup-col">
          <div className="popup-col2">
            <div className="popup-header">
              <div className="popup-header-inner">
                <div className="popup-header-row">
                  <IconButton prefix="close" label="Close" path={ICON.close} onClick={close} />
                  <div className="popup-actions">
                    <IconButton prefix="edit" label="Edit event" path={ICON.edit} onClick={edit} disabled={!d.can_edit} title={d.can_edit ? undefined : 'This calendar is read-only'} />
                    <IconButton prefix="delete" label="Delete event" path={ICON.delete} onClick={remove} disabled={!d.can_delete} title={d.can_delete ? undefined : 'This calendar is read-only'} />
                    <div className="popup-options-cell">
                      <div className="popup-options-n50">
                        <div className="popup-options-n51">
                          <div className="popup-options-n52">
                            <span className="popup-options-span">
                              <button className="popup-options" aria-label="Options" type="button" aria-haspopup="menu" aria-expanded={menu} onClick={() => setMenu(!menu)} disabled={otherAccounts.length === 0} title={otherAccounts.length === 0 ? 'Add another Google account to move events between accounts' : undefined} data-tooltip="Options">
                                <span className="popup-options-ripple"></span>
                                <span className="popup-options-icon-box">
                                  <span className="popup-options-icon-span">
                                    <svg className="popup-options-icon" viewBox="0 0 24 24" focusable="false">
                                      <path className="popup-options-path" d={ICON.more} />
                                    </svg>
                                  </span>
                                </span>
                                <div className="popup-options-overlay"></div>
                              </button>
                            </span>
                            <div className="popup-options-foot"></div>
                          </div>
                        </div>
                      </div>
                      {menu ? (
                        <ul className="popup-options-menu create-menu-root motion-menu" role="menu">
                          {otherAccounts.map((a) => (
                            <li
                              className="create-menu-item"
                              role="menuitem"
                              key={a.id}
                              tabIndex={0}
                              onClick={() => {
                                setMenu(false);
                                ipc.moveEventAccount(d.occurrence_id, a.id, 'primary').then(setDetail).catch((e: unknown) => setError(String(e)));
                              }}
                            >
                              <span className="create-menu-item-ripple ugc-state"></span>
                              <span className="create-menu-item-box">
                                <span className="create-menu-item-label">{`Move to ${a.display_name}`}</span>
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
            <div className="popup-header-bottom"></div>
            <div className="popup-body">
              <div className="popup-title-row">
                <div className="popup-title-icon-cell">
                  <div className="popup-title-dot" style={dotStyle}></div>
                </div>
                <div className="popup-title-box">
                  <div className="popup-title-inner">
                    <div className="popup-title-line">
                      <span className="popup-title" role="heading" aria-level={2}>
                        {d.title ?? '(No title)'}
                      </span>
                    </div>
                    <div className="popup-date-line">
                      {timeLabel ? `${dateLabel} ` : dateLabel}
                      {timeLabel ? (
                        <>
                          <span className="popup-date-sep">{'⋅'}</span>
                          {' '}
                          <span className="popup-date-time">{timeLabel}</span>
                        </>
                      ) : null}
                    </div>
                    {d.recurrence_text ? <div className="popup-recurrence-line">{d.recurrence_text}</div> : null}
                  </div>
                </div>
                <div className="popup-title-end"></div>
              </div>
              {d.meet_link ? (
                <div className="popup-meet-block">
                  <div className="popup-meet-row">
                    <div className="popup-meet-icon-cell">
                      <i className="popup-meet-icon">videocam</i>
                    </div>
                    <div className="popup-meet-box">
                      <div className="popup-meet-inner">
                        <div className="popup-meet-link-line">
                          <span className="popup-meet-link-span">
                            <a className="popup-meet-link" href={d.meet_link} aria-label={`Join with Google Meet (${meetCode})`} onClick={(e) => { e.preventDefault(); ipc.openUrl(d.meet_link ?? '').catch(() => undefined); }}>
                              {`Join with ${d.conference_label ?? 'Google Meet'}`}
                            </a>
                          </span>
                        </div>
                        <div className="popup-meet-code">{d.meet_link.replace(/^https?:\/\//, '')}</div>
                      </div>
                    </div>
                    <div className="popup-meet-copy-cell">
                      <div className="popup-meet-copy-box">
                        <span className="popup-meet-copy-span">
                          <button className="popup-meet-copy" aria-label="Copy conference info" type="button" onClick={() => navigator.clipboard?.writeText(d.meet_link ?? '').catch(() => undefined)} data-tooltip="Copy conference info">
                            <span className="popup-meet-copy-ripple"></span>
                            <span className="popup-meet-copy-icon-box">
                              <i className="popup-meet-copy-icon">content_copy</i>
                            </span>
                            <div className="popup-meet-copy-overlay"></div>
                          </button>
                        </span>
                      </div>
                    </div>
                  </div>
                  <div className="popup-meet-alert" role="alert"></div>
                  {d.conference_phone ? (
                    <>
                      <div className="popup-phone-row">
                        <div className="popup-phone-icon-cell">
                          <span className="popup-phone-icon-span">
                            <svg className="popup-phone-icon" viewBox="0 0 24 24" focusable="false">
                              <path d={ICON.phone} />
                            </svg>
                          </span>
                        </div>
                        <div className="popup-phone-box">
                          <div className="popup-phone-inner">
                            <div className="popup-phone-link-line">
                              <span className="popup-phone-link-span">
                                <a className="popup-phone-link" href={d.conference_phone.uri ?? '#'} onClick={(e) => { e.preventDefault(); if (d.conference_phone?.uri) ipc.openUrl(d.conference_phone.uri).catch(() => undefined); }}>
                                  Join by phone
                                </a>
                              </span>
                            </div>
                            <div className="popup-phone-number">{`\u202a${d.conference_phone.label}\u202c${d.conference_phone.pin ? ` PIN: \u202a${d.conference_phone.pin}\u202c#` : ''}`}</div>
                          </div>
                        </div>
                        <div className="popup-phone-end"></div>
                      </div>
                      {d.conference_phone.more_url ? (
                        <div className="popup-more-row">
                          <div className="popup-more-icon-cell">
                            <i className="popup-more-icon">launch</i>
                          </div>
                          <div className="popup-more-box">
                            <a className="popup-more-link" href={d.conference_phone.more_url} onClick={(e) => { e.preventDefault(); ipc.openUrl(d.conference_phone?.more_url ?? '').catch(() => undefined); }}>
                              More phone numbers
                            </a>
                          </div>
                        </div>
                      ) : null}
                    </>
                  ) : null}
                </div>
              ) : null}
              {d.attendees.length > 0 ? (
                <div className="popup-guests-block">
                  <div className="popup-guests-row">
                    <div className="popup-guests-icon-cell">
                      <span className="popup-guests-icon-span">
                        <svg className="popup-guests-icon" viewBox="0 0 24 24" focusable="false">
                          <path d={ICON.people} />
                        </svg>
                      </span>
                    </div>
                    <div className="popup-guests-box">
                      <div className="popup-guests-inner">
                        <div className="popup-guests-count">{`${d.attendees.length} guest${d.attendees.length === 1 ? '' : 's'}`}</div>
                        {summary[0] ? <div className="popup-guests-yes">{summary[0]}</div> : null}
                        {summary.slice(1).map((line) => (
                          <div className="popup-guests-awaiting" key={line}>
                            {line}
                          </div>
                        ))}
                      </div>
                    </div>
                    <div className="popup-guests-actions">
                      <div className="popup-guests-actions-inner">
                        <span className="popup-guests-copy-span">
                          <button className="popup-guests-copy" aria-label="Copy guest emails" type="button" onClick={() => navigator.clipboard?.writeText(d.attendees.map((a) => a.email).join(', ')).catch(() => undefined)}>
                            <span className="popup-guests-copy-ripple"></span>
                            <span className="popup-guests-copy-icon-box">
                              <i className="popup-guests-copy-icon">content_copy</i>
                            </span>
                            <div className="popup-guests-copy-overlay"></div>
                          </button>
                        </span>
                      </div>
                    </div>
                  </div>
                  <div className="popup-tree" role="tree" aria-label="Guests">
                    {attendees.map((a) => {
                      const status = a.response_status === 'accepted' ? 'Attending' : a.response_status === 'declined' ? 'Not attending' : a.response_status === 'tentative' ? 'Maybe attending' : null;
                      const name = nameOf(a);
                      return (
                        <div className="popup-treeitem" role="treeitem" aria-label={status ? `${name}, ${status}` : name} key={a.email}>
                          <div className="popup-guest-icon-cell">
                            <div className="popup-avatar">
                              <div className="popup-avatar-bg">
                                <svg className="popup-avatar-svg" viewBox="0 0 24 24" focusable="false">
                                  <path d={ICON.person} />
                                </svg>
                              </div>
                              <div className="popup-avatar-overlay"></div>
                              {status ? (
                                <div className="popup-guest-status-wrap">
                                  <span className={`popup-guest-status popup-guest-status-${a.response_status}`}>
                                    <svg className="popup-guest-status-svg" viewBox="0 0 24 24" focusable="false">
                                      <path d={a.response_status === 'accepted' ? ICON.check : a.response_status === 'declined' ? ICON.close : ICON.question} />
                                    </svg>
                                  </span>
                                </div>
                              ) : null}
                            </div>
                          </div>
                          <div className="popup-guest-box">
                            <div className="popup-guest-inner">
                              <div className="popup-guest-n147">
                                <div className="popup-guest-line">
                                  <span className="popup-guest-name">{name}</span>
                                  {a.organizer ? <span className="popup-guest-role">Organizer</span> : null}
                                </div>
                              </div>
                            </div>
                          </div>
                          <div className="popup-guest-end"></div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              ) : null}
              {d.location ? (
                <div className="popup-location-row" role="button" tabIndex={0} aria-label={`Open ${d.location} in Maps`} onClick={openMaps}>
                  <div className="popup-location-icon-cell">
                    <span className="popup-location-icon-span">
                      <svg className="popup-location-icon" viewBox="0 0 24 24" focusable="false">
                        <path d={ICON.place} />
                      </svg>
                    </span>
                  </div>
                  <div className="popup-location-box">
                    <span className="popup-location-sr">Location:</span>
                    <div className="popup-location-inner">
                      <div className="popup-location-text">{d.location}</div>
                      <div className="popup-location-n149"></div>
                    </div>
                  </div>
                </div>
              ) : null}
              {d.description ? (
                <div className="popup-description-row">
                  <div className="popup-description-icon-cell">
                    <i className="popup-description-icon">notes</i>
                  </div>
                  <div className="popup-description-box">
                    <span className="popup-description-sr">Description:</span>
                    <span className="popup-description-text">{d.description}</span>
                  </div>
                </div>
              ) : null}
              <div className="popup-calendar-row">
                <div className="popup-calendar-icon-cell">
                  <i className="popup-calendar-icon">event</i>
                </div>
                <div className="popup-calendar-box">
                  <div className="popup-calendar-inner">
                    <div className="popup-calendar-name-line">
                      <span className="popup-calendar-sr">{`Organizer: ${d.calendar_name}`}</span>
                      <div className="popup-calendar-name-box">
                        <span className="popup-calendar-name">{d.calendar_name}</span>
                      </div>
                    </div>
                    <div className="popup-creator-line">
                      <span className="popup-creator-sr">{`Created by: ${creator}`}</span>
                      <div className="popup-creator-button" role="button">
                        <span className="popup-creator">{`Created by: ${creator}`}</span>
                      </div>
                    </div>
                    {d.also_in.length > 0 ? <div className="popup-also-in">{`Also in ${d.also_in.map((x) => x.account_name).join(', ')}`}</div> : null}
                    {d.conflict_with ? <div className="popup-conflict">{`Conflicts with ${d.conflict_with.title ?? '(No title)'} (${hhmm(d.conflict_with.start, tz)} – ${hhmm(d.conflict_with.end, tz)})`}</div> : null}
                  </div>
                </div>
              </div>
            </div>
            <div className="popup-foot1"></div>
            <div className="popup-foot2"></div>
          </div>
        </div>
      </span>
      <div className="popup-side-r1"></div>
      <div className="popup-side-r2"></div>
    </div>
  );
}
