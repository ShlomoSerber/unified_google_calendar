import { useEffect, useState, type CSSProperties } from 'react';
import { format } from 'date-fns';
import { ipc } from '../ipc';
import { withNotice } from '../lib/notice';
import { chipBackground, useTheme } from '../lib/colors';
import { hhmm, inZone, toTs } from '../lib/dates';
import { useUi } from '../state/ui';
import type { CalendarKey, ColorEntry, EditScope, EventDetail, EventDraft, Reminder, Transparency } from '../types/ipc';
import { writableCalendars } from './QuickCreate';
import { DatePicker } from './DatePicker';
import './FullForm.css';

// The event form. Google's full-page editor (component 16) was replaced on the user's request
// (docs/99, 2026-09-15) by a modal with only what works: title, dates with pickers, all day,
// repeat, Google Meet, location, description, calendar, colour, busy/visibility, notifications.
// Guests, the Find-a-time tab, Drive attachments, rich text and meeting notes are gone.
// Sizes and colours come from the measured tokens of the editor (`--ff-*`) and the dialogs.

export interface RecurrenceOption {
  label: string;
  rrule: string | null;
  custom?: boolean;
}

/** Google's repeat menu for a start date: none, daily, weekly on that weekday, monthly, annually, weekdays, custom. */
export function recurrenceOptionsFor(startTs: number, tz: string): RecurrenceOption[] {
  const d = inZone(startTs, tz);
  const day = ['SU', 'MO', 'TU', 'WE', 'TH', 'FR', 'SA'][d.getDay()] ?? 'MO';
  const nth = Math.ceil(d.getDate() / 7);
  return [
    { label: 'Does not repeat', rrule: null },
    { label: 'Daily', rrule: 'RRULE:FREQ=DAILY' },
    { label: `Weekly on ${format(d, 'EEEE')}`, rrule: `RRULE:FREQ=WEEKLY;BYDAY=${day}` },
    { label: `Monthly on the ${['first', 'second', 'third', 'fourth', 'last'][Math.min(nth, 5) - 1]} ${format(d, 'EEEE')}`, rrule: `RRULE:FREQ=MONTHLY;BYDAY=${nth >= 5 ? -1 : nth}${day}` },
    { label: `Annually on ${format(d, 'MMMM d')}`, rrule: 'RRULE:FREQ=YEARLY' },
    { label: 'Every weekday (Monday to Friday)', rrule: 'RRULE:FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR' },
    { label: 'Custom...', rrule: null, custom: true },
  ];
}

export function nextVisibility(v: string | null): string | null {
  const order = [null, 'public', 'private'];
  return order[(order.indexOf(v) + 1) % order.length] ?? null;
}

/** Times of a day in quarter hours, "HH:mm", like Google's time menu. */
const TIMES = Array.from({ length: 96 }, (_, i) => `${String(Math.floor(i / 4)).padStart(2, '0')}:${String((i % 4) * 15).padStart(2, '0')}`);

const ICON = {
  close: 'M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z',
};

export interface FullFormProps {
  occurrenceId: string | null;
  startTs: number;
  endTs: number;
  allDay: boolean;
  draft?: EventDraft;
}

export function FullForm({ occurrenceId, startTs: initialStart, endTs: initialEnd, allDay: initialAllDay, draft: initialDraft }: FullFormProps) {
  const tz = useUi((s) => s.tz);
  const calendars = useUi((s) => s.calendars);
  const accounts = useUi((s) => s.accounts);
  const settings = useUi((s) => s.settings);
  const pendingRrule = useUi((s) => s.pendingRrule);
  const theme = useTheme();
  const writable = writableCalendars(calendars, settings?.default_calendar ?? null);
  const [detail, setDetail] = useState<EventDetail | null>(null);
  const [title, setTitle] = useState(initialDraft?.title ?? '');
  const [startTs, setStartTs] = useState(initialStart);
  const [endTs, setEndTs] = useState(initialEnd);
  const [allDay, setAllDay] = useState(initialAllDay);
  const [recurrence, setRecurrence] = useState<string[]>(initialDraft?.recurrence ?? []);
  const [location, setLocation] = useState(initialDraft?.location ?? '');
  const [description, setDescription] = useState(initialDraft?.description ?? '');
  const [calendarKey, setCalendarKey] = useState<CalendarKey | null>(initialDraft ? { account_id: initialDraft.account_id, calendar_id: initialDraft.calendar_id } : writable[0] ? { account_id: writable[0].account_id, calendar_id: writable[0].id } : null);
  const [colorId, setColorId] = useState<string | null>(initialDraft?.color_id ?? null);
  const [transparency, setTransparency] = useState<Transparency | null>(initialDraft?.transparency ?? null);
  const [visibility, setVisibility] = useState<string | null>(initialDraft?.visibility ?? null);
  const [reminders, setReminders] = useState<Reminder[]>(initialDraft?.reminders ?? []);
  const [addMeet, setAddMeet] = useState(initialDraft?.add_meet ?? false);
  const [palette, setPalette] = useState<ColorEntry[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const calendar = writable.find((c) => calendarKey && c.account_id === calendarKey.account_id && c.id === calendarKey.calendar_id) ?? writable[0];
  const accountName = (id: string) => accounts.find((a) => a.id === id)?.display_name ?? '';
  const close = () => useUi.getState().closeDialog();

  useEffect(() => {
    ipc.getColors().then((p) => setPalette(p.events)).catch(() => undefined);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && useUi.getState().overlay.kind === 'none') useUi.getState().closeDialog();
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, []);

  // Editing: load the event and fill the form once.
  useEffect(() => {
    if (!occurrenceId) return undefined;
    let cancelled = false;
    ipc
      .getEvent(occurrenceId)
      .then((d) => {
        if (cancelled) return;
        setDetail(d);
        setTitle(d.title ?? '');
        setStartTs(d.start);
        setEndTs(d.end);
        setAllDay(d.all_day);
        setRecurrence(d.recurrence);
        setLocation(d.location ?? '');
        setDescription(d.description ?? '');
        setCalendarKey({ account_id: d.account_id, calendar_id: d.calendar_id });
        setColorId(d.color_id);
        setTransparency(d.transparency);
        setVisibility(d.visibility);
        setReminders(d.use_default_reminders ? [] : d.reminders);
        setAddMeet(Boolean(d.meet_link));
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [occurrenceId]);

  // The custom recurrence dialog hands its rule back through the store: applied during render
  // (React's derived-state pattern), then cleared.
  const [appliedRule, setAppliedRule] = useState<string | null | undefined>(undefined);
  if (pendingRrule !== undefined && appliedRule !== pendingRrule) {
    setAppliedRule(pendingRrule);
    setRecurrence(pendingRrule ? [pendingRrule] : []);
  }
  useEffect(() => {
    if (pendingRrule !== undefined) useUi.getState().setPendingRrule(undefined);
  }, [pendingRrule]);

  /** Keeps the end after the start, preserving the duration when the start moves. */
  const moveStart = (ts: number) => {
    const duration = Math.max(endTs - startTs, 0);
    setStartTs(ts);
    setEndTs(ts + duration);
  };
  const moveEnd = (ts: number) => setEndTs(ts > startTs ? ts : startTs + 900);
  const withTime = (ts: number, time: string) => {
    const d = inZone(ts, tz);
    const [h, m] = time.split(':').map(Number);
    d.setHours(h ?? 0, m ?? 0, 0, 0);
    return toTs(d);
  };
  const timeOption = (ts: number) => {
    const t = hhmm(ts, tz);
    return TIMES.includes(t) ? TIMES : [t, ...TIMES].sort();
  };
  const setReminder = (i: number, r: Reminder) => setReminders(reminders.map((x, j) => (j === i ? r : x)));
  const recurrenceOptions = recurrenceOptionsFor(startTs, tz);
  const currentRule = recurrence[0] ?? null;
  const knownRule = recurrenceOptions.some((o) => !o.custom && o.rrule === currentRule);
  const onRecurrence = (value: string) => {
    if (value === 'custom') {
      useUi.getState().openOverlay({ kind: 'recurrence', rrule: currentRule, startTs });
      return;
    }
    setRecurrence(value ? [value] : []);
  };

  const draft = (): EventDraft => ({
    account_id: calendar?.account_id ?? '',
    calendar_id: calendar?.id ?? '',
    title: title.trim() || null,
    description: description.trim() || null,
    location: location.trim() || null,
    all_day: allDay,
    start: allDay ? null : startTs,
    end: allDay ? null : endTs,
    start_date: allDay ? format(inZone(startTs, tz), 'yyyy-MM-dd') : null,
    end_date: allDay ? format(inZone(endTs + 86_400, tz), 'yyyy-MM-dd') : null,
    time_zone: tz,
    recurrence,
    attendees: detail?.attendees.map((a) => a.email) ?? initialDraft?.attendees ?? [],
    reminders: reminders.length ? reminders : null,
    color_id: colorId,
    add_meet: addMeet,
    transparency,
    visibility,
  });
  const save = async () => {
    if (!calendar) return;
    setBusy(true);
    try {
      if (!occurrenceId) {
        const d = draft();
        close();
        await withNotice('Saving...', 'Event saved', () => ipc.createEvent(d));
      } else if (detail?.is_recurring) {
        // The scope dialog (component 19) applies the update (docs/03 section 4).
        useUi.getState().openOverlay({ kind: 'edit-scope', occurrenceId, action: 'update', draft: draft() });
        setBusy(false);
      } else {
        const d = draft();
        close();
        await withNotice('Saving...', 'Event saved', () => ipc.updateEvent(occurrenceId, d, 'this' as EditScope));
      }
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  const dotStyle = { '--data-chip-bg': chipBackground(colorId ? (palette.find((c) => c.id === colorId)?.bg ?? calendar?.color_bg ?? '') : (calendar?.color_bg ?? ''), theme) } as CSSProperties;
  const calendarValue = calendar ? `${calendar.account_id}|${calendar.id}` : '';

  return (
    <div className="ef-scrim" onMouseDown={close}>
      <div className="ef-root scope-root" role="dialog" aria-modal="true" aria-label={occurrenceId ? 'Edit event' : 'New event'} onMouseDown={(e) => e.stopPropagation()}>
        <div className="ef-head">
          <input className="ef-title" placeholder="Add title" aria-label="Title" value={title} onChange={(e) => setTitle(e.target.value)} autoFocus />
          <button className="ef-save" type="button" disabled={busy || !calendar} onClick={() => void save()}>
            <span className="ef-save-ripple ugc-state"></span>
            <span className="ef-save-label">Save</span>
          </button>
          <button className="ef-close" type="button" aria-label="Close" data-tooltip="Close" onClick={close}>
            <span className="ef-close-ripple ugc-state ugc-state-icon"></span>
            <svg className="ef-close-icon" viewBox="0 0 24 24" focusable="false">
              <path d={ICON.close} />
            </svg>
          </button>
        </div>
        <div className="ef-body">
          <div className="ef-row">
            <i className="ef-icon">schedule</i>
            <div className="ef-cell">
              <div className="ef-when">
                <DatePicker value={startTs} tz={tz} label="Start date" onChange={moveStart} />
                {!allDay ? (
                  <select className="ef-select ef-time" aria-label="Start time" value={hhmm(startTs, tz)} onChange={(e) => moveStart(withTime(startTs, e.target.value))}>
                    {timeOption(startTs).map((t) => (
                      <option value={t} key={t}>{t}</option>
                    ))}
                  </select>
                ) : null}
                <span className="ef-when-to">to</span>
                {!allDay ? (
                  <select className="ef-select ef-time" aria-label="End time" value={hhmm(endTs, tz)} onChange={(e) => moveEnd(withTime(endTs, e.target.value))}>
                    {timeOption(endTs).map((t) => (
                      <option value={t} key={t}>{t}</option>
                    ))}
                  </select>
                ) : null}
                <DatePicker value={endTs} tz={tz} label="End date" onChange={moveEnd} />
              </div>
              <div className="ef-when-options">
                <label className="ef-allday">
                  <input className="ef-allday-input" type="checkbox" checked={allDay} onChange={(e) => setAllDay(e.target.checked)} />
                  <span className={allDay ? 'ef-check ef-check-on' : 'ef-check'} aria-hidden="true">
                    <svg className="ef-check-svg" viewBox="0 0 24 24" focusable="false">
                      <path className="ef-check-path" d="M1.73,12.91 8.1,19.28 22.79,4.59" fill="none" />
                    </svg>
                  </span>
                  <span className="ef-allday-label">All day</span>
                </label>
                <select className="ef-select" aria-label="Repeat" value={knownRule ? (currentRule ?? '') : 'current'} onChange={(e) => onRecurrence(e.target.value)}>
                  {!knownRule && currentRule ? <option value="current">{detail?.recurrence_text ?? 'Custom'}</option> : null}
                  {recurrenceOptions.map((o) => (
                    <option value={o.custom ? 'custom' : (o.rrule ?? '')} key={o.label}>{o.label}</option>
                  ))}
                </select>
              </div>
            </div>
          </div>
          <div className="ef-row">
            <i className="ef-icon">videocam</i>
            <div className="ef-cell">
              {addMeet ? (
                <div className="ef-meet-on">
                  <span className="ef-meet-on-text">{detail?.meet_link ? `Google Meet: ${detail.meet_link.replace(/^https?:\/\//, '')}` : 'Google Meet video conferencing will be added on save'}</span>
                  <button className="ef-close ef-meet-remove" type="button" aria-label="Remove Google Meet" data-tooltip="Remove Google Meet" onClick={() => setAddMeet(false)}>
                    <span className="ef-close-ripple ugc-state ugc-state-icon"></span>
                    <svg className="ef-close-icon" viewBox="0 0 24 24" focusable="false">
                      <path d={ICON.close} />
                    </svg>
                  </button>
                </div>
              ) : (
                <button className="ef-meet" type="button" onClick={() => setAddMeet(true)}>
                  <span className="ef-meet-ripple ugc-state ugc-state-primary"></span>
                  <span className="ef-meet-label">Add Google Meet video conferencing</span>
                </button>
              )}
            </div>
          </div>
          <div className="ef-row">
            <i className="ef-icon">location_on</i>
            <div className="ef-cell">
              <input className="ef-input" placeholder="Add location" aria-label="Location" value={location} onChange={(e) => setLocation(e.target.value)} />
            </div>
          </div>
          <div className="ef-row">
            <i className="ef-icon">notes</i>
            <div className="ef-cell">
              <textarea className="ef-input ef-description" placeholder="Add description" aria-label="Description" value={description} onChange={(e) => setDescription(e.target.value)} rows={4} />
            </div>
          </div>
          <div className="ef-row">
            <i className="ef-icon">calendar_today</i>
            <div className="ef-cell">
              <div className="ef-inline">
                <span className="ef-dot" style={dotStyle}></span>
                <select className="ef-select" aria-label="Calendar" value={calendarValue} onChange={(e) => { const [account_id, calendar_id] = e.target.value.split('|'); if (account_id && calendar_id) setCalendarKey({ account_id, calendar_id }); }}>
                  {writable.map((c) => (
                    <option value={`${c.account_id}|${c.id}`} key={`${c.account_id}|${c.id}`}>{`${c.summary} · ${accountName(c.account_id)}`}</option>
                  ))}
                </select>
                <select className="ef-select" aria-label="Colour" value={colorId ?? ''} onChange={(e) => setColorId(e.target.value || null)}>
                  <option value="">Calendar colour</option>
                  {palette.map((c) => (
                    <option value={c.id} key={c.id}>{c.name}</option>
                  ))}
                </select>
              </div>
              <div className="ef-inline">
                <select className="ef-select" aria-label="Show as" value={transparency ?? 'opaque'} onChange={(e) => setTransparency(e.target.value as Transparency)}>
                  <option value="opaque">Busy</option>
                  <option value="transparent">Free</option>
                </select>
                <select className="ef-select" aria-label="Visibility" value={visibility ?? ''} onChange={(e) => setVisibility(e.target.value || null)}>
                  <option value="">Default visibility</option>
                  <option value="public">Public</option>
                  <option value="private">Private</option>
                </select>
              </div>
            </div>
          </div>
          <div className="ef-row">
            <i className="ef-icon">notifications</i>
            <div className="ef-cell">
              {reminders.map((r, i) => (
                <div className="ef-inline" key={i}>
                  <select className="ef-select" aria-label="Notification method" value={r.method} onChange={(e) => setReminder(i, { ...r, method: e.target.value as Reminder['method'] })}>
                    <option value="popup">Notification</option>
                    <option value="email">Email</option>
                  </select>
                  <input className="ef-input ef-minutes" type="number" min={0} aria-label="Minutes before" value={r.minutes % 1440 === 0 && r.minutes > 0 ? r.minutes / 1440 : r.minutes % 60 === 0 && r.minutes > 0 ? r.minutes / 60 : r.minutes} onChange={(e) => { const n = Math.max(0, Number(e.target.value) || 0); const unit = r.minutes % 1440 === 0 && r.minutes > 0 ? 1440 : r.minutes % 60 === 0 && r.minutes > 0 ? 60 : 1; setReminder(i, { ...r, minutes: n * unit }); }} />
                  <select className="ef-select" aria-label="Unit" value={r.minutes % 1440 === 0 && r.minutes > 0 ? 'days' : r.minutes % 60 === 0 && r.minutes > 0 ? 'hours' : 'minutes'} onChange={(e) => { const cur = r.minutes % 1440 === 0 && r.minutes > 0 ? r.minutes / 1440 : r.minutes % 60 === 0 && r.minutes > 0 ? r.minutes / 60 : r.minutes; const unit = e.target.value === 'days' ? 1440 : e.target.value === 'hours' ? 60 : 1; setReminder(i, { ...r, minutes: cur * unit }); }}>
                    <option value="minutes">minutes</option>
                    <option value="hours">hours</option>
                    <option value="days">days</option>
                  </select>
                  <button className="ef-close" type="button" aria-label="Remove notification" data-tooltip="Remove notification" onClick={() => setReminders(reminders.filter((_, j) => j !== i))}>
                    <span className="ef-close-ripple ugc-state ugc-state-icon"></span>
                    <svg className="ef-close-icon" viewBox="0 0 24 24" focusable="false">
                      <path d={ICON.close} />
                    </svg>
                  </button>
                </div>
              ))}
              <button className="ef-meet ef-add-notification" type="button" onClick={() => setReminders([...reminders, { method: 'popup', minutes: 10 }])}>
                <span className="ef-meet-ripple ugc-state ugc-state-primary"></span>
                <span className="ef-meet-label">Add notification</span>
              </button>
            </div>
          </div>
          {error ? <div className="ef-error" role="alert">{error}</div> : null}
        </div>
      </div>
    </div>
  );
}
