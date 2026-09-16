import { format } from 'date-fns';
import { MonthGrid } from '../../components/MonthGrid';
import { inZone, toTs } from '../../lib/dates';
import { useUi } from '../../state/ui';
import './YearView.css';

// Year view (docs/11 section 7): twelve surface-container-low cards, each with the month name
// (title-medium) and a compact MonthGrid. Clicking a day opens it in the day view.
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
    <div className="year-view" role="main" aria-label={`Year ${year}`}>
      {Array.from({ length: 12 }, (_, m) => {
        // Noon of the first day, so the month is the same in every zone.
        const monthTs = toTs(new Date(year, m, 1, 12));
        const name = format(new Date(year, m, 1), 'MMMM');
        return (
          <section className="year-view-month" key={m} aria-label={`${name} ${year}`}>
            <h2 className="year-view-title md-typescale-title-medium">{name}</h2>
            <MonthGrid monthTs={monthTs} tz={tz} todayTs={now} size="compact" onPick={openDay} />
          </section>
        );
      })}
    </div>
  );
}
