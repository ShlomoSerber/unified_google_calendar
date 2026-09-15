import { format } from 'date-fns';
import { dayAria } from '../../app/MiniCalendar';
import { inZone, isSameDay, monthGrid, toTs } from '../../lib/dates';
import { useUi } from '../../state/ui';
import './YearView.css';

// Component 23 (docs/design/measurements/year_view-light.json, docs/99 2026-09-15): twelve
// month grids of six rows, four per row, like Google's year view. Every class is a measured
// node. Clicking a day opens it in the day view (Google opens a list popup, out of scope).

const DOW = ['M', 'T', 'W', 'T', 'F', 'S', 'S'];
const DOW_NAMES = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];

export function YearView() {
  const date = useUi((s) => s.date);
  const tz = useUi((s) => s.tz);
  const now = useUi((s) => s.now);
  const year = inZone(date, tz).getFullYear();
  const openDay = (ts: number) => {
    const ui = useUi.getState();
    ui.setDate(ts + 12 * 3600);
    ui.setView('day');
  };
  return (
    <div className="year-root" role="main">
      <h1 className="year-sr">{`An overview showing all the days in the year ${year}`}</h1>
      <div className="year-page">
        <div className="year-grid">
          {Array.from({ length: 12 }, (_, m) => {
            const monthTs = toTs(new Date(year, m, 1, 12));
            const rows = monthGrid(monthTs, tz, 6);
            const name = format(new Date(year, m, 1), 'MMMM');
            return (
              <div className="year-month" key={m}>
                <div className="year-month-inner">
                  <div className="year-title-box">
                    <span className="year-title">{name}</span>
                  </div>
                  <table className="year-table" role="grid" aria-label={name}>
                    <thead className="year-thead">
                      <tr className="year-head-row">
                        {DOW.map((d, i) => (
                          <th className="year-th" key={i} aria-label={DOW_NAMES[i]}>
                            <span className="year-th-span">
                              <div className="year-dow">{d}</div>
                            </span>
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody className="year-tbody">
                      {rows.map((row) => (
                        <tr className="year-row" key={row[0]}>
                          {row.map((ts) => {
                            const today = isSameDay(ts, now, tz);
                            const kind = today ? '-today' : '';
                            return (
                              <td className="year-cell" key={ts}>
                                <button className={`year-day${kind}`} aria-label={dayAria(ts, monthTs, now, tz)} type="button" onClick={() => openDay(ts)}>
                                  <span className={`year-day${kind}-ripple ugc-state ugc-state-primary`}></span>
                                  <div className={`year-day${kind}-label`}>{inZone(ts, tz).getDate()}</div>
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
          })}
        </div>
      </div>
    </div>
  );
}
