import type { MouseEvent } from 'react';
import { format } from 'date-fns';
import { useTheme } from '../../lib/colors';
import { dayStart, fromIsoDate, hhmm, inZone, isSameDay, isoDate } from '../../lib/dates';
import { useUi } from '../../state/ui';
import { useViewData } from '../../state/useViewData';
import type { ViewOccurrence } from '../../types/ipc';
import { chipDescription, chipStyle } from '../week/EventChip';
import './AgendaView.css';

// Schedule view (docs/11 section 7): one group per day with events, starting at the anchor
// date. The date sits in a 40 px circle (today: primary); rows of 48 px carry the calendar's
// colour dot, the title (body-large) and the time (body-medium). A divider under today.

/** Days shown from the anchor: six weeks. */
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
  const openEvent = (o: ViewOccurrence) => (e: MouseEvent<HTMLDivElement>) => {
    e.stopPropagation();
    useUi.getState().openDialog({ kind: 'event', occurrenceId: o.id, anchor: e.currentTarget.getBoundingClientRect() });
  };
  const openDay = (ts: number) => {
    const ui = useUi.getState();
    ui.setDate(ts + 12 * 3600);
    ui.setView('day');
  };

  return (
    <div className="agenda-view" role="main" aria-label={`Schedule starting ${format(inZone(from, tz), 'EEEE, MMMM d, yyyy')}`}>
      {groups.length === 0 ? <p className="agenda-view-empty md-typescale-body-medium">No events in the next six weeks.</p> : null}
      {groups.map(({ day, items: list }) => {
        const d = inZone(day, tz);
        const today = isSameDay(day, now, tz);
        const dayLabel = `${format(d, 'EEEE, MMMM d')}${today ? ', today' : ''}`;
        return (
          <section className={today ? 'agenda-view-group agenda-view-today' : 'agenda-view-group'} key={day} aria-label={dayLabel}>
            <div className="agenda-view-date">
              <button className="agenda-view-number md-typescale-title-medium" type="button" aria-label={dayLabel} onClick={() => openDay(day)}>
                <md-ripple></md-ripple>
                <md-focus-ring></md-focus-ring>
                {d.getDate()}
              </button>
              <span className="agenda-view-month md-typescale-label-medium">{format(d, 'MMM, EEE')}</span>
            </div>
            <div className="agenda-view-rows" role="list">
              {list.map((o) => {
                const title = o.title ?? '(No title)';
                const time = o.all_day ? 'All day' : `${hhmm(o.start, tz)} – ${hhmm(o.end, tz)}`;
                return (
                  <div className="agenda-view-row" role="listitem" key={o.id}>
                    <div className="agenda-view-event" role="button" tabIndex={0} style={chipStyle(o.color_bg, theme)} aria-label={chipDescription(o, tz, calendarName(o), format(d, 'MMMM d, yyyy'))} onClick={openEvent(o)}>
                      <md-ripple></md-ripple>
                      <md-focus-ring></md-focus-ring>
                      <span className="agenda-view-dot"></span>
                      <span className="agenda-view-title md-typescale-body-large">{title}</span>
                      <span className="agenda-view-time md-typescale-body-medium">{time}</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </section>
        );
      })}
    </div>
  );
}
