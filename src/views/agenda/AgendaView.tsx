import { format } from 'date-fns';
import { chipBackground, useTheme } from '../../lib/colors';
import { dayStart, fromIsoDate, hhmm, inZone, isSameDay, isoDate } from '../../lib/dates';
import { useUi } from '../../state/ui';
import { useViewData } from '../../state/useViewData';
import type { ViewOccurrence } from '../../types/ipc';
import { chipDescription } from '../week/EventChip';
import './AgendaView.css';

// Component 13 of docs/04 section 4 (docs/design/measurements/agenda_view-light.json): one
// group per day with events, starting at the anchor date; 32px rows with time, title and the
// calendar dot; a red separator under today's group.

/** Days shown from the anchor: Google's schedule loads a rolling range; six weeks here. */
const AGENDA_DAYS = 42;

export function AgendaView() {
  const date = useUi((s) => s.date);
  const tz = useUi((s) => s.tz);
  const now = useUi((s) => s.now);
  const calendars = useUi((s) => s.calendars);
  const theme = useTheme();
  const from = dayStart(date, tz);
  const to = from + AGENDA_DAYS * 86_400;
  const { payload } = useViewData(from, to, tz);
  const calendarName = (o: ViewOccurrence) => calendars.find((c) => c.account_id === o.account_id && c.id === o.calendar_id)?.summary ?? '';

  const hidden = new Set((payload?.calendars ?? []).filter((c) => !c.visible).map((c) => `${c.account_id}/${c.id}`));
  const items = (payload?.occurrences ?? [])
    .filter((o) => !hidden.has(`${o.account_id}/${o.calendar_id}`))
    .map((o) => (o.all_day ? { ...o, start: fromIsoDate(isoDate(o.start, 'UTC'), tz), end: fromIsoDate(isoDate(o.end, 'UTC'), tz) } : o));
  // Group by local day; multi-day events appear on every day they cover.
  const groups: { day: number; items: ViewOccurrence[] }[] = [];
  for (let day = from; day < to; day += 86_400) {
    const list = items.filter((o) => o.start < day + 86_400 && o.end > day).sort((a, b) => Number(!a.all_day) - Number(!b.all_day) || a.start - b.start);
    if (list.length) groups.push({ day, items: list });
  }
  const openEvent = (o: ViewOccurrence) => (e: React.MouseEvent<HTMLDivElement>) => {
    e.stopPropagation();
    useUi.getState().openDialog({ kind: 'event', occurrenceId: o.id, anchor: e.currentTarget.getBoundingClientRect() });
  };
  const openDay = (ts: number) => {
    const ui = useUi.getState();
    ui.setDate(ts + 12 * 3600);
    ui.setView('day');
  };

  return (
    <div className="agenda-main" role="main">
      <h1 className="agenda-main-sr">{`Schedule starting ${format(inZone(from, tz), 'EEEE, MMMM d, yyyy')}`}</h1>
      <div className="agenda-main-box">
        <div className="agenda-root" role="grid">
          {groups.map(({ day, items: list }) => {
            const d = inZone(day, tz);
            const today = isSameDay(day, now, tz);
            const dayLabel = `${format(d, 'EEEE, MMMM d')}${today ? ', today' : ''}`;
            return (
              <div className="agenda-group" role="rowgroup" key={day}>
                {list.map((o, i) => {
                  const title = o.title ?? '(No title)';
                  const time = o.all_day ? 'All day' : `${hhmm(o.start, tz)} – ${hhmm(o.end, tz)}`;
                  const dot = { '--data-calendar-color': chipBackground(o.color_bg, theme) } as React.CSSProperties;
                  return (
                    <div className={i === 0 ? 'agenda-row-first' : 'agenda-row'} role="row" key={o.id}>
                      {i === 0 ? (
                        <div className="agenda-date-cell" role="gridcell">
                          <h2 className="agenda-date-h2">
                            <button className={today ? 'agenda-date-today' : 'agenda-date'} aria-label={dayLabel} type="button" onClick={() => openDay(day)}>
                              <span className={today ? 'agenda-date-today-ripple' : 'agenda-date-ripple'}></span>
                              <div className={today ? 'agenda-daynum-today' : 'agenda-daynum'}>{d.getDate()}</div>
                            </button>
                            <div className={today ? 'agenda-date-label-box' : 'agenda-date-label-box-other'}>
                              <div className={today ? 'agenda-date-label' : 'agenda-date-label-other'}>{format(d, 'MMM')}{`, ${format(d, 'EEE')}`}</div>
                            </div>
                          </h2>
                        </div>
                      ) : (
                        <div className="agenda-date-sr" role="gridcell">
                          {dayLabel}
                        </div>
                      )}
                      <div className={i === 0 ? 'agenda-pres' : 'agenda-pres-next'} role="presentation">
                        <div className="agenda-time-cell" role="gridcell">
                          {time}
                        </div>
                        <div className="agenda-title-cell" role="gridcell">
                          <div className="agenda-title" role="button" tabIndex={0} data-title={title} aria-label={chipDescription(o, tz, calendarName(o), format(d, 'MMMM d, yyyy'))} onClick={openEvent(o)}>
                            {title}
                          </div>
                          {o.location ? <div className="agenda-location">{o.location}</div> : null}
                        </div>
                        <div className="agenda-dot-cell" role="gridcell">
                          <div className="agenda-dot-box">
                            <div className="agenda-dot" style={dot}>
                              <span className="agenda-dot-sr">{`Calendar: ${calendarName(o)}`}</span>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  );
                })}
                {today ? <div className="agenda-separator"></div> : null}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
