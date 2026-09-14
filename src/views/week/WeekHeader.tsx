import { format } from 'date-fns';
import { inZone, isSameDay } from '../../lib/dates';
import { useUi } from '../../state/ui';

// Component 6 of docs/04 section 4 (docs/design/measurements/week_header-light.json).
// Today's column uses the *-today node classes; the others share one set.

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
    <div className="week-header-root" role="row">
      <div className="week-header-pres" role="presentation">
        <div className="week-header-pad"></div>
        {days.map((ts) => {
          const d = inZone(ts, tz);
          const t = isSameDay(ts, now, tz) ? '-today' : '';
          const aria = dayHeaderAria(ts, tz, t !== '');
          return (
            <div className={`week-header-col${t}`} role="columnheader" key={ts}>
              <div className={`week-header-col-line${t}`}></div>
              <h2 className={`week-header-h2${t}`} aria-label={aria}>
                <div className={`week-header-dow${t}`}>{format(d, 'EEE')}</div>
                <button className={`week-header-daybtn${t}`} aria-label={aria} type="button" onClick={() => openDay(ts)}>
                  <span className={`week-header-daybtn${t}-ripple`}></span>
                  <div className={`week-header-daynum${t}`}>{d.getDate()}</div>
                </button>
              </h2>
            </div>
          );
        })}
      </div>
    </div>
  );
}
