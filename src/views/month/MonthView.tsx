import { useEffect, useRef, useState, type CSSProperties } from 'react';
import { format } from 'date-fns';
import { chipBackground, useTheme } from '../../lib/colors';
import { fromIsoDate, hhmm, inZone, isSameDay, isoDate, monthGrid } from '../../lib/dates';
import { useUi } from '../../state/ui';
import { useViewData } from '../../state/useViewData';
import { LAYOUT, layoutNumber } from '../../styles/layout';
import type { ViewOccurrence } from '../../types/ipc';
import { chipDescription } from '../week/EventChip';
import { layoutAllDay } from '../week/layout';
import './MonthView.css';

// Component 12 of docs/04 section 4 (docs/design/measurements/month_view-light.json). Six or
// five week rows; all-day chips are filled, timed chips show a dot, the time and the title;
// when a cell overflows, the last visible slot becomes "N more".

const DOW = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];

/** Chip slots that fit in the cell box below the day number (24px each); the last one turns into "more" on overflow. */
export function visibleSlots(cellHeight: number): number {
  const row = layoutNumber(LAYOUT.allday_chip_row_height);
  return Math.max(1, Math.floor(cellHeight / row));
}

interface CellItem {
  occurrence: ViewOccurrence;
  span: number;
  allDay: boolean;
  /** Slot index inside the cell (all-day rows come first, as in Google). */
  slot: number;
}

export function MonthView() {
  const date = useUi((s) => s.date);
  const tz = useUi((s) => s.tz);
  const now = useUi((s) => s.now);
  const calendars = useUi((s) => s.calendars);
  const theme = useTheme();
  const rows = monthGrid(date, tz);
  const from = rows[0]?.[0] ?? 0;
  const lastRow = rows[rows.length - 1] ?? [];
  const to = (lastRow[lastRow.length - 1] ?? 0) + 86_400;
  const { payload } = useViewData(from, to, tz);
  const month = inZone(date, tz).getMonth();
  const calendarName = (o: ViewOccurrence) => calendars.find((c) => c.account_id === o.account_id && c.id === o.calendar_id)?.summary ?? '';

  // How many chip slots fit: read from the rendered cell (rows differ between 5- and 6-week months).
  const firstCells = useRef<HTMLDivElement>(null);
  const [slots, setSlots] = useState(4);
  useEffect(() => {
    const el = firstCells.current;
    if (!el) return;
    const measure = () => setSlots(visibleSlots(el.getBoundingClientRect().height));
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const occurrences = payload?.occurrences ?? [];
  const hidden = new Set((payload?.calendars ?? []).filter((c) => !c.visible).map((c) => `${c.account_id}/${c.id}`));
  const visible = occurrences.filter((o) => !hidden.has(`${o.account_id}/${o.calendar_id}`));
  const allDayItems = visible
    .filter((o) => o.all_day || o.end - o.start >= 86_400)
    .map((o) => (o.all_day ? { ...o, start: fromIsoDate(isoDate(o.start, 'UTC'), tz), end: fromIsoDate(isoDate(o.end, 'UTC'), tz) } : o));
  const timedItems = visible.filter((o) => !o.all_day && o.end - o.start < 86_400).sort((a, b) => a.start - b.start);

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
    <div className="day-main month-main" role="main">
      <h1 className="day-main-sr month-main-sr">{`${format(inZone(date, tz), 'MMMM yyyy')}, ${visible.length} events`}</h1>
      <div className="day-main-box month-main-box">
        <div className="day-main-n3 month-main-n3"></div>
        <div className="month-root" role="grid">
          <div className="month-header-row" role="row">
            <div className="month-header-pres" role="presentation"></div>
            {DOW.map((name) => (
              <div className="month-header-col" role="columnheader" key={name}>
                <span className="month-header-col-sr">{name}</span>
                <span className="month-header-col-label">{name.slice(0, 3)}</span>
              </div>
            ))}
          </div>
          <div className="month-body" role="presentation">
            {rows.map((week, r) => {
              const weekEnd = (week[6] ?? 0) + 86_400;
              const spans = layoutAllDay(
                allDayItems.filter((o) => o.start < weekEnd && o.end > (week[0] ?? 0)),
                week,
              );
              // Per cell: all-day spans starting here (by row), then timed events by start.
              const cells: CellItem[][] = week.map((ts, col) => {
                const items: CellItem[] = spans
                  .filter((s) => s.startCol === col)
                  .map((s) => ({ occurrence: s.occurrence, span: s.span, allDay: true, slot: s.row }));
                const rowsUsed = Math.max(0, ...spans.filter((s) => s.startCol <= col && col < s.startCol + s.span).map((s) => s.row + 1));
                let slot = rowsUsed;
                for (const o of timedItems.filter((o) => o.start >= ts && o.start < ts + 86_400)) {
                  items.push({ occurrence: o, span: 1, allDay: false, slot });
                  slot++;
                }
                return items;
              });
              return (
                <div className="month-row" role="row" key={week[0]}>
                  <div className="month-row-bg">
                    {week.map((ts) => (
                      <div className="month-row-bg-cell" key={ts}></div>
                    ))}
                  </div>
                  <div className="month-row-nums">
                    {week.map((ts) => {
                      const d = inZone(ts, tz);
                      const today = isSameDay(ts, now, tz);
                      const first = d.getDate() === 1;
                      const other = d.getMonth() !== month;
                      const cls = today ? 'month-num-today' : other ? 'month-num-other' : first ? 'month-num-first' : 'month-num';
                      return (
                        <div className="month-num-cell" key={ts}>
                          <h2 className={cls} tabIndex={0} onClick={() => openDay(ts)}>
                            {first ? format(d, 'MMM d') : d.getDate()}
                          </h2>
                        </div>
                      );
                    })}
                  </div>
                  <div className="month-cells-box" ref={r === 0 ? firstCells : undefined}>
                    <div className="month-cells" role="presentation">
                      {week.map((ts, col) => {
                        const items = cells[col] ?? [];
                        const total = items.length;
                        const label = format(inZone(ts, tz), 'EEEE, MMMM d');
                        const sr = total === 0 ? `No events, ${label}` : `${total} event${total === 1 ? '' : 's'}, ${label}`;
                        const used = Math.max(0, ...items.map((it) => it.slot + 1));
                        const overflow = used > slots;
                        const shown = overflow ? items.filter((it) => it.slot < slots - 1) : items;
                        const hiddenCount = total - shown.length;
                        return (
                          <div className="month-cell" role="gridcell" key={ts}>
                            <h2 className="month-cell-sr">{sr}</h2>
                            {total > 0 ? (
                              <div className="month-cell-chips" role="presentation">
                                {shown.map((it) => {
                                  const o = it.occurrence;
                                  const title = o.title ?? '(No title)';
                                  const bg = chipBackground(o.color_bg, theme);
                                  const style = {
                                    '--data-chip-bg': bg,
                                    '--data-calendar-color': bg,
                                    top: `calc(var(--layout-allday-chip-row-height) * ${it.slot})`,
                                    left: `${(col / 7) * 100}%`,
                                    width: `${(it.span / 7) * 100}%`,
                                  } as CSSProperties;
                                  const desc = chipDescription(o, tz, calendarName(o), format(inZone(ts, tz), 'MMMM d, yyyy'));
                                  return it.allDay ? (
                                    <div className="month-chip-wrap" key={o.id} style={style}>
                                      <div className="month-chip" role="button" tabIndex={0} data-title={title} onClick={openEvent(o)}>
                                        <span className="month-chip-text-box">
                                          <span className="month-chip-text">{title}</span>
                                        </span>
                                        <span className="month-chip-sr">{desc}</span>
                                      </div>
                                    </div>
                                  ) : (
                                    <div className="month-timed-wrap" key={o.id} style={style}>
                                      <div className="month-timed" role="button" tabIndex={0} data-title={title} onClick={openEvent(o)}>
                                        <div className="month-timed-dot-box">
                                          <div className="month-timed-dot"></div>
                                        </div>
                                        <span className="month-timed-text">
                                          <span className="month-timed-time">{hhmm(o.start, tz)}</span>
                                          <span className="month-timed-title">{title}</span>
                                        </span>
                                        <span className="month-timed-sr">{desc}</span>
                                      </div>
                                    </div>
                                  );
                                })}
                                {overflow ? (
                                  <div className="month-more-wrap" role="presentation" style={{ top: `calc(var(--layout-allday-chip-row-height) * ${slots - 1})`, left: `${(col / 7) * 100}%`, width: `${100 / 7}%` }}>
                                    <div className="month-more" role="button" tabIndex={0} aria-label={`${hiddenCount} more events`} onClick={() => openDay(ts)}>
                                      {`${hiddenCount} more`}
                                    </div>
                                  </div>
                                ) : null}
                              </div>
                            ) : null}
                          </div>
                        );
                      })}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
