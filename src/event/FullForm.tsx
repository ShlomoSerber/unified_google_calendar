import { useEffect, useRef, useState, type CSSProperties } from 'react';
import { format } from 'date-fns';
import type { MdDialog } from '@material/web/dialog/dialog.js';
import type { MdOutlinedSelect } from '@material/web/select/outlined-select.js';
import type { MdOutlinedTextField } from '@material/web/textfield/outlined-text-field.js';
import type { MdSwitch } from '@material/web/switch/switch.js';
import { ipc } from '../ipc';
import { withNotice } from '../lib/notice';
import { writableCalendars } from '../lib/calendars';
import { chipColors, useTheme } from '../lib/colors';
import { hhmm, inZone, toTs } from '../lib/dates';
import { useUi } from '../state/ui';
import type { CalendarKey, ColorEntry, EditScope, EventDetail, EventDraft, Reminder, Transparency } from '../types/ipc';
import { DatePicker } from './DatePicker';
import './FullForm.css';

// The event form (docs/11 section 7): an md-dialog of 640 px with only what works in version 1
// (docs/99, 2026-09-15): title, dates with pickers, times, all day, repeat, Google Meet,
// location, description, calendar, colour, busy/free, visibility and notifications. Guests,
// attachments, rich text and meeting notes are out of scope.

export interface RecurrenceOption {
  label: string;
  rrule: string | null;
  custom?: boolean;
}

/** The repeat menu for a start date: none, daily, weekly on that weekday, monthly, annually, weekdays, custom. */
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

/** Times of a day in quarter hours, "HH:mm". */
const TIMES = Array.from({ length: 96 }, (_, i) => `${String(Math.floor(i / 4)).padStart(2, '0')}:${String((i % 4) * 15).padStart(2, '0')}`);

/** Reminder minutes → the unit the form shows them in. */
function reminderUnit(minutes: number): { unit: 'minutes' | 'hours' | 'days'; count: number; factor: number } {
  if (minutes > 0 && minutes % 1440 === 0) return { unit: 'days', count: minutes / 1440, factor: 1440 };
  if (minutes > 0 && minutes % 60 === 0) return { unit: 'hours', count: minutes / 60, factor: 60 };
  return { unit: 'minutes', count: minutes, factor: 1 };
}
const FACTOR: Record<'minutes' | 'hours' | 'days', number> = { minutes: 1, hours: 60, days: 1440 };

const selectValue = (e: Event) => (e.target as MdOutlinedSelect).value;
/** md-select drops an empty-string value once its options render, so "none" options use this. */
const NONE = 'none';
const fieldValue = (e: Event) => (e.target as MdOutlinedTextField).value;

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
  const dialog = useRef<MdDialog>(null);
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
  // The dialog closes itself (animation), then `closed` removes it from the store.
  const close = () => dialog.current?.close();

  useEffect(() => {
    dialog.current?.show();
    ipc.getColors().then((p) => setPalette(p.events)).catch(() => undefined);
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
  const repeatValue = knownRule ? (currentRule ?? NONE) : 'current';
  // A custom rule adds its option in the same render that selects it; md-select only resolves a
  // value against options already in the DOM, so the value is set again once they are.
  const repeatSelect = useRef<MdOutlinedSelect>(null);
  useEffect(() => {
    if (repeatSelect.current && repeatSelect.current.value !== repeatValue) repeatSelect.current.value = repeatValue;
  }, [repeatValue]);
  const onRecurrence = (value: string) => {
    if (value === 'custom') {
      useUi.getState().openOverlay({ kind: 'recurrence', rrule: currentRule, startTs });
      return;
    }
    setRecurrence(value === NONE ? [] : [value]);
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
        // The scope dialog applies the update (docs/03 section 4).
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

  const dot = (hex: string) => ({ '--data-chip-color': chipColors(hex, theme).color }) as CSSProperties;
  const calendarValue = calendar ? `${calendar.account_id}|${calendar.id}` : '';
  const onCancel = (e: Event) => {
    // Escape belongs to the dialog stacked on top (recurrence, scope) while one is open.
    if (useUi.getState().overlay.kind !== 'none') e.preventDefault();
  };

  return (
    <md-dialog className="full-form" ref={dialog} aria-label={occurrenceId ? 'Edit event' : 'New event'} oncancel={onCancel} onclosed={() => useUi.getState().closeDialog()}>
      <div slot="headline">{occurrenceId ? 'Edit event' : 'New event'}</div>
      <div slot="content" className="full-form-content">
        <md-outlined-text-field label="Title" value={title} autofocus oninput={(e) => setTitle(fieldValue(e))}></md-outlined-text-field>
        <div className="full-form-row">
          <DatePicker value={startTs} tz={tz} label="Start date" onChange={moveStart} />
          {!allDay ? (
            <md-outlined-select label="Start time" value={hhmm(startTs, tz)} onchange={(e) => moveStart(withTime(startTs, selectValue(e)))}>
              {timeOption(startTs).map((t) => (
                <md-select-option value={t} key={t}>
                  <div slot="headline">{t}</div>
                </md-select-option>
              ))}
            </md-outlined-select>
          ) : null}
        </div>
        <div className="full-form-row">
          <DatePicker value={endTs} tz={tz} label="End date" onChange={moveEnd} />
          {!allDay ? (
            <md-outlined-select label="End time" value={hhmm(endTs, tz)} onchange={(e) => moveEnd(withTime(endTs, selectValue(e)))}>
              {timeOption(endTs).map((t) => (
                <md-select-option value={t} key={t}>
                  <div slot="headline">{t}</div>
                </md-select-option>
              ))}
            </md-outlined-select>
          ) : null}
        </div>
        <div className="full-form-row">
          <label className="full-form-switch md-typescale-body-medium">
            <md-switch selected={allDay} aria-label="All day" onchange={(e) => setAllDay((e.target as MdSwitch).selected)}></md-switch>
            All day
          </label>
          <md-outlined-select label="Repeat" ref={repeatSelect} value={repeatValue} onchange={(e) => onRecurrence(selectValue(e))}>
            {!knownRule && currentRule ? (
              <md-select-option value="current">
                <div slot="headline">{detail?.recurrence_text ?? 'Custom'}</div>
              </md-select-option>
            ) : null}
            {recurrenceOptions.map((o) => (
              <md-select-option value={o.custom ? 'custom' : (o.rrule ?? NONE)} key={o.label}>
                <div slot="headline">{o.label}</div>
              </md-select-option>
            ))}
          </md-outlined-select>
        </div>
        {addMeet ? (
          <div className="full-form-meet md-typescale-body-medium">
            <md-icon>videocam</md-icon>
            <span className="full-form-meet-text">{detail?.meet_link ? `Google Meet: ${detail.meet_link.replace(/^https?:\/\//, '')}` : 'Google Meet will be added on save'}</span>
            <md-icon-button aria-label="Remove Google Meet" onclick={() => setAddMeet(false)}>
              <md-icon>close</md-icon>
            </md-icon-button>
          </div>
        ) : (
          <div>
            <md-text-button onclick={() => setAddMeet(true)}>
              <md-icon slot="icon">videocam</md-icon>
              Add Google Meet
            </md-text-button>
          </div>
        )}
        <md-outlined-text-field label="Location" value={location} oninput={(e) => setLocation(fieldValue(e))}>
          <md-icon slot="leading-icon">place</md-icon>
        </md-outlined-text-field>
        <md-outlined-text-field label="Description" type="textarea" rows={3} value={description} oninput={(e) => setDescription(fieldValue(e))}></md-outlined-text-field>
        <div className="full-form-row">
          <md-outlined-select
            label="Calendar"
            value={calendarValue}
            onchange={(e) => {
              const [account_id, calendar_id] = selectValue(e).split('|');
              if (account_id && calendar_id) setCalendarKey({ account_id, calendar_id });
            }}
          >
            {writable.map((c) => (
              <md-select-option value={`${c.account_id}|${c.id}`} key={`${c.account_id}|${c.id}`}>
                <span slot="start" className="full-form-dot" style={dot(c.color_bg)}></span>
                <div slot="headline">{`${c.summary} · ${accountName(c.account_id)}`}</div>
              </md-select-option>
            ))}
          </md-outlined-select>
          <md-outlined-select label="Colour" value={colorId ?? NONE} onchange={(e) => setColorId(selectValue(e) === NONE ? null : selectValue(e))}>
            <md-select-option value={NONE}>
              <span slot="start" className="full-form-dot" style={dot(calendar?.color_bg ?? '')}></span>
              <div slot="headline">Calendar colour</div>
            </md-select-option>
            {palette.map((c) => (
              <md-select-option value={c.id} key={c.id}>
                <span slot="start" className="full-form-dot" style={dot(c.bg)}></span>
                <div slot="headline">{c.name}</div>
              </md-select-option>
            ))}
          </md-outlined-select>
        </div>
        <div className="full-form-row">
          <md-outlined-select label="Show as" value={transparency ?? 'opaque'} onchange={(e) => setTransparency(selectValue(e) as Transparency)}>
            <md-select-option value="opaque">
              <div slot="headline">Busy</div>
            </md-select-option>
            <md-select-option value="transparent">
              <div slot="headline">Free</div>
            </md-select-option>
          </md-outlined-select>
          <md-outlined-select label="Visibility" value={visibility ?? NONE} onchange={(e) => setVisibility(selectValue(e) === NONE ? null : selectValue(e))}>
            <md-select-option value={NONE}>
              <div slot="headline">Default visibility</div>
            </md-select-option>
            <md-select-option value="public">
              <div slot="headline">Public</div>
            </md-select-option>
            <md-select-option value="private">
              <div slot="headline">Private</div>
            </md-select-option>
          </md-outlined-select>
        </div>
        {reminders.map((r, i) => {
          const u = reminderUnit(r.minutes);
          return (
            <div className="full-form-row" key={i}>
              <md-outlined-select label="Notification" value={r.method} onchange={(e) => setReminder(i, { ...r, method: selectValue(e) as Reminder['method'] })}>
                <md-select-option value="popup">
                  <div slot="headline">Notification</div>
                </md-select-option>
                <md-select-option value="email">
                  <div slot="headline">Email</div>
                </md-select-option>
              </md-outlined-select>
              <md-outlined-text-field label="Before" type="number" min="0" value={String(u.count)} oninput={(e) => setReminder(i, { ...r, minutes: Math.max(0, Number(fieldValue(e)) || 0) * u.factor })}></md-outlined-text-field>
              <md-outlined-select label="Unit" value={u.unit} onchange={(e) => setReminder(i, { ...r, minutes: u.count * FACTOR[selectValue(e) as keyof typeof FACTOR] })}>
                <md-select-option value="minutes">
                  <div slot="headline">minutes</div>
                </md-select-option>
                <md-select-option value="hours">
                  <div slot="headline">hours</div>
                </md-select-option>
                <md-select-option value="days">
                  <div slot="headline">days</div>
                </md-select-option>
              </md-outlined-select>
              <md-icon-button aria-label="Remove notification" onclick={() => setReminders(reminders.filter((_, j) => j !== i))}>
                <md-icon>close</md-icon>
              </md-icon-button>
            </div>
          );
        })}
        <div>
          <md-text-button onclick={() => setReminders([...reminders, { method: 'popup', minutes: 10 }])}>
            <md-icon slot="icon">notifications</md-icon>
            Add notification
          </md-text-button>
        </div>
        {error ? (
          <div className="full-form-error md-typescale-body-small" role="alert">
            {error}
          </div>
        ) : null}
      </div>
      <div slot="actions">
        <md-text-button onclick={close}>Cancel</md-text-button>
        <md-filled-button disabled={busy || !calendar} onclick={() => void save()}>
          Save
        </md-filled-button>
      </div>
    </md-dialog>
  );
}
