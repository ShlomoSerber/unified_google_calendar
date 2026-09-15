import { useEffect, useRef, useState, type CSSProperties, type KeyboardEvent as ReactKeyboardEvent } from 'react';
import { format, parse } from 'date-fns';
import { ipc } from '../ipc';
import { chipBackground, useTheme } from '../lib/colors';
import { hhmm, inZone, toTs } from '../lib/dates';
import { useUi } from '../state/ui';
import type { CalendarKey, ColorEntry, EditScope, EventDetail, EventDraft, Reminder, Transparency } from '../types/ipc';
import { writableCalendars } from './QuickCreate';
import './FullForm.css';

// Component 16 of docs/04 section 4 (docs/design/measurements/full_form-light.json): the event
// edit page. Google's "Find a time" tab, Drive attachments, rich-text formatting and meeting notes
// are outside version 1 and stay inert (docs/01 section 3, docs/99 F7-T4).

const UNAVAILABLE = 'Not available in this version';
const DATE_FORMAT = 'MMM d, yyyy';

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
  const [attendees, setAttendees] = useState<string[]>(initialDraft?.attendees ?? []);
  const [guestText, setGuestText] = useState('');
  const [location, setLocation] = useState(initialDraft?.location ?? '');
  const [description, setDescription] = useState(initialDraft?.description ?? '');
  const [calendarKey, setCalendarKey] = useState<CalendarKey | null>(initialDraft ? { account_id: initialDraft.account_id, calendar_id: initialDraft.calendar_id } : writable[0] ? { account_id: writable[0].account_id, calendar_id: writable[0].id } : null);
  const [colorId, setColorId] = useState<string | null>(initialDraft?.color_id ?? null);
  const [transparency, setTransparency] = useState<Transparency | null>(initialDraft?.transparency ?? null);
  const [visibility, setVisibility] = useState<string | null>(initialDraft?.visibility ?? null);
  const [reminders, setReminders] = useState<Reminder[]>(initialDraft?.reminders ?? []);
  const [addMeet, setAddMeet] = useState(initialDraft?.add_meet ?? false);
  const [palette, setPalette] = useState<ColorEntry[]>([]);
  const [menu, setMenu] = useState<'recurrence' | 'calendar' | 'color' | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const descRef = useRef<HTMLDivElement>(null);
  const [startDateText, setStartDateText] = useState(format(inZone(initialStart, tz), DATE_FORMAT));
  const [endDateText, setEndDateText] = useState(format(inZone(initialEnd, tz), DATE_FORMAT));
  const [startTimeText, setStartTimeText] = useState(hhmm(initialStart, tz));
  const [endTimeText, setEndTimeText] = useState(hhmm(initialEnd, tz));
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
    if (!occurrenceId) return;
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
        setAttendees(d.attendees.map((a) => a.email));
        setLocation(d.location ?? '');
        setDescription(d.description ?? '');
        if (descRef.current) descRef.current.innerText = d.description ?? '';
        setCalendarKey({ account_id: d.account_id, calendar_id: d.calendar_id });
        setColorId(d.color_id);
        setTransparency(d.transparency);
        setVisibility(d.visibility);
        setReminders(d.use_default_reminders ? [] : d.reminders);
        setAddMeet(Boolean(d.meet_link));
        setStartDateText(format(inZone(d.start, tz), DATE_FORMAT));
        setEndDateText(format(inZone(d.end, tz), DATE_FORMAT));
        setStartTimeText(hhmm(d.start, tz));
        setEndTimeText(hhmm(d.end, tz));
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [occurrenceId, tz]);

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

  const commitDates = () => {
    const sd = parse(startDateText, DATE_FORMAT, inZone(startTs, tz));
    const ed = parse(endDateText, DATE_FORMAT, inZone(endTs, tz));
    const st = parse(startTimeText, 'HH:mm', inZone(startTs, tz));
    const et = parse(endTimeText, 'HH:mm', inZone(endTs, tz));
    if (Number.isNaN(sd.getTime()) || Number.isNaN(ed.getTime()) || Number.isNaN(st.getTime()) || Number.isNaN(et.getTime())) {
      setStartDateText(format(inZone(startTs, tz), DATE_FORMAT));
      setEndDateText(format(inZone(endTs, tz), DATE_FORMAT));
      setStartTimeText(hhmm(startTs, tz));
      setEndTimeText(hhmm(endTs, tz));
      return;
    }
    const s = inZone(startTs, tz);
    s.setFullYear(sd.getFullYear(), sd.getMonth(), sd.getDate());
    s.setHours(st.getHours(), st.getMinutes(), 0, 0);
    const e = inZone(endTs, tz);
    e.setFullYear(ed.getFullYear(), ed.getMonth(), ed.getDate());
    e.setHours(et.getHours(), et.getMinutes(), 0, 0);
    const ns = toTs(s);
    let ne = toTs(e);
    if (ne <= ns) ne = ns + (endTs - startTs > 0 ? endTs - startTs : 3600);
    setStartTs(ns);
    setEndTs(ne);
    setStartDateText(format(inZone(ns, tz), DATE_FORMAT));
    setEndDateText(format(inZone(ne, tz), DATE_FORMAT));
    setStartTimeText(hhmm(ns, tz));
    setEndTimeText(hhmm(ne, tz));
  };
  const addGuest = () => {
    const email = guestText.trim().replace(/,$/, '');
    if (email && /@/.test(email) && !attendees.includes(email)) setAttendees([...attendees, email]);
    setGuestText('');
  };
  const addGuestOnEnter = (e: ReactKeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' || e.key === ',') {
      e.preventDefault();
      addGuest();
    }
  };
  const setReminder = (i: number, r: Reminder) => setReminders(reminders.map((x, j) => (j === i ? r : x)));
  const openLink = (e: React.MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault();
    ipc.openUrl(e.currentTarget.href).catch(() => undefined);
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
    attendees,
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
        await ipc.createEvent(draft());
      } else if (detail?.is_recurring) {
        // The scope dialog (component 19) applies the update (docs/03 section 4).
        useUi.getState().openOverlay({ kind: 'edit-scope', occurrenceId, action: 'update', draft: draft() });
        setBusy(false);
        return;
      } else {
        await ipc.updateEvent(occurrenceId, draft(), 'this' as EditScope);
      }
      close();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  const recurrenceOptions = recurrenceOptionsFor(startTs, tz);
  const recurrenceLabel = recurrence[0] ? (recurrenceOptions.find((o) => o.rrule === recurrence[0])?.label ?? detail?.recurrence_text ?? 'Custom') : 'Does not repeat';
  const visibilityLabel = visibility === 'public' ? 'Public' : visibility === 'private' ? 'Private' : 'Default visibility';
  const dotStyle = { '--data-chip-bg': chipBackground(colorId ? (palette.find((c) => c.id === colorId)?.bg ?? calendar?.color_bg ?? '') : (calendar?.color_bg ?? ''), theme) } as CSSProperties;

  return (
    <div className="ff-main" role="main">
      <h1 className="ff-sr" aria-label="Event edit page">{"Event edit page"}</h1>
      <div className="ff-col">
        <div className="ff-head-block">
          <div className="ff-head-row">
            <div className="ff-head-left">
              <div className="ff-n6">
                <div className="ff-n7"></div>
              </div>
              <div className="ff-head-title-row">
                <span className="ff-close-cell">
                  <button className="ff-close" aria-label={occurrenceId ? 'Cancel event edit' : 'Cancel event creation'} type="button" onClick={close}>
                    <span className="ff-n11"></span>
                    <span className="ff-n12">
                      <i className="ff-close-icon">{"close"}</i>
                    </span>
                    <div className="ff-n14"></div>
                  </button>
                  <div className="ff-n15" role="tooltip">{"Cancel event creation"}</div>
                </span>
                <div className="ff-title-wrap">
                  <div className="ff-n17">
                    <span className="ff-n18">
                      <div className="ff-n19">
                        <div className="ff-n20">
                          <div className="ff-n21">
                            <span className="ff-n22">
                              <input className="ff-title-input" aria-label="Title" placeholder="Add title" value={title} onChange={(e) => setTitle(e.target.value)} autoFocus />
                            </span>
                            <div className="ff-n24"></div>
                          </div>
                          <div className="ff-n25">
                            <p className="ff-n26"></p>
                            <span className="ff-n27"></span>
                          </div>
                        </div>
                      </div>
                      <div className="ff-n28" role="tooltip"></div>
                    </span>
                  </div>
                </div>
              </div>
            </div>
            <button className="ff-save" aria-label="Save" type="button" onClick={() => void save()} disabled={busy || !calendar}>
              <span className="ff-n30">
                <span className="ff-n31"></span>
              </span>
              <span className="ff-n32"></span>
              <span className="ff-save-label">{"Save"}</span>
            </button>
            <div className="ff-head-end">{error ? <span className="ff-error" role="alert">{error}</span> : null}</div>
          </div>
          <div className="ff-when-row">
            <div className="ff-when-icon-cell"></div>
            <div className="ff-when-cell">
              <div className="ff-when-box">
                <div className="ff-when-line">
                  <div className="ff-n40">
                    <div className="ff-n41">
                      <div className="ff-n42">
                        <div className="ff-n43">
                          <div className="ff-n44">
                            <div className="ff-n45">
                              <span className="ff-n46">
                                <span className="ff-n47">
                                  <div className="ff-n48">
                                    <div className="ff-n49">
                                      <span className="ff-n50">
                                        <input className="ff-start-date" aria-label="Start date" value={startDateText} onChange={(e) => setStartDateText(e.target.value)} onBlur={commitDates} />
                                      </span>
                                      <div className="ff-n52"></div>
                                    </div>
                                  </div>
                                </span>
                                <div className="ff-n53" role="tooltip"></div>
                              </span>
                            </div>
                          </div>
                        </div>
                        <div className="ff-n54">
                          <div className="ff-n55">
                            <div className="ff-n56">
                              <span className="ff-n57">
                                <span className="ff-n58">
                                  <div className="ff-n59">
                                    <div className="ff-n60">
                                      <span className="ff-n61">
                                        <input className="ff-start-time" role="combobox" aria-label="Start time" value={startTimeText} onChange={(e) => setStartTimeText(e.target.value)} onBlur={commitDates} disabled={allDay} />
                                      </span>
                                      <div className="ff-n63"></div>
                                    </div>
                                  </div>
                                </span>
                                <div className="ff-n64" role="tooltip"></div>
                              </span>
                            </div>
                          </div>
                        </div>
                      </div>
                      <div className="ff-when-to">{"to"}</div>
                      <div className="ff-n66">
                        <div className="ff-n67">
                          <div className="ff-n68">
                            <div className="ff-n69">
                              <span className="ff-n70">
                                <span className="ff-n71">
                                  <div className="ff-n72">
                                    <div className="ff-n73">
                                      <span className="ff-n74">
                                        <input className="ff-end-time" role="combobox" aria-label="End time" value={endTimeText} onChange={(e) => setEndTimeText(e.target.value)} onBlur={commitDates} disabled={allDay} />
                                      </span>
                                      <div className="ff-n76"></div>
                                    </div>
                                  </div>
                                </span>
                                <div className="ff-n77" role="tooltip"></div>
                              </span>
                            </div>
                          </div>
                        </div>
                        <div className="ff-n78">
                          <div className="ff-n79">
                            <div className="ff-n80">
                              <span className="ff-n81">
                                <span className="ff-n82">
                                  <div className="ff-n83">
                                    <div className="ff-n84">
                                      <span className="ff-n85">
                                        <input className="ff-end-date" aria-label="End date" value={endDateText} onChange={(e) => setEndDateText(e.target.value)} onBlur={commitDates} />
                                      </span>
                                      <div className="ff-n87"></div>
                                    </div>
                                  </div>
                                </span>
                                <div className="ff-n88" role="tooltip"></div>
                              </span>
                            </div>
                          </div>
                        </div>
                      </div>
                      <div className="ff-n89">
                        <button className="ff-tz-button" type="button" disabled title={`${tz} (change it in Settings)`}>
                          <span className="ff-n91"></span>
                          <span className="ff-tz-label">{"Time zone"}</span>
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
                <div className="ff-allday-row">
                  <div className="ff-n94">
                    <label className="ff-allday-label">
                      <div className="ff-n96">
                        <div className="ff-n97">
                          <input className="ff-n98" aria-label="All day" type="checkbox" checked={allDay} onChange={(e) => setAllDay(e.target.checked)} />
                          <div className={allDay ? 'ff-allday-check ff-allday-check-on' : 'ff-allday-check'}>
                            <svg className="ff-n100">
                              <path className="ff-n101" />
                            </svg>
                            <div className="ff-n102"></div>
                          </div>
                          <span className="ff-n103"></span>
                        </div>
                      </div>
                      <span className="ff-allday-text">{"All day"}</span>
                    </label>
                    <div className="ff-n105">
                      <div className="ff-n106">
                        <div className="ff-n107">
                          <div className="ff-recurrence-combo" role="combobox" aria-label="Recurrence" aria-expanded={menu === 'recurrence'} tabIndex={0} onClick={() => setMenu(menu === 'recurrence' ? null : 'recurrence')}>
                            <span className="ff-n109">
                              <span className="ff-recurrence-text" aria-label={recurrenceLabel}>{recurrenceLabel}</span>
                            </span>
                            <span className="ff-n111">
                              <svg className="ff-n112" role="presentation">
                                <polygon className="ff-n113"></polygon>
                                <polygon className="ff-n114"></polygon>
                              </svg>
                            </span>
                            <span className="ff-n115"></span>
                            <div className="ff-n116"></div>
                          </div>
                          {menu === 'recurrence' ? (
                            <ul className="ff-menu create-menu-root" role="listbox">
                              {recurrenceOptions.map((o) => (
                                <li className="create-menu-item" role="option" key={o.label} tabIndex={0} aria-selected={o.label === recurrenceLabel} onClick={() => { setMenu(null); if (o.custom) useUi.getState().openOverlay({ kind: 'recurrence', rrule: recurrence[0] ?? null, startTs }); else setRecurrence(o.rrule ? [o.rrule] : []); }}>
                                  <span className="create-menu-item-ripple"></span>
                                  <span className="create-menu-item-box">
                                    <span className="create-menu-item-label">{o.label}</span>
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
              </div>
            </div>
          </div>
        </div>
        <div className="ff-columns">
          <div className="ff-side-panel"></div>
          <div className="ff-guests-col">
            <h2 className="ff-guests-sr">{"Guests and rooms"}</h2>
            <div className="ff-n121">
              <div className="ff-guests-tablist" role="tablist">
                <div className="ff-n123">
                  <div className="ff-n124">
                    <span className="ff-n125">
                      <button className="ff-guests-tab" role="tab">
                        <span className="ff-n127">
                          <span className="ff-guests-tab-label">{"Guests"}</span>
                          <span className="ff-n129">
                            <span className="ff-n130"></span>
                          </span>
                        </span>
                        <span className="ff-n131"></span>
                      </button>
                    </span>
                  </div>
                </div>
              </div>
              <div className="ff-guests-panel" role="tabpanel">
                <span className="ff-n133">
                  <div className="ff-n134">
                    <div className="ff-n135">
                      <div className="ff-n136">
                        <div className="ff-n137">
                          <div className="ff-n138">
                            <span className="ff-n139">
                              <input className="ff-guests-input" role="combobox" aria-label="Guests" placeholder="Add guests" value={guestText} onChange={(e) => setGuestText(e.target.value)} onKeyDown={addGuestOnEnter} onBlur={addGuest} />
                            </span>
                            <div className="ff-n141"></div>
                          </div>
                        </div>
                        <span className="ff-n142"></span>
                      </div>
                    </div>
                    <div className="ff-n143">
                      <div className="ff-n144">
                        <div className="ff-n145">
                          <div className="ff-n146">
                            <div className="ff-n147">
                              <div className="ff-n148" aria-label="Guests invited to this event.">
                              {attendees.map((email) => (
                                <div className="ff-guest" key={email}>
                                  <span className="ff-guest-email">{email}</span>
                                  <button className="ff-guest-remove" type="button" aria-label={`Remove ${email}`} onClick={() => setAttendees(attendees.filter((a) => a !== email))}>
                                    <i className="ff-guest-remove-icon">close</i>
                                  </button>
                                </div>
                              ))}
                            </div>
                            </div>
                            <div className="ff-n149">
                              <div className="ff-n150" aria-label="Rooms added to this event."></div>
                            </div>
                          </div>
                        </div>
                      </div>
                      <div className="ff-n151">
                        <fieldset className="ff-perms" role="group">
                          <legend className="ff-perms-legend">{"Guest permissions"}</legend>
                          <div className="ff-n154">
                            <label className="ff-perm-modify">
                              <div className="ff-n156">
                                <div className="ff-n157">
                                  <div className="ff-n158">
                                    <input className="ff-n159" aria-label="Let guests modify the event" type="checkbox" disabled />
                                    <div className="ff-n160">
                                      <svg className="ff-n161">
                                        <path className="ff-n162" />
                                      </svg>
                                      <div className="ff-n163"></div>
                                    </div>
                                    <span className="ff-n164"></span>
                                  </div>
                                </div>
                              </div>
                              <div className="ff-perm-modify-label">{"Modify event"}</div>
                            </label>
                            <label className="ff-perm-invite">
                              <div className="ff-n167">
                                <div className="ff-n168">
                                  <div className="ff-n169">
                                    <input className="ff-n170" aria-label="Let guests invite others to the event" type="checkbox" defaultChecked disabled />
                                    <div className="ff-n171">
                                      <svg className="ff-n172">
                                        <path className="ff-n173" />
                                      </svg>
                                      <div className="ff-n174"></div>
                                    </div>
                                    <span className="ff-n175"></span>
                                  </div>
                                </div>
                              </div>
                              <div className="ff-perm-invite-label">{"Invite others"}</div>
                            </label>
                            <label className="ff-perm-see">
                              <div className="ff-n178">
                                <div className="ff-n179">
                                  <div className="ff-n180">
                                    <input className="ff-n181" aria-label="Let guests see the event guest list." type="checkbox" defaultChecked disabled />
                                    <div className="ff-n182">
                                      <svg className="ff-n183">
                                        <path className="ff-n184" />
                                      </svg>
                                      <div className="ff-n185"></div>
                                    </div>
                                    <span className="ff-n186"></span>
                                  </div>
                                </div>
                              </div>
                              <div className="ff-perm-see-label">{"See guest list"}</div>
                            </label>
                          </div>
                        </fieldset>
                      </div>
                    </div>
                  </div>
                </span>
              </div>
            </div>
          </div>
          <div className="ff-details-col">
            <h2 className="ff-details-sr">{"Event details and find a time"}</h2>
            <div className="ff-details-box">
              <span className="ff-n191">
                <span className="ff-n192"></span>
              </span>
              <div className="ff-n193">
                <div className="ff-details-tablist" role="tablist">
                  <div className="ff-n195">
                    <div className="ff-n196">
                      <span className="ff-n197">
                        <button className="ff-tab-details" role="tab" type="button" aria-selected="true">
                          <span className="ff-n199">
                            <span className="ff-tab-details-label">{"Event details"}</span>
                            <span className="ff-n201">
                              <span className="ff-n202"></span>
                            </span>
                          </span>
                          <span className="ff-n203"></span>
                        </button>
                        <button className="ff-tab-find" role="tab" type="button" aria-selected="false" disabled title={UNAVAILABLE}>
                          <span className="ff-n205">
                            <span className="ff-tab-find-label">{"Find a time"}</span>
                            <span className="ff-n207">
                              <span className="ff-n208"></span>
                            </span>
                          </span>
                          <span className="ff-n209"></span>
                        </button>
                      </span>
                    </div>
                  </div>
                </div>
                <div className="ff-details-panel" role="tabpanel">
                  <span className="ff-n211">
                    <div className="ff-n212">
                      <div className="ff-n213">
                        <div className="ff-meet-row">
                          <div className="ff-n215">
                            <i className="ff-n216">
                              <div className="ff-n217">
                                <i className="ff-n218">videocam</i>
                              </div>
                            </i>
                          </div>
                          <div className="ff-n219">
                            <div className="ff-n220">
                              <div className="ff-n221">
                                <div className="ff-n222">
                                  <div className="ff-n223">
                                    <button className="ff-meet-button" type="button" aria-pressed={addMeet} onClick={() => setAddMeet(!addMeet)}>
                                      <span className="ff-n225"></span>
                                      <span className="ff-meet-label">{addMeet ? 'Google Meet video conferencing added' : 'Add Google Meet video conferencing'}</span>
                                    </button>
                                  </div>
                                </div>
                                <div className="ff-n227"></div>
                              </div>
                            </div>
                          </div>
                        </div>
                        <div className="ff-n228"></div>
                        <div className="ff-location-row">
                          <div className="ff-n230">
                            <i className="ff-n231">
                              <svg className="ff-n232">
                                <path className="ff-n233" />
                                <circle className="ff-n234" />
                              </svg>
                            </i>
                          </div>
                          <div className="ff-n235">
                            <div className="ff-n236">
                              <div className="ff-n237">
                                <div className="ff-n238">
                                  <div className="ff-n239">
                                    <div className="ff-n240">
                                      <span className="ff-n241">
                                        <input className="ff-location-input" role="combobox" aria-label="Add location" placeholder="Add location" value={location} onChange={(e) => setLocation(e.target.value)} />
                                      </span>
                                      <div className="ff-n243"></div>
                                    </div>
                                  </div>
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                        <div className="ff-notif-row">
                          <div className="ff-n245">
                            <i className="ff-n246">
                              <svg className="ff-n247">
                                <path className="ff-n248" />
                              </svg>
                            </i>
                          </div>
                          <div className="ff-n249">
                            <div className="ff-n250">
                              <ul className="ff-notif-list" aria-label="Notifications">
                                {reminders.map((r, i) => (
                                  <li className="ff-notif-item" key={i}>
                                    <select className="ff-notif-method" aria-label="Notification method" value={r.method} onChange={(e) => setReminder(i, { ...r, method: e.target.value as Reminder['method'] })}>
                                      <option value="popup">Notification</option>
                                      <option value="email">Email</option>
                                    </select>
                                    <input className="ff-notif-minutes" aria-label="Minutes before" type="number" min={0} value={r.minutes} onChange={(e) => setReminder(i, { ...r, minutes: Number(e.target.value) })} />
                                    <span className="ff-notif-unit">minutes before</span>
                                    <button className="ff-notif-remove" type="button" aria-label="Remove notification" onClick={() => setReminders(reminders.filter((_, j) => j !== i))}>
                                      <i className="ff-notif-remove-icon">close</i>
                                    </button>
                                  </li>
                                ))}
                              </ul>
                              <div className="ff-n252">
                                <span className="ff-notif-hint">{"Notifications only apply to you."}</span>
                                <a className="ff-notif-learn" href="https://support.google.com/calendar/answer/37242" onClick={openLink}>{"Learn more about notifications"}</a>
                              </div>
                              <div className="ff-n255">
                                <div className="ff-n256">
                                  <div className="ff-n257">
                                    <button className="ff-notif-add" aria-label="Add notification" type="button" onClick={() => setReminders([...reminders, { method: 'popup', minutes: 10 }])}>
                                      <span className="ff-n259"></span>
                                      <span className="ff-n260"></span>
                                      <span className="ff-notif-add-label">{"Add notification"}</span>
                                    </button>
                                  </div>
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                        <div className="ff-calendar-row">
                          <div className="ff-n263">
                            <i className="ff-n264">{"event"}</i>
                          </div>
                          <div className="ff-n265">
                            <div className="ff-n266">
                              <div className="ff-n267">
                                <div className="ff-n268">
                                  <div className="ff-calendar-combo" role="combobox" aria-label="Calendar" aria-expanded={menu === 'calendar'} tabIndex={0} onClick={() => setMenu(menu === 'calendar' ? null : 'calendar')}>
                                    <span className="ff-n270">
                                      <span className="ff-calendar-text">{calendar?.summary ?? 'No calendar'}</span>
                                    </span>
                                    <span className="ff-n272">
                                      <svg className="ff-n273" role="presentation">
                                        <polygon className="ff-n274"></polygon>
                                        <polygon className="ff-n275"></polygon>
                                      </svg>
                                    </span>
                                    <span className="ff-n276"></span>
                                    <div className="ff-n277"></div>
                                  </div>
                                  {menu === 'calendar' ? (
                                    <ul className="ff-menu create-menu-root" role="listbox">
                                      {writable.map((c) => (
                                        <li className="create-menu-item" role="option" key={`${c.account_id}/${c.id}`} tabIndex={0} onClick={() => { setCalendarKey({ account_id: c.account_id, calendar_id: c.id }); setMenu(null); }}>
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
                            <div className="ff-n278">
                              <div className="ff-n279">
                                <div className="ff-n280">
                                  <span className="ff-n281">
                                    <button className="ff-color-button" aria-label="Calendar color, event color" type="button" aria-expanded={menu === 'color'} onClick={() => setMenu(menu === 'color' ? null : 'color')}>
                                      <span className="ff-n283"></span>
                                      <span className="ff-n284">
                                        <div className="ff-n285">
                                          <div className="ff-color-dot" style={dotStyle}></div>
                                        </div>
                                      </span>
                                      <span className="ff-n287">
                                        <svg className="ff-n288">
                                          <path className="ff-n289" />
                                          <path className="ff-n290" />
                                        </svg>
                                      </span>
                                    </button>
                                    <div className="ff-n291" role="tooltip">{"Select event color"}</div>
                                  </span>
                                </div>
                                <div className="ff-n292"></div>
                              </div>
                            </div>
                          </div>
                        </div>
                        <div className="ff-showas-row">
                          <div className="ff-n294">
                            <i className="ff-n295">
                              <svg className="ff-n296">
                                <path className="ff-n297" />
                              </svg>
                            </i>
                          </div>
                          <div className="ff-n298">
                            <div className="ff-n299">
                              <div className="ff-n300">
                                <div className="ff-n301">
                                  <div className="ff-showas-combo" role="combobox" aria-label="Show as" tabIndex={0} onClick={() => setTransparency(transparency === 'transparent' ? 'opaque' : 'transparent')}>
                                    <span className="ff-n303">
                                      <span className="ff-showas-text">{transparency === 'transparent' ? 'Free' : 'Busy'}</span>
                                    </span>
                                    <span className="ff-n305">
                                      <svg className="ff-n306" role="presentation">
                                        <polygon className="ff-n307"></polygon>
                                        <polygon className="ff-n308"></polygon>
                                      </svg>
                                    </span>
                                    <span className="ff-n309"></span>
                                    <div className="ff-n310"></div>
                                  </div>
                                </div>
                              </div>
                              <div className="ff-n311">
                                <div className="ff-n312">
                                  <div className="ff-n313">
                                    <div className="ff-visibility-combo" role="combobox" aria-label="Visibility" tabIndex={0} onClick={() => setVisibility(nextVisibility(visibility))}>
                                      <span className="ff-n315">
                                        <span className="ff-visibility-text" aria-label={visibilityLabel}>{visibilityLabel}</span>
                                      </span>
                                      <span className="ff-n317">
                                        <svg className="ff-n318" role="presentation">
                                          <polygon className="ff-n319"></polygon>
                                          <polygon className="ff-n320"></polygon>
                                        </svg>
                                      </span>
                                      <span className="ff-n321"></span>
                                      <div className="ff-n322"></div>
                                    </div>
                                    {menu === 'color' ? (
                                      <div className="ff-menu ff-color-menu create-menu-root" role="listbox" aria-label="Event color">
                                        <button className="ff-color-swatch" type="button" role="option" aria-label="Calendar color" aria-selected={colorId === null} style={{ '--data-chip-bg': chipBackground(calendar?.color_bg ?? '', theme) } as CSSProperties} onClick={() => { setColorId(null); setMenu(null); }}></button>
                                        {palette.map((c) => (
                                          <button className="ff-color-swatch" type="button" role="option" key={c.id} aria-label={c.name} aria-selected={colorId === c.id} style={{ '--data-chip-bg': chipBackground(c.bg, theme) } as CSSProperties} onClick={() => { setColorId(c.id); setMenu(null); }}></button>
                                        ))}
                                      </div>
                                    ) : null}
                                  </div>
                                </div>
                              </div>
                              <div className="ff-n323">
                                <div className="ff-n324">
                                  <span className="ff-n325">
                                    <div className="ff-n326">
                                      <span className="ff-n327"></span>
                                      <span className="ff-n328">
                                        <svg className="ff-n329">
                                          <path className="ff-n330" />
                                        </svg>
                                      </span>
                                      <a className="ff-n331" aria-label="This event follows the sharing settings of this calendar. Anyone who can see details of other events can also see this event's details, including the description and names of attachments. Learn more about event privacy"></a>
                                    </div>
                                  </span>
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                        <div className="ff-n332"></div>
                        <div className="ff-description-row">
                          <div className="ff-n334">
                            <div className="ff-n335">
                              <i className="ff-n336">
                                <span className="ff-n337">{"notes"}</span>
                              </i>
                            </div>
                            <div className="ff-n338">
                              <div className="ff-n339"></div>
                              <div className="ff-n340">
                                <div className="ff-n341">
                                  <div className="ff-n342">
                                    <div className="ff-n343">
                                      <div className="ff-toolbar" role="toolbar" aria-label="Formatting options">
                                        <div className="ff-n345">
                                          <span className="ff-n346">
                                            <button className="ff-drive-button" aria-label="Add a Google Drive attachment" type="button" disabled title={UNAVAILABLE}>
                                              <span className="ff-n348"></span>
                                              <span className="ff-n349">
                                                <i className="ff-n350">{"drive"}</i>
                                              </span>
                                              <div className="ff-n351"></div>
                                            </button>
                                            <div className="ff-n352" role="tooltip">{"Add a Google Drive attachment"}</div>
                                          </span>
                                        </div>
                                        <div className="ff-n353"></div>
                                        <div className="ff-bold" role="button" aria-label="Bold" aria-disabled="true" title={UNAVAILABLE}>
                                          <span className="ff-n355">
                                            <span className="ff-n356">
                                              <span className="ff-n357">{""}</span>
                                            </span>
                                          </span>
                                        </div>
                                        <div className="ff-italic" role="button" aria-label="Italic" aria-disabled="true" title={UNAVAILABLE}>
                                          <span className="ff-n359">
                                            <span className="ff-n360">
                                              <span className="ff-n361">{""}</span>
                                            </span>
                                          </span>
                                        </div>
                                        <div className="ff-underline" role="button" aria-label="Underline" aria-disabled="true" title={UNAVAILABLE}>
                                          <span className="ff-n363">
                                            <span className="ff-n364">
                                              <span className="ff-n365">{""}</span>
                                            </span>
                                          </span>
                                        </div>
                                        <div className="ff-n366"></div>
                                        <div className="ff-numbered" role="button" aria-label="Numbered list" aria-disabled="true" title={UNAVAILABLE}>
                                          <span className="ff-n368">
                                            <span className="ff-n369">
                                              <span className="ff-n370">{""}</span>
                                            </span>
                                          </span>
                                        </div>
                                        <div className="ff-bulleted" role="button" aria-label="Bulleted list" aria-disabled="true" title={UNAVAILABLE}>
                                          <span className="ff-n372">
                                            <span className="ff-n373">
                                              <span className="ff-n374">{""}</span>
                                            </span>
                                          </span>
                                        </div>
                                        <div className="ff-n375"></div>
                                        <div className="ff-link" role="button" aria-label="Insert link" aria-disabled="true" title={UNAVAILABLE}>
                                          <span className="ff-n377">
                                            <span className="ff-n378">
                                              <span className="ff-n379">{""}</span>
                                            </span>
                                          </span>
                                        </div>
                                        <div className="ff-clear" role="button" aria-label="Remove formatting" aria-disabled="true" title={UNAVAILABLE}>
                                          <span className="ff-n381">
                                            <span className="ff-n382">
                                              <span className="ff-n383">{""}</span>
                                            </span>
                                          </span>
                                        </div>
                                      </div>
                                      <span className="ff-n384">
                                        <div className="ff-n385">
                                          <span className="ff-n386"></span>
                                          <span className="ff-n387">
                                            <button className="ff-notes-button" type="button" disabled title={UNAVAILABLE}>
                                              <span className="ff-n389"></span>
                                              <div className="ff-n390">
                                                <svg className="ff-n391">
                                                  <path className="ff-n392" />
                                                  <path className="ff-n393" />
                                                </svg>
                                                <span className="ff-notes-label">{"Create meeting notes"}</span>
                                              </div>
                                            </button>
                                            <div className="ff-n395" role="tooltip">{"Create meeting notes"}</div>
                                          </span>
                                        </div>
                                        <div className="ff-n396">
                                          <ul className="ff-n397"></ul>
                                        </div>
                                      </span>
                                      <div className="ff-n398">
                                        <div className="ff-n399">{"Add description"}</div>
                                        <div className="ff-description-box" role="textbox" aria-label="Description" contentEditable suppressContentEditableWarning ref={descRef} onInput={(e) => setDescription((e.currentTarget as HTMLDivElement).innerText)}>
                                          <div className="ff-n401">
                                            <br className="ff-n402" />
                                          </div>
                                        </div>
                                      </div>
                                    </div>
                                  </div>
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
