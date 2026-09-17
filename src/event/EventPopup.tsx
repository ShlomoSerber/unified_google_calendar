import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type MouseEvent } from 'react';
import { format } from 'date-fns';
import type { MdMenu } from '@material/web/menu/menu.js';
import { ipc } from '../ipc';
import { withNotice } from '../lib/notice';
import { chipColors, useTheme } from '../lib/colors';
import { hhmm, inZone } from '../lib/dates';
import { useUi } from '../state/ui';
import { LAYOUT, layoutNumber } from '../styles/layout';
import type { AttendeeInfo, EventDetail } from '../types/ipc';
import { Tooltip } from '../app/Tooltip';
import './EventPopup.css';

// Event detail card (docs/11 section 7): surface-container-high, extra-large corner, level-3
// elevation, 400 px wide, placed next to the day column of its chip (right when it fits, else
// left). Actions at the top right; then the colour dot and title, Meet, guests, location,
// description, calendar and organizer.

/** Where the popup goes for the day column of its chip: right of the column when it fits, else
 *  left of it; vertically centred in the viewport and kept inside it. */
export function popupPosition(
  column: { left: number; right: number },
  viewport: { width: number; height: number },
  size: { width: number; height: number },
): { left: number; top: number; side: 'right' | 'left' } {
  const gap = layoutNumber(LAYOUT.popup_gap);
  let left = column.right + gap;
  let side: 'right' | 'left' = 'right';
  if (left + size.width > viewport.width) {
    left = column.left - gap - size.width;
    side = 'left';
  }
  left = Math.max(0, Math.min(left, viewport.width - size.width));
  const top = Math.max(0, Math.min((viewport.height - size.height) / 2, viewport.height - size.height));
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

/** The day column or cell that contains an anchor rect, or the rect itself (agenda rows). */
export function columnRect(anchor: DOMRect): { left: number; right: number } {
  const x = anchor.left + anchor.width / 2;
  const y = anchor.top + anchor.height / 2;
  const col = document.elementsFromPoint(x, y).find((e) => e.classList.contains('week-column') || e.classList.contains('month-view-cell') || e.classList.contains('all-day-cell'));
  if (col) {
    const r = col.getBoundingClientRect();
    return { left: r.left, right: r.right };
  }
  return { left: anchor.left, right: anchor.right };
}

const STATUS: Record<string, string> = { accepted: 'Attending', declined: 'Not attending', tentative: 'Maybe attending' };

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
  const rootRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<MdMenu>(null);
  const [pos, setPos] = useState<{ left: number; top: number; side: 'right' | 'left' } | null>(null);
  const close = () => useUi.getState().closeDialog();
  const motion = exiting ? ' motion-popup-exit' : detail && pos ? ' motion-popup-enter' : '';

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
    // Escape and clicks outside close the popup, unless a dialog is stacked over it.
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && useUi.getState().overlay.kind === 'none') close();
    };
    const onDown = (e: globalThis.MouseEvent) => {
      if (e.target instanceof Element && !e.target.closest('.event-popup, md-dialog, md-menu')) close();
    };
    document.addEventListener('keydown', onKey);
    document.addEventListener('mousedown', onDown);
    return () => {
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('mousedown', onDown);
    };
  }, []);

  const style: CSSProperties = { left: pos?.left ?? 0, top: pos?.top ?? 0, visibility: pos ? 'visible' : 'hidden' };
  const stop = (e: MouseEvent) => e.stopPropagation();

  if (error) {
    return (
      <div className={`event-popup md-typescale-body-medium${motion}`} role="dialog" aria-label="Event" ref={rootRef} style={style} onMouseDown={stop}>
        <div className="event-popup-error">{`The event could not be loaded: ${error}`}</div>
      </div>
    );
  }
  if (!detail) return <div className="event-popup" role="dialog" aria-label="Event" ref={rootRef} style={style}></div>;

  const d = detail;
  // All-day events carry UTC midnights (docs/03): their dates are read in UTC, end exclusive.
  const start = inZone(d.start, d.all_day ? 'UTC' : tz);
  const lastDay = d.all_day ? inZone(d.end - 86_400, 'UTC') : null;
  const dateLabel = lastDay && lastDay.getTime() > start.getTime() ? `${format(start, 'EEEE, MMMM d')} – ${format(lastDay, 'EEEE, MMMM d')}` : format(start, 'EEEE, MMMM d');
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
  const openUrl = (url: string | null | undefined) => () => {
    if (url) ipc.openUrl(url).catch(() => undefined);
  };
  const toggleMenu = () => {
    if (menuRef.current) menuRef.current.open = !menuRef.current.open;
  };
  const dotStyle = { '--data-chip-color': chipColors(d.color_bg, theme).color } as CSSProperties;
  const meetCode = (d.meet_link ?? '').replace(/^https?:\/\//, '');
  // Google resolves attendee names from contacts; the app knows its own accounts' names.
  const nameOf = (a: AttendeeInfo) => a.display_name ?? accounts.find((x) => x.email?.toLowerCase() === a.email.toLowerCase())?.display_name ?? a.email;
  const order = (a: AttendeeInfo) => (a.organizer ? 0 : a.response_status === 'accepted' ? 1 : a.response_status === 'tentative' ? 2 : a.response_status === 'declined' ? 3 : 4);
  const attendees = [...d.attendees].sort((x, y) => order(x) - order(y) || nameOf(x).localeCompare(nameOf(y)));
  const summary = attendeeSummary(d.attendees);
  const organizerEmail = d.organizer?.email ?? '';
  const creator = d.organizer?.display_name ?? (organizerEmail.endsWith('@group.calendar.google.com') || !organizerEmail ? d.account_name : organizerEmail);
  const readOnly = 'This calendar is read-only';

  return (
    <div className={`event-popup${motion}`} role="dialog" aria-labelledby="event-popup-title" ref={rootRef} style={style} onMouseDown={stop}>
      <div className="event-popup-actions">
        <Tooltip text={d.can_edit ? 'Edit event' : readOnly}>
          <md-icon-button aria-label="Edit event" disabled={!d.can_edit} onclick={edit}>
            <md-icon>edit</md-icon>
          </md-icon-button>
        </Tooltip>
        <Tooltip text={d.can_delete ? 'Delete event' : readOnly}>
          <md-icon-button aria-label="Delete event" disabled={!d.can_delete} onclick={remove}>
            <md-icon>delete</md-icon>
          </md-icon-button>
        </Tooltip>
        <span className="event-popup-more">
          <Tooltip text={otherAccounts.length === 0 ? 'Add another Google account to move events between accounts' : 'Options'}>
            <md-icon-button id="event-popup-options" aria-label="Options" aria-haspopup="menu" disabled={otherAccounts.length === 0} onclick={toggleMenu}>
              <md-icon>more_vert</md-icon>
            </md-icon-button>
          </Tooltip>
          <md-menu ref={menuRef} anchor="event-popup-options" positioning="fixed" aria-label="Options">
            {otherAccounts.map((a) => (
              <md-menu-item key={a.id} onclick={() => ipc.moveEventAccount(d.occurrence_id, a.id, 'primary').then(setDetail).catch((e: unknown) => setError(String(e)))}>
                <div slot="headline">{`Move to ${a.display_name}`}</div>
              </md-menu-item>
            ))}
          </md-menu>
        </span>
        <Tooltip text="Close">
          <md-icon-button aria-label="Close" onclick={close}>
            <md-icon>close</md-icon>
          </md-icon-button>
        </Tooltip>
      </div>
      <div className="event-popup-body">
        <span className="event-popup-dot" style={dotStyle}></span>
        <div className="event-popup-heading">
          <h2 className="event-popup-title md-typescale-headline-small" id="event-popup-title">
            {d.title ?? '(No title)'}
          </h2>
          <div className="md-typescale-body-medium">{timeLabel ? `${dateLabel} ⋅ ${timeLabel}` : dateLabel}</div>
          {d.recurrence_text ? <div className="event-popup-muted md-typescale-body-medium">{d.recurrence_text}</div> : null}
        </div>
        {d.meet_link ? (
          <>
            <md-icon>videocam</md-icon>
            <div className="event-popup-block">
              <div className="event-popup-inline">
                <md-text-button onclick={openUrl(d.meet_link)}>
                  <md-icon slot="icon">videocam</md-icon>
                  {`Join with ${d.conference_label ?? 'Google Meet'}`}
                </md-text-button>
                <Tooltip text="Copy conference info">
                  <md-icon-button aria-label="Copy conference info" onclick={() => navigator.clipboard?.writeText(d.meet_link ?? '').catch(() => undefined)}>
                    <md-icon>content_copy</md-icon>
                  </md-icon-button>
                </Tooltip>
              </div>
              <div className="event-popup-muted md-typescale-body-small">{meetCode}</div>
              {d.conference_phone ? (
                <div className="event-popup-phone">
                  <div className="event-popup-chips">
                    <md-text-button onclick={openUrl(d.conference_phone.uri)}>
                      <md-icon slot="icon">call</md-icon>
                      Join by phone
                    </md-text-button>
                    {d.conference_phone.more_url ? (
                      <md-text-button onclick={openUrl(d.conference_phone.more_url)}>
                        <md-icon slot="icon">open_in_new</md-icon>
                        More phone numbers
                      </md-text-button>
                    ) : null}
                  </div>
                  <span className="event-popup-muted md-typescale-body-small">{`‪${d.conference_phone.label}‬${d.conference_phone.pin ? ` PIN: ‪${d.conference_phone.pin}‬#` : ''}`}</span>
                </div>
              ) : null}
            </div>
          </>
        ) : null}
        {d.attendees.length > 0 ? (
          <>
            <md-icon>group</md-icon>
            <div className="event-popup-block">
              <div className="event-popup-inline">
                <div>
                  <div className="md-typescale-body-medium">{`${d.attendees.length} guest${d.attendees.length === 1 ? '' : 's'}`}</div>
                  <div className="event-popup-muted md-typescale-body-small">{summary.join(', ')}</div>
                </div>
                <Tooltip text="Copy guest emails">
                  <md-icon-button aria-label="Copy guest emails" onclick={() => navigator.clipboard?.writeText(d.attendees.map((a) => a.email).join(', ')).catch(() => undefined)}>
                    <md-icon>content_copy</md-icon>
                  </md-icon-button>
                </Tooltip>
              </div>
              <ul className="event-popup-guests" aria-label="Guests">
                {attendees.map((a) => {
                  const status = STATUS[a.response_status] ?? null;
                  const name = nameOf(a);
                  return (
                    <li className="event-popup-guest" key={a.email}>
                      <md-icon>person</md-icon>
                      <div>
                        <div className="md-typescale-body-medium">{a.organizer ? `${name} · Organizer` : name}</div>
                        {status ? <div className="event-popup-muted md-typescale-body-small">{status}</div> : null}
                      </div>
                    </li>
                  );
                })}
              </ul>
            </div>
          </>
        ) : null}
        {d.location ? (
          <>
            <md-icon>place</md-icon>
            <div className="event-popup-block">
              <div className="md-typescale-body-medium">{d.location}</div>
              <div>
                <md-text-button aria-label={`Open ${d.location} in Maps`} onclick={openMaps}>
                  <md-icon slot="icon">open_in_new</md-icon>
                  Open in Maps
                </md-text-button>
              </div>
            </div>
          </>
        ) : null}
        {d.description ? (
          <>
            <md-icon>notes</md-icon>
            <div className="event-popup-description md-typescale-body-medium">{d.description}</div>
          </>
        ) : null}
        <md-icon>calendar_today</md-icon>
        <div className="event-popup-block">
          <div className="md-typescale-body-medium">{d.calendar_name}</div>
          <div className="event-popup-muted md-typescale-body-small">{`Created by: ${creator}`}</div>
          {d.also_in.length > 0 ? <div className="event-popup-muted md-typescale-body-small">{`Also in ${d.also_in.map((x) => x.account_name).join(', ')}`}</div> : null}
          {d.conflict_with ? <div className="event-popup-conflict md-typescale-body-small">{`Conflicts with ${d.conflict_with.title ?? '(No title)'} (${hhmm(d.conflict_with.start, tz)} – ${hhmm(d.conflict_with.end, tz)})`}</div> : null}
        </div>
      </div>
    </div>
  );
}
