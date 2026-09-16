import type { CSSProperties, MouseEvent } from 'react';
import { format } from 'date-fns';
import { inZone } from '../../lib/dates';
import { useTheme } from '../../lib/colors';
import { useUi } from '../../state/ui';
import type { ViewOccurrence } from '../../types/ipc';
import { layoutAllDay } from './layout';
import { chipDescription, chipStyle } from './EventChip';
import './AllDayRow.css';

// All-day and multi-day chips over the day columns (docs/11 section 7): 24 px tonal chips
// positioned in percent of the row, one row per overlap level.

export interface AllDayRowProps {
  days: number[];
  occurrences: ViewOccurrence[];
  calendarName: (o: ViewOccurrence) => string;
}

export function AllDayRow({ days, occurrences, calendarName }: AllDayRowProps) {
  const tz = useUi((s) => s.tz);
  const theme = useTheme();
  const rows = layoutAllDay(occurrences, days);
  const rowCount = Math.max(1, ...rows.map((r) => r.row + 1));
  const rootStyle = { height: `calc((var(--ugc-layout-allday-chip-height) + var(--ugc-layout-chip-gap)) * ${rowCount} + var(--ugc-layout-chip-gap))` } as CSSProperties;
  const open = (o: ViewOccurrence) => (e: MouseEvent<HTMLDivElement>) => {
    e.stopPropagation();
    useUi.getState().openDialog({ kind: 'event', occurrenceId: o.id, anchor: e.currentTarget.getBoundingClientRect() });
  };
  return (
    <div className="all-day-row" role="row" style={rootStyle}>
      {days.map((ts) => (
        <div className="all-day-cell" role="gridcell" key={ts} aria-label={`${format(inZone(ts, tz), 'EEEE, MMMM d')} all day`}></div>
      ))}
      {rows.map((r) => {
        const o = r.occurrence;
        const day = days[r.startCol] ?? days[0] ?? 0;
        const style: CSSProperties = {
          ...chipStyle(o.color_bg, theme),
          top: `calc((var(--ugc-layout-allday-chip-height) + var(--ugc-layout-chip-gap)) * ${r.row} + var(--ugc-layout-chip-gap))`,
          left: `calc(${(r.startCol / days.length) * 100}% + var(--ugc-layout-chip-gap))`,
          width: `calc(${(r.span / days.length) * 100}% - var(--ugc-layout-chip-gap) * 2)`,
        };
        return (
          <div className="all-day-chip md-typescale-label-medium" role="button" tabIndex={0} key={o.id} style={style} aria-label={chipDescription(o, tz, calendarName(o), format(inZone(day, tz), 'MMMM d, yyyy'))} onClick={open(o)}>
            <md-ripple></md-ripple>
            <md-focus-ring></md-focus-ring>
            <span className="all-day-chip-title">{o.title ?? '(No title)'}</span>
          </div>
        );
      })}
    </div>
  );
}
