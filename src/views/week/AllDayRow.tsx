import type { CSSProperties } from 'react';
import { format } from 'date-fns';
import { inZone } from '../../lib/dates';
import { chipBackground, useTheme } from '../../lib/colors';
import { useUi } from '../../state/ui';
import type { ViewOccurrence } from '../../types/ipc';
import { layoutAllDay } from './layout';
import { chipDescription } from './EventChip';

// Component 7 of docs/04 section 4 (docs/design/measurements/allday_row-light.json). Chips
// are absolutely positioned inside the cell of their first day and span whole columns.

export interface AllDayRowProps {
  days: number[];
  occurrences: ViewOccurrence[];
  calendarName: (o: ViewOccurrence) => string;
  mode?: 'week' | 'day';
}

export function AllDayRow({ days, occurrences, calendarName, mode = 'week' }: AllDayRowProps) {
  const c = (name: string) => (mode === 'day' ? `allday-${name} day-allday-${name}` : `allday-${name}`);
  const tz = useUi((s) => s.tz);
  const theme = useTheme();
  const rows = layoutAllDay(occurrences, days);
  const rowCount = Math.max(1, ...rows.map((r) => r.row + 1));
  const rootStyle = { height: `calc(var(--layout-allday-chip-row-height) * ${rowCount})` } as CSSProperties;
  const open = (o: ViewOccurrence) => (e: React.MouseEvent<HTMLDivElement>) => {
    e.stopPropagation();
    useUi.getState().openDialog({ kind: 'event', occurrenceId: o.id, anchor: e.currentTarget.getBoundingClientRect() });
  };
  return (
    <div className={c('root')} role="row" style={rootStyle}>
      <div className={c('pad')}></div>
      <div className={c('pres')} role="presentation">
        <ul className={c('ul')}>
          {days.map((ts) => (
            <li className={c('li')} key={ts}></li>
          ))}
        </ul>
        <div className={c('cells')} role="presentation">
          {days.map((ts, col) => {
            const mine = rows.filter((r) => r.startCol === col);
            const dateLabel = format(inZone(ts, tz), 'EEEE, MMMM d');
            const n = mine.length;
            const sr = n === 0 ? `No all day events, ${dateLabel}` : `${n} all day event${n === 1 ? '' : 's'}, ${dateLabel}`;
            if (n === 0) {
              return (
                <div className="allday-cell-empty" role="gridcell" key={ts}>
                  <h2 className="allday-cell-empty-sr">{sr}</h2>
                </div>
              );
            }
            return (
              <div className={c('cell')} role="gridcell" key={ts}>
                <h2 className={c('cell-sr')}>{sr}</h2>
                <div className={c('cell-chips')} role="presentation">
                  {mine.map((r) => {
                    const o = r.occurrence;
                    const title = o.title ?? '(No title)';
                    // Positioned against the whole row (the cells box), like Google's chips.
                    const style = {
                      '--data-chip-bg': chipBackground(o.color_bg, theme),
                      top: `calc(var(--layout-allday-chip-row-height) * ${r.row})`,
                      left: `${(r.startCol / days.length) * 100}%`,
                      width: `${(r.span / days.length) * 100}%`,
                    } as CSSProperties;
                    return (
                      <div className={c('chip-wrap')} key={o.id} style={style}>
                        <div className={c('chip')} role="button" tabIndex={0} data-title={title} onClick={open(o)}>
                          <span className={c('chip-text-box')}>
                            <span className={c('chip-text')}>{title}</span>
                          </span>
                          <span className={c('chip-sr')}>{chipDescription(o, tz, calendarName(o), format(inZone(ts, tz), 'MMMM d, yyyy'))}</span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
