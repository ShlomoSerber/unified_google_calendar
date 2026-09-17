import { format } from 'date-fns';
import { inZone, isSameDay } from '../../lib/dates';
import { useUi } from '../../state/ui';
import { DayNumber } from '../../components/DayNumber';
import './WeekHeader.css';

// Day headers of the week and day views (docs/11 section 7): weekday name (label-medium,
// on-surface-variant) over the day number in a 40 px circle (title-large, brand typeface).
// Today: primary circle with on-primary number and a primary name. Clicking a day opens it.

export interface WeekHeaderProps {
  days: number[];
  now: number;
}

/** "Monday, September 14, today" */
export function dayHeaderAria(ts: number, tz: string, today: boolean): string {
  return `${format(inZone(ts, tz), 'EEEE, MMMM d')}${today ? ', today' : ''}`;
}

export function WeekHeader({ days, now }: WeekHeaderProps) {
  const tz = useUi((s) => s.tz);
  const openDay = (ts: number) => {
    const ui = useUi.getState();
    ui.setDate(ts + 12 * 3600);
    ui.setView('day');
  };
  return (
    <div className="week-header" role="row">
      {days.map((ts) => {
        const d = inZone(ts, tz);
        const today = isSameDay(ts, now, tz);
        const aria = dayHeaderAria(ts, tz, today);
        return (
          <div className={today ? 'week-header-day week-header-today' : 'week-header-day'} role="columnheader" aria-label={aria} key={ts}>
            <span className="week-header-name md-typescale-label-medium">{format(d, 'EEE')}</span>
            <DayNumber size="day" typescale="md-typescale-title-large" label={aria} today={today} onClick={() => openDay(ts)}>
              {d.getDate()}
            </DayNumber>
          </div>
        );
      })}
    </div>
  );
}
