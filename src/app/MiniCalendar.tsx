import { useState } from 'react';
import { addMonths, format } from 'date-fns';
import { inZone, isSameDay, monthGrid, toTs } from '../lib/dates';
import { useUi } from '../state/ui';

// Component 4 of docs/04 section 4: the navigation calendar of the drawer. The DOM mirrors
// nodes 5-243 of docs/design/measurements/sidebar-light.json (classes sidebar-minical-*).

// Material Design chevrons (Apache 2.0), the glyphs Google inlines in the month buttons.
const CHEVRON_LEFT = 'M15.41 7.41L14 6l-6 6 6 6 1.41-1.41L10.83 12z';
const CHEVRON_RIGHT = 'M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z';

const DOW = [
  ['M', 'Monday'],
  ['T', 'Tuesday'],
  ['W', 'Wednesday'],
  ['T', 'Thursday'],
  ['F', 'Friday'],
  ['S', 'Saturday'],
  ['S', 'Sunday'],
] as const;

/** Google's day labels: "August 31, Monday" outside the month, "1, Tuesday" inside, "14, Monday, today". */
export function dayAria(ts: number, monthTs: number, todayTs: number, tz: string): string {
  const d = inZone(ts, tz);
  const sameMonth = d.getMonth() === inZone(monthTs, tz).getMonth() && d.getFullYear() === inZone(monthTs, tz).getFullYear();
  const base = sameMonth ? format(d, 'd, EEEE') : format(d, 'MMMM d, EEEE');
  return isSameDay(ts, todayTs, tz) ? `${base}, today` : base;
}

interface NavButtonProps {
  dir: 'prev' | 'next';
  onClick: () => void;
}

function NavButton({ dir, onClick }: NavButtonProps) {
  const label = dir === 'prev' ? 'Previous month' : 'Next month';
  return (
    <div className={`sidebar-minical-${dir}-cell`}>
      <span className={`sidebar-minical-${dir}-span`}>
        <button className={`sidebar-minical-${dir}`} aria-label={label} type="button" onClick={onClick}>
          <span className={`sidebar-minical-${dir}-ripple`}></span>
          <span className={`sidebar-minical-${dir}-icon-box`}>
            <span className={`sidebar-minical-${dir}-icon-span`}>
              <svg className={`sidebar-minical-${dir}-icon`} viewBox="0 0 24 24" focusable="false">
                <path className={`sidebar-minical-${dir}-path`} d={dir === 'prev' ? CHEVRON_LEFT : CHEVRON_RIGHT} />
              </svg>
            </span>
          </span>
          <div className={`sidebar-minical-${dir}-overlay`}></div>
        </button>
        <div className={`sidebar-minical-${dir}-tooltip`} role="tooltip">
          {label}
        </div>
      </span>
    </div>
  );
}

export function MiniCalendar() {
  const date = useUi((s) => s.date);
  const tz = useUi((s) => s.tz);
  const setDate = useUi((s) => s.setDate);
  const todayTs = useUi((s) => s.now);
  // The month shown can differ from the view's anchor; it follows the anchor when that changes.
  const [shown, setShown] = useState({ anchor: date, month: date });
  const monthTs = shown.anchor === date ? shown.month : date;
  const monthLabel = format(inZone(monthTs, tz), 'MMMM yyyy');
  const month = inZone(monthTs, tz).getMonth();
  const rows = monthGrid(monthTs, tz, 6);
  const shift = (n: number) => setShown({ anchor: date, month: toTs(addMonths(inZone(monthTs, tz), n)) });

  return (
    <div className="sidebar-minical">
      <h2 className="sidebar-minical-sr">Navigation calendar</h2>
      <div className="sidebar-minical-box">
        <div className="sidebar-minical-head">
          <span className="sidebar-minical-month">{monthLabel}</span>
          <div className="sidebar-minical-nav">
            <NavButton dir="prev" onClick={() => shift(-1)} />
            <NavButton dir="next" onClick={() => shift(1)} />
          </div>
        </div>
        <table className="sidebar-minical-grid" role="grid" aria-label={monthLabel}>
          <thead className="sidebar-minical-thead">
            <tr className="sidebar-minical-head-row">
              {DOW.map(([letter, name]) => (
                <th className="sidebar-minical-dow" key={name}>
                  <span className="sidebar-minical-dow-span">
                    <div className="sidebar-minical-dow-label">{letter}</div>
                    <div className="sidebar-minical-dow-tooltip" role="tooltip">
                      {name}
                    </div>
                  </span>
                  <span className="sidebar-minical-dow-sr">{name}</span>
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="sidebar-minical-tbody">
            {rows.map((row) => (
              <tr className="sidebar-minical-row" key={row[0]}>
                {row.map((ts) => {
                  const d = inZone(ts, tz);
                  const other = d.getMonth() !== month;
                  const today = isSameDay(ts, todayTs, tz);
                  const selected = !today && isSameDay(ts, date, tz);
                  const kind = today ? '-today' : selected ? '-selected' : other ? '-other' : '';
                  const cellKind = today ? '-today' : other ? '-other' : '';
                  return (
                    <td className={`sidebar-minical-cell${cellKind}`} key={ts}>
                      <button
                        className={`sidebar-minical-day${kind}`}
                        aria-label={dayAria(ts, monthTs, todayTs, tz)}
                        type="button"
                        onClick={() => setDate(ts + 12 * 3600)}
                      >
                        <span className={`sidebar-minical-day${kind}-ripple`}></span>
                        <div className={`sidebar-minical-day${kind}-label`}>{d.getDate()}</div>
                      </button>
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
