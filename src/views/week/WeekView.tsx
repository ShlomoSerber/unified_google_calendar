import { useEffect, useMemo, useRef, type CSSProperties, type UIEvent } from 'react';
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

// Components 6-10 of docs/04 section 4 in one grid (docs/design/measurements/hour_grid-light.json).
// The DOM mirrors Google's: top block (time zone labels, day headers, all-day row) and a body
// whose hour gutter scrolls in sync with the day columns. Classes mirror the dump nodes.

const HOURS = Array.from({ length: 24 }, (_, i) => i);
const CHIP_WIDTH_FACTOR = layoutNumber(LAYOUT.chip_width_factor);

/** "GMT-03" as Google prints it: the long offset without the ":00" minutes. */
export function gmtLabel(ts: number, tz: string): string {
  const parts = new Intl.DateTimeFormat('en-US', { timeZone: tz, timeZoneName: 'longOffset' }).formatToParts(new Date(ts * 1000));
  const name = parts.find((p) => p.type === 'timeZoneName')?.value ?? 'GMT';
  return name === 'GMT' ? 'GMT+00' : name.replace(/:00$/, '');
}

/** Google's chip columns: left = i/n; width = min(factor/n, 1 − i/n) (docs/design/tokens.json layout.chip_width_factor). */
export function chipColumn(column: number, columns: number): { left: number; width: number } {
  const left = column / columns;
  return { left, width: Math.min(CHIP_WIDTH_FACTOR / columns, 1 - left) };
}

export interface WeekViewProps {
  days: number[];
}

export function WeekView({ days }: WeekViewProps) {
  const tz = useUi((s) => s.tz);
  const secondaryTz = useUi((s) => s.secondaryTz);
  const calendars = useUi((s) => s.calendars);
  const from = days[0] ?? 0;
  const to = (days[days.length - 1] ?? 0) + 86_400;
  const { payload, error } = useViewData(from, to, tz);
  const now = useUi((s) => s.now);

  const scroller = useRef<HTMLDivElement>(null);
  const gutter = useRef<HTMLDivElement>(null);
  const syncGutter = (e: UIEvent<HTMLDivElement>) => {
    if (gutter.current) gutter.current.scrollTop = e.currentTarget.scrollTop;
  };
  useEffect(() => {
    // Google opens the grid at 07:00 (layout.week_initial_scroll); when the now line falls outside
    // that window it scrolls to the line minus layout.week_scroll_now_margin, clamped.
    const el = scroller.current;
    if (!el) return;
    const base = layoutNumber(LAYOUT.week_initial_scroll);
    const nowY = (minutesOfDay(Math.floor(Date.now() / 1000), tz) / 60) * layoutNumber(LAYOUT.hour_row_height);
    const inWindow = nowY >= base && nowY <= base + el.clientHeight;
    el.scrollTop = inWindow ? base : Math.max(0, nowY - layoutNumber(LAYOUT.week_scroll_now_margin));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- initial position only, like Google
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
  const gutterCol = (labels: string[], cls: string) => (
    <div className={cls}>
      {labels.map((label, h) =>
        h === 0 ? (
          <div className="week-gutter-cell-first" key={h}></div>
        ) : (
          <div className="week-gutter-cell" key={h}>
            <span className="week-gutter-label">{label}</span>
          </div>
        ),
      )}
    </div>
  );

  return (
    <div className="week-root" role="grid">
      <div className="week-top">
        <div className="week-gutter-head">
          <div className="week-gutter-head-inner">
            <div className="week-tz-table">
              {secondaryTz ? (
                <div className="week-tz-cell">
                  <div className="week-tz-label">{gmtLabel(from, secondaryTz)}</div>
                </div>
              ) : null}
              <div className="week-tz-cell-primary">
                <div className="week-tz-label-primary">{gmtLabel(from, tz)}</div>
              </div>
            </div>
          </div>
        </div>
        <div className="week-top-right" role="presentation">
          <div className="week-allday-overlay">
            <div className="week-allday-overlay-inner">
              <div className="week-allday-overlay-pad"></div>
              <div className="week-allday-overlay-cols">
                {days.map((ts) => (
                  <div className="week-allday-overlay-col" key={ts}></div>
                ))}
              </div>
            </div>
          </div>
          <WeekHeader days={days} now={now} />
          <div className="week-header-bottom"></div>
          <AllDayRow days={days} occurrences={allDay} calendarName={calendarName} />
        </div>
      </div>
      <div className="week-body">
        <div className="week-body-inner">
          <div className="week-gutter" ref={gutter}>
            {secondaryHours ? gutterCol(secondaryHours, 'week-gutter-col') : null}
            {gutterCol(primaryHours, 'week-gutter-col-primary')}
          </div>
          <div className="week-scroller" ref={scroller} onScroll={syncGutter}>
            <div className="week-row" role="row">
              <div className="week-lines">
                {HOURS.map((h) => (
                  <div className="week-line" key={h}></div>
                ))}
              </div>
              <div className="week-row-pad"></div>
              {days.map((ts) => {
                const start = dayStart(ts, tz);
                const items = timed.filter((o) => o.start < start + 86_400 && o.end > start);
                const placed = layoutDay(items, { dayStart: start, minMinutes: 15 });
                const today = isSameDay(ts, now, tz);
                const dateLabel = format(inZone(ts, tz), 'EEEE, MMMM d');
                const n = items.length;
                const sr = n === 0 ? `No events, ${dateLabel}` : `${n} event${n === 1 ? '' : 's'}, ${dateLabel}`;
                const nowTop = { top: `calc(var(--layout-hour-row-height) * ${minutesOfDay(now, tz) / 60})` } as CSSProperties;
                return (
                  <div className="week-daycol" role="gridcell" key={ts} data-day={ts}>
                    <h2 className="week-daycol-sr">{sr}</h2>
                    {today ? <div className="week-now-line" style={nowTop}></div> : null}
                    {today ? <div className="week-now-dot" style={nowTop}></div> : null}
                    <div className="week-daycol-bg"></div>
                    <div className="week-chips">
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
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>
      {error ? <div className="week-error" role="alert">{`The calendar could not be loaded: ${error}`}</div> : null}
    </div>
  );
}
