import { useEffect, useRef, useState, type CSSProperties, type MouseEvent } from 'react';
import { format } from 'date-fns';
import { useTheme } from '../../lib/colors';
import { fromIsoDate, hhmm, inZone, isSameDay, isoDate, monthGrid } from '../../lib/dates';
import { useUi } from '../../state/ui';
import { useViewData } from '../../state/useViewData';
import { LAYOUT, layoutNumber } from '../../styles/layout';
import type { ViewOccurrence } from '../../types/ipc';
import { chipDescription, chipStyle } from '../week/EventChip';
import { layoutAllDay } from '../week/layout';
import { DayNumber } from '../../components/DayNumber';
import './MonthView.css';

// Month view (docs/11 section 7): cells with outline-variant borders, the day number in a
// 24 px pill (today: primary), all-day chips of 20 px, timed events as a dot plus text, and
// "N more" as a text button when a cell overflows.

const DOW = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];

/** Chip slots that fit below the day number; the last one turns into "more" on overflow. */
export function visibleSlots(cellHeight: number): number {
  const row = layoutNumber(LAYOUT.month_chip_height) + layoutNumber(LAYOUT.chip_gap);
  return Math.max(1, Math.floor((cellHeight - layoutNumber(LAYOUT.month_cell_header_height)) / row));
}

interface CellItem {
  occurrence: ViewOccurrence;
  span: number;
  allDay: boolean;
  /** Slot index inside the cell (all-day rows come first). */
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

  // How many chip slots fit: read from the rendered week (rows differ between 5- and 6-week months).
  const firstWeek = useRef<HTMLDivElement>(null);
  const [slots, setSlots] = useState(4);
  useEffect(() => {
    const el = firstWeek.current;
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

  const openEvent = (o: ViewOccurrence) => (e: MouseEvent<HTMLDivElement>) => {
    e.stopPropagation();
    useUi.getState().openDialog({ kind: 'event', occurrenceId: o.id, anchor: e.currentTarget.getBoundingClientRect() });
  };
  const openDay = (ts: number) => {
    const ui = useUi.getState();
    ui.setDate(ts + 12 * 3600);
    ui.setView('day');
  };
  const slotTop = (slot: number) => `calc(var(--ugc-layout-month-cell-header-height) + (var(--ugc-layout-month-chip-height) + var(--ugc-layout-chip-gap)) * ${slot})`;

  return (
    <div className="month-view" role="main" aria-label={`${format(inZone(date, tz), 'MMMM yyyy')}, ${visible.length} events`}>
      <div className="month-view-grid" role="grid" aria-label={format(inZone(date, tz), 'MMMM yyyy')}>
        <div className="month-view-head" role="row">
          {DOW.map((name) => (
            <div className="month-view-dow md-typescale-label-medium" role="columnheader" aria-label={name} key={name}>
              {name.slice(0, 3)}
            </div>
          ))}
        </div>
        {rows.map((week, r) => {
          const weekEnd = (week[6] ?? 0) + 86_400;
          const spans = layoutAllDay(
            allDayItems.filter((o) => o.start < weekEnd && o.end > (week[0] ?? 0)),
            week,
          );
          // Per cell: all-day spans starting here (by row), then timed events by start.
          const cells: CellItem[][] = week.map((ts, col) => {
            const items: CellItem[] = spans.filter((s) => s.startCol === col).map((s) => ({ occurrence: s.occurrence, span: s.span, allDay: true, slot: s.row }));
            const rowsUsed = Math.max(0, ...spans.filter((s) => s.startCol <= col && col < s.startCol + s.span).map((s) => s.row + 1));
            let slot = rowsUsed;
            for (const o of timedItems.filter((o) => o.start >= ts && o.start < ts + 86_400)) {
              items.push({ occurrence: o, span: 1, allDay: false, slot });
              slot++;
            }
            return items;
          });
          return (
            <div className="month-view-week" role="row" key={week[0]} ref={r === 0 ? firstWeek : undefined}>
              {week.map((ts, col) => {
                const d = inZone(ts, tz);
                const today = isSameDay(ts, now, tz);
                const first = d.getDate() === 1;
                const other = d.getMonth() !== month;
                const items = cells[col] ?? [];
                const total = items.length;
                const label = format(d, 'EEEE, MMMM d');
                const used = Math.max(0, ...items.map((it) => it.slot + 1));
                const overflow = used > slots;
                const shown = overflow ? items.filter((it) => it.slot < slots - 1) : items;
                const hiddenCount = total - shown.length;
                return (
                  <div className="month-view-cell" role="gridcell" key={ts} aria-label={total === 0 ? `No events, ${label}` : `${total} event${total === 1 ? '' : 's'}, ${label}`}>
                    <DayNumber size="month" typescale="md-typescale-label-large" className="month-view-number" label={`${label}${today ? ', today' : ''}`} today={today} muted={other} onClick={() => openDay(ts)}>
                      {first ? format(d, 'MMM d') : d.getDate()}
                    </DayNumber>
                    {shown.map((it) => {
                      const o = it.occurrence;
                      const title = o.title ?? '(No title)';
                      const style: CSSProperties = {
                        ...chipStyle(o.color_bg, theme),
                        top: slotTop(it.slot),
                        width: `calc(${it.span * 100}% - var(--ugc-layout-chip-gap) * 2)`,
                      };
                      const desc = chipDescription(o, tz, calendarName(o), format(d, 'MMMM d, yyyy'));
                      return it.allDay ? (
                        <div className="month-view-chip md-typescale-label-medium" role="button" tabIndex={0} key={o.id} style={style} aria-label={desc} onClick={openEvent(o)}>
                          <md-ripple></md-ripple>
                          <md-focus-ring></md-focus-ring>
                          <span className="month-view-text">{title}</span>
                        </div>
                      ) : (
                        <div className="month-view-timed md-typescale-label-medium" role="button" tabIndex={0} key={o.id} style={style} aria-label={desc} onClick={openEvent(o)}>
                          <md-ripple></md-ripple>
                          <md-focus-ring></md-focus-ring>
                          <span className="month-view-dot"></span>
                          <span className="month-view-text">{`${hhmm(o.start, tz)} ${title}`}</span>
                        </div>
                      );
                    })}
                    {overflow ? (
                      <button className="month-view-more md-typescale-label-medium" type="button" style={{ top: slotTop(slots - 1) }} aria-label={`${hiddenCount} more events, ${label}`} onClick={() => openDay(ts)}>
                        <md-ripple></md-ripple>
                        <md-focus-ring></md-focus-ring>
                        {`${hiddenCount} more`}
                      </button>
                    ) : null}
                  </div>
                );
              })}
            </div>
          );
        })}
      </div>
    </div>
  );
}
