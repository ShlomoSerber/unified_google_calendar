import type { CSSProperties, MouseEvent } from 'react';
import { hhmm } from '../../lib/dates';
import { chipColors, useTheme } from '../../lib/colors';
import { useUi } from '../../state/ui';
import { EVENT_COLOR_NAMES } from '../../styles/palette';
import type { ViewOccurrence } from '../../types/ipc';
import './EventChip.css';

// A timed event in a day column (docs/11 section 7): tonal chip with the calendar's colour on
// the left border, title (label-medium) and time (body-small); with 30 minutes or less only the
// title. Declined events are struck through on a half-opacity container; tentative ones get a
// dashed border. Chips stacked over another (column > 0) rise with a level-1 shadow.

/** "10:00 to 11:00, Title, Calendar: X, No location, September 19, 2026" (the chip's accessible description). */
export function chipDescription(o: ViewOccurrence, tz: string, calendarName: string, dateLabel: string): string {
  const title = o.title ?? '(No title)';
  const loc = o.location ? `Location: ${o.location}` : 'No location';
  const color = o.color_id && EVENT_COLOR_NAMES[o.color_id] ? `, Color: ${EVENT_COLOR_NAMES[o.color_id]}` : '';
  if (o.all_day) return `All day, ${title}, Calendar: ${calendarName}, ${loc}${color}, ${dateLabel}`;
  return `${hhmm(o.start, tz)} to ${hhmm(o.end, tz)}, ${title}, Calendar: ${calendarName}, ${loc}${color}, ${dateLabel}`;
}

/** Inline style with the chip's colour roles as --data-chip-* (docs/11 section 6). */
export function chipStyle(hex: string, theme: 'light' | 'dark'): CSSProperties {
  const c = chipColors(hex, theme);
  return { '--data-chip-color': c.color, '--data-chip-container': c.container, '--data-chip-on-container': c.onContainer } as CSSProperties;
}

export interface EventChipProps {
  occurrence: ViewOccurrence;
  /** Column geometry inside the chips layer, fractions of its width. */
  left: number;
  width: number;
  /** Column index in the overlap cluster: stacked chips (1+) rise over the ones to their left. */
  column: number;
  /** Minutes from the top of the day column and rendered minutes. */
  topMinutes: number;
  minutes: number;
  calendarName: string;
  dateLabel: string;
}

export function EventChip({ occurrence: o, left, width, column, topMinutes, minutes, calendarName, dateLabel }: EventChipProps) {
  const tz = useUi((s) => s.tz);
  const theme = useTheme();
  const title = o.title ?? '(No title)';
  const state = o.my_response === 'declined' ? ' event-chip-declined' : o.my_response === 'tentative' ? ' event-chip-tentative' : '';
  const style: CSSProperties = {
    ...chipStyle(o.color_bg, theme),
    left: `calc(${left * 100}% + var(--ugc-layout-chip-gap))`,
    width: `calc(${width * 100}% - var(--ugc-layout-chip-gap) * 2)`,
    top: `calc(var(--ugc-layout-hour-row-height) * ${topMinutes / 60})`,
    height: `calc(var(--ugc-layout-hour-row-height) * ${minutes / 60} - var(--ugc-layout-chip-gap))`,
  };
  const open = (e: MouseEvent<HTMLDivElement>) => {
    e.stopPropagation();
    useUi.getState().openDialog({ kind: 'event', occurrenceId: o.id, anchor: e.currentTarget.getBoundingClientRect() });
  };
  const cls = `event-chip${minutes <= 30 ? ' event-chip-short' : ''}${column > 0 ? ' event-chip-stacked' : ''}${state}`;
  return (
    <div className={cls} role="button" tabIndex={0} aria-label={chipDescription(o, tz, calendarName, dateLabel)} style={style} onClick={open}>
      <md-ripple></md-ripple>
      <md-focus-ring></md-focus-ring>
      <div className="event-chip-title md-typescale-label-medium">{title}</div>
      <div className="event-chip-time md-typescale-body-small">{`${hhmm(o.start, tz)} – ${hhmm(o.end, tz)}`}</div>
    </div>
  );
}
