import { format } from 'date-fns';
import { inZone, isSameDay, monthGrid } from '../lib/dates';
import { DayNumber } from './DayNumber';
import './MonthGrid.css';

// A month as seven columns of day buttons (docs/11 section 7): the mini calendar of the drawer,
// the date picker popover and, in `compact` size, the twelve cards of the year view. Today is
// primary on on-primary; the selected range is primary-container; days of another month are
// on-surface-variant. Each day is a `DayNumber`.

const DOW = [
  ['M', 'Monday'],
  ['T', 'Tuesday'],
  ['W', 'Wednesday'],
  ['T', 'Thursday'],
  ['F', 'Friday'],
  ['S', 'Saturday'],
  ['S', 'Sunday'],
] as const;

/** Day labels: "August 31, Monday" outside the month, "1, Tuesday" inside, "14, Monday, today". */
export function dayAria(ts: number, monthTs: number, todayTs: number, tz: string): string {
  const d = inZone(ts, tz);
  const sameMonth = d.getMonth() === inZone(monthTs, tz).getMonth() && d.getFullYear() === inZone(monthTs, tz).getFullYear();
  const base = sameMonth ? format(d, 'd, EEEE') : format(d, 'MMMM d, EEEE');
  return isSameDay(ts, todayTs, tz) ? `${base}, today` : base;
}

export interface MonthGridProps {
  /** Any instant inside the month to show. */
  monthTs: number;
  tz: string;
  todayTs: number;
  /** Highlighted range, `to` exclusive (the visible week, or the picked day). */
  selected?: { from: number; to: number };
  size: 'normal' | 'compact';
  onPick: (ts: number) => void;
}

export function MonthGrid({ monthTs, tz, todayTs, selected, size, onPick }: MonthGridProps) {
  const monthLabel = format(inZone(monthTs, tz), 'MMMM yyyy');
  const month = inZone(monthTs, tz).getMonth();
  const rows = monthGrid(monthTs, tz, 6);
  return (
    <div className="month-grid" role="grid" aria-label={monthLabel} data-size={size}>
      <div className="month-grid-row" role="row">
        {DOW.map(([letter, name]) => (
          <div className="month-grid-dow md-typescale-label-medium" role="columnheader" aria-label={name} key={name}>
            {letter}
          </div>
        ))}
      </div>
      {rows.map((row) => (
        <div className="month-grid-row" role="row" key={row[0]}>
          {row.map((ts) => {
            const d = inZone(ts, tz);
            const today = isSameDay(ts, todayTs, tz);
            const inRange = !today && selected !== undefined && ts >= selected.from && ts < selected.to;
            const other = d.getMonth() !== month;
            return (
              <div className="month-grid-cell" role="gridcell" key={ts}>
                <DayNumber size={size === 'compact' ? 'year' : 'mini'} typescale="md-typescale-label-medium" label={dayAria(ts, monthTs, todayTs, tz)} today={today} selected={inRange} muted={other} onClick={() => onPick(ts)}>
                  {d.getDate()}
                </DayNumber>
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
}
