import { useState } from 'react';
import { addMonths, format } from 'date-fns';
import { MonthGrid } from '../components/MonthGrid';
import { dayStart, inZone, toTs, weekDays } from '../lib/dates';
import { useUi } from '../state/ui';
import { Tooltip } from './Tooltip';
import './MiniCalendar.css';

// The navigation calendar of the drawer (docs/11 section 7): month title with two icon
// buttons over a MonthGrid. Today is primary; the visible week (or day) is primary-container.

export function MiniCalendar() {
  const date = useUi((s) => s.date);
  const view = useUi((s) => s.view);
  const tz = useUi((s) => s.tz);
  const setDate = useUi((s) => s.setDate);
  const todayTs = useUi((s) => s.now);
  // The month shown can differ from the view's anchor; it follows the anchor when that changes.
  const [shown, setShown] = useState({ anchor: date, month: date });
  const monthTs = shown.anchor === date ? shown.month : date;
  const monthLabel = format(inZone(monthTs, tz), 'MMMM yyyy');
  const shift = (n: number) => setShown({ anchor: date, month: toTs(addMonths(inZone(monthTs, tz), n)) });
  const days = weekDays(date, tz);
  const selected = view === 'week' || view === 'agenda' ? { from: days[0] ?? 0, to: (days[6] ?? 0) + 86_400 } : { from: dayStart(date, tz), to: dayStart(date, tz) + 86_400 };

  return (
    <div className="minical">
      <div className="minical-head">
        <span className="minical-month md-typescale-title-small">{monthLabel}</span>
        <Tooltip text="Previous month">
          <md-icon-button aria-label="Previous month" onclick={() => shift(-1)}>
            <md-icon>chevron_left</md-icon>
          </md-icon-button>
        </Tooltip>
        <Tooltip text="Next month">
          <md-icon-button aria-label="Next month" onclick={() => shift(1)}>
            <md-icon>chevron_right</md-icon>
          </md-icon-button>
        </Tooltip>
      </div>
      <MonthGrid monthTs={monthTs} tz={tz} todayTs={todayTs} selected={selected} size="normal" onPick={(ts) => setDate(ts + 12 * 3600)} />
    </div>
  );
}
