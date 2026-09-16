import { useEffect, useMemo, useRef, useState, type CSSProperties, type UIEvent } from 'react';
import { format } from 'date-fns';
import { dayStart, fromIsoDate, hhmm, inZone, isSameDay, isoDate, minutesOfDay } from '../../lib/dates';
import { useUi } from '../../state/ui';
import { useViewData } from '../../state/useViewData';
import { LAYOUT, layoutNumber } from '../../styles/layout';
import type { ViewOccurrence } from '../../types/ipc';
import { AllDayRow } from './AllDayRow';
import { EventChip } from './EventChip';
import { layoutDay } from './layout';
import { WeekHeader } from './WeekHeader';
import './WeekView.css';

// Week and day views (docs/11 section 7): a fixed top block (time-zone labels, day headers,
// all-day row) over a scroller with the hour gutter and the day columns. Hour lines and column
// borders are outline-variant hairlines; the now line is drawn in today's column only.

const HOURS = Array.from({ length: 24 }, (_, i) => i);
const CHIP_WIDTH_FACTOR = layoutNumber(LAYOUT.chip_width_factor);

/** "GMT-03": the long offset without the ":00" minutes. */
export function gmtLabel(ts: number, tz: string): string {
  const parts = new Intl.DateTimeFormat('en-US', { timeZone: tz, timeZoneName: 'longOffset' }).formatToParts(new Date(ts * 1000));
  const name = parts.find((p) => p.type === 'timeZoneName')?.value ?? 'GMT';
  return name === 'GMT' ? 'GMT+00' : name.replace(/:00$/, '');
}

/** Overlap columns: left = i/n; width = min(factor/n, 1 − i/n) (theme.json layout.chip_width_factor). */
export function chipColumn(column: number, columns: number): { left: number; width: number } {
  const left = column / columns;
  return { left, width: Math.min(CHIP_WIDTH_FACTOR / columns, 1 - left) };
}

export interface WeekViewProps {
  days: number[];
  /** `day`: one column. */
  mode?: 'week' | 'day';
}

export function WeekView({ days, mode = 'week' }: WeekViewProps) {
  const tz = useUi((s) => s.tz);
  const secondaryTz = useUi((s) => s.secondaryTz);
  const calendars = useUi((s) => s.calendars);
  const from = days[0] ?? 0;
  const to = (days[days.length - 1] ?? 0) + 86_400;
  const { payload, error } = useViewData(from, to, tz);
  const now = useUi((s) => s.now);

  const scroller = useRef<HTMLDivElement>(null);
  // The header casts a shadow only once the grid is scrolled.
  const [scrolled, setScrolled] = useState(false);
  const onScroll = (e: UIEvent<HTMLDivElement>) => {
    const isScrolled = e.currentTarget.scrollTop > 0;
    if (isScrolled !== scrolled) setScrolled(isScrolled);
  };
  useEffect(() => {
    // The grid opens at week_initial_scroll_hour; when the now line falls outside that window it
    // scrolls to the line minus one hour, clamped.
    const el = scroller.current;
    if (!el) return;
    const rowHeight = layoutNumber(LAYOUT.hour_row_height);
    const base = layoutNumber(LAYOUT.week_initial_scroll_hour) * rowHeight;
    const nowY = (minutesOfDay(Math.floor(Date.now() / 1000), tz) / 60) * rowHeight;
    const inWindow = nowY >= base && nowY <= base + el.clientHeight;
    el.scrollTop = inWindow ? base : Math.max(0, nowY - rowHeight);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- initial position only
  }, []);

  const calendarName = (o: ViewOccurrence) => calendars.find((c) => c.account_id === o.account_id && c.id === o.calendar_id)?.summary ?? '';
  const visible = useMemo(() => {
    const p = payload;
    if (!p) return [];
    const hidden = new Set(p.calendars.filter((c) => !c.visible).map((c) => `${c.account_id}/${c.id}`));
    return p.occurrences.filter((o) => !hidden.has(`${o.account_id}/${o.calendar_id}`));
  }, [payload]);
  // All-day occurrences carry UTC midnights (docs/03); the row lays them out on local days.
  const allDay = visible
    .filter((o) => o.all_day || o.end - o.start >= 86_400)
    .map((o) => (o.all_day ? { ...o, start: fromIsoDate(isoDate(o.start, 'UTC'), tz), end: fromIsoDate(isoDate(o.end, 'UTC'), tz) } : o));
  const timed = visible.filter((o) => !o.all_day && o.end - o.start < 86_400);

  const primaryHours = HOURS.map((h) => (h === 0 ? '' : `${String(h).padStart(2, '0')}:00`));
  const secondaryHours = secondaryTz ? HOURS.map((h) => (h === 0 ? '' : hhmm(from + h * 3600, secondaryTz))) : null;
  const title = mode === 'day' ? format(inZone(from, tz), 'EEEE, MMMM d, yyyy') : `Week of ${format(inZone(from, tz), 'MMMM d, yyyy')}`;

  return (
    <div className="week" role="main" aria-label={title} data-mode={mode} data-zones={secondaryTz ? '2' : '1'}>
      <div className="week-fixed" data-scrolled={scrolled}>
        <div className="week-tz md-typescale-label-small">
          {secondaryTz ? <span>{gmtLabel(from, secondaryTz)}</span> : null}
          <span>{gmtLabel(from, tz)}</span>
        </div>
        <WeekHeader days={days} now={now} />
        <div className="week-tz-allday"></div>
        <AllDayRow days={days} occurrences={allDay} calendarName={calendarName} />
      </div>
      <div className="week-scroll" ref={scroller} onScroll={onScroll} role="grid" aria-label={title}>
        <div className="week-grid" role="row">
          <div className="week-hours md-typescale-label-small">
            {HOURS.map((h) => (
              <div className="week-hour" key={h}>
                {secondaryHours ? <span>{secondaryHours[h]}</span> : null}
                <span>{primaryHours[h]}</span>
              </div>
            ))}
          </div>
          {days.map((ts) => {
            const start = dayStart(ts, tz);
            const items = timed.filter((o) => o.start < start + 86_400 && o.end > start);
            const placed = layoutDay(items, { dayStart: start, minMinutes: 15 });
            const today = isSameDay(ts, now, tz);
            const dateLabel = format(inZone(ts, tz), 'EEEE, MMMM d');
            const n = items.length;
            const nowTop = { top: `calc(var(--ugc-layout-hour-row-height) * ${minutesOfDay(now, tz) / 60})` } as CSSProperties;
            return (
              <div className="week-column" role="gridcell" key={ts} data-day={ts} aria-label={n === 0 ? `No events, ${dateLabel}` : `${n} event${n === 1 ? '' : 's'}, ${dateLabel}`}>
                {today ? <div className="week-now" style={nowTop}></div> : null}
                {placed.map((p) => {
                  const { left, width } = chipColumn(p.column, p.columns);
                  return (
                    <EventChip
                      key={p.occurrence.id}
                      occurrence={p.occurrence}
                      left={left}
                      width={width}
                      column={p.column}
                      topMinutes={p.top}
                      minutes={p.height}
                      calendarName={calendarName(p.occurrence)}
                      dateLabel={format(inZone(ts, tz), 'MMMM d, yyyy')}
                    />
                  );
                })}
              </div>
            );
          })}
        </div>
      </div>
      {error ? (
        <div className="week-error md-typescale-body-medium" role="alert">
          {`The calendar could not be loaded: ${error}`}
        </div>
      ) : null}
    </div>
  );
}
