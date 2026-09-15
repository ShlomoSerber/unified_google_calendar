import { format } from 'date-fns';
import { inZone, isSameDay } from '../../lib/dates';
import { useUi } from '../../state/ui';

// Day view header (component 11, docs/design/measurements/day_view-light.json nodes 19-29): the
// week header's row with a single column whose label block sits at the left.

export interface DayHeaderProps {
  day: number;
  now: number;
}

export function DayHeader({ day, now }: DayHeaderProps) {
  const tz = useUi((s) => s.tz);
  const d = inZone(day, tz);
  const today = isSameDay(day, now, tz);
  const aria = `${format(d, 'EEEE, MMMM d')}${today ? ', today' : ''}`;
  return (
    <div className="week-header-root day-header-row" role="row">
      <div className="week-header-pres day-header-pres" role="presentation">
        <div className="week-header-pad day-header-pad"></div>
        <div className="day-header-col" role="columnheader">
          <div className="week-header-col-line day-header-col-line"></div>
          <div className="day-header-label-box">
            <h2 className="day-header-h2" aria-label={aria}>
              <div className={today ? 'day-header-dow' : 'day-header-dow day-header-dow-other'}>{format(d, 'EEE')}</div>
              <div className={today ? 'day-header-daynum' : 'day-header-daynum day-header-daynum-other'} aria-label={aria}>
                {d.getDate()}
              </div>
            </h2>
          </div>
          <div className="day-header-spacer">
            <div className="day-header-spacer-inner"></div>
          </div>
        </div>
      </div>
    </div>
  );
}
