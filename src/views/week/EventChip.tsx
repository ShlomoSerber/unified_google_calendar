import type { CSSProperties } from 'react';
import { hhmm } from '../../lib/dates';
import { chipBackground, useTheme } from '../../lib/colors';
import { useUi } from '../../state/ui';
import type { ViewOccurrence } from '../../types/ipc';

// Component 10 of docs/04 section 4. Google renders three shapes by chip height
// (docs/design/measurements/event_chip-{fifteen,thirty,forty_five}-light.json): a single
// line "Title, HH:MM" at 11px (tiny) or 12px (short), and title + time lines from 45 min.
// Classes mirror the dump nodes (chip-* for 60 min, chip-tiny-*, chip-short-*, chip-mid-* for 15, 30, 45).

export type ChipShape = 'tiny' | 'short' | 'mid' | 'long';

/** Rendered height in px → shape. Measured heights: 13 (15 min), 28 (30 min), 43 (45 min), 58 (60 min);
 *  the switch points between them are not observable and sit halfway. */
export function chipShape(heightPx: number): ChipShape {
  if (heightPx < 15) return 'tiny';
  if (heightPx < 34) return 'short';
  if (heightPx < 51) return 'mid';
  return 'long';
}

/** "10:00 to 11:00, Title, Calendar: X, No location, September 19, 2026" (Google's hidden description). */
export function chipDescription(o: ViewOccurrence, tz: string, calendarName: string, dateLabel: string): string {
  const title = o.title ?? '(No title)';
  const loc = o.location ?? 'No location';
  if (o.all_day) return `All day, ${title}, Calendar: ${calendarName}, ${loc}, ${dateLabel}`;
  const status = o.my_response === 'tentative' ? ', Tentative' : o.my_response === 'declined' ? ', Declined' : '';
  return `${hhmm(o.start, tz)} to ${hhmm(o.end, tz)}, ${title}${status}, Calendar: ${calendarName}, ${loc}, ${dateLabel}`;
}

export interface EventChipProps {
  occurrence: ViewOccurrence;
  /** Column geometry inside the chips layer, fractions of its width. */
  left: number;
  width: number;
  /** Column index in the overlap cluster: stacked chips (1+) get the outline and a higher z-index. */
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
  const heightPx = minutes - 2; // measured: minutes − chip_height_gap at 60px per hour
  const shape = chipShape(heightPx);
  const state = o.my_response === 'declined' ? 'declined' : o.my_response === 'tentative' ? 'tentative' : null;
  const title = o.title ?? '(No title)';
  const style = {
    '--data-chip-bg': chipBackground(o.color_bg, theme),
    '--data-column': column,
    left: `${left * 100}%`,
    width: `${width * 100}%`,
    top: `calc(var(--layout-hour-row-height) * ${topMinutes / 60} - var(--chip-root-margin-top))`,
    height: `calc(var(--layout-hour-row-height) * ${minutes / 60} - var(--layout-chip-height-gap))`,
  } as CSSProperties;
  const open = (e: React.MouseEvent<HTMLDivElement>) => {
    e.stopPropagation();
    useUi.getState().openDialog({ kind: 'event', occurrenceId: o.id, anchor: e.currentTarget.getBoundingClientRect() });
  };
  const stacked = column > 0 ? ' chip-stacked' : '';
  const cls = (base: string) => `${base}${state ? ` chip-${state}-${base.slice(5)}` : ''}${base === 'chip-root' ? stacked : ''}`;
  const sr = <div className="chip-sr">{chipDescription(o, tz, calendarName, dateLabel)}</div>;
  const resize = <div className={shape === 'long' ? 'chip-resize' : `chip-${shape}-resize`}></div>;

  if (shape === 'tiny' || shape === 'short') {
    const p = `chip-${shape}`;
    return (
      <div className={cls('chip-root') + ` ${p}-root`} role="button" tabIndex={0} data-title={title} style={style} onClick={open}>
        {sr}
        <div className={`${p}-body`}>
          <div className={`${p}-inner`}>
            <div className={`${p}-line`}>
              <span className={`${p}-span`}>
                <span className={`${p}-title`}>{title}</span>
                <span className={`${p}-comma`}>{',\u00a0'}</span>
                <span className={`${p}-time`}>{hhmm(o.start, tz)}</span>
              </span>
            </div>
          </div>
        </div>
        {resize}
      </div>
    );
  }
  if (shape === 'mid') {
    return (
      <div className={cls('chip-root') + ' chip-mid-root'} role="button" tabIndex={0} data-title={title} style={style} onClick={open}>
        {sr}
        <div className={cls('chip-body') + ' chip-mid-body'}>
          <div className="chip-mid-inner">
            <div className={cls('chip-title-box') + ' chip-mid-title-box'}>
              <span className="chip-mid-title-span">
                <span className={cls('chip-title') + ' chip-mid-title'}>{title}</span>
              </span>
            </div>
            <div className={cls('chip-time') + ' chip-mid-time'}>{`${hhmm(o.start, tz)} – ${hhmm(o.end, tz)}`}</div>
          </div>
        </div>
        {resize}
      </div>
    );
  }
  // With a location Google keeps the title on one line and adds it as a third line
  // (docs/design/measurements/event_chip-with_location-light.json: chip-loc-* nodes).
  const loc = o.location ? ' chip-loc' : '';
  return (
    <div className={cls('chip-root')} role="button" tabIndex={0} data-title={title} style={style} onClick={open}>
      {sr}
      <div className={cls('chip-body') + (loc && `${loc}-body`)}>
        <div className={`chip-inner${loc && `${loc}-inner`}`}>
          <div className={cls('chip-title-box') + (loc && `${loc}-title-box`)}>
            <span className={`chip-title-span${loc && `${loc}-title-span`}`}>
              <span className={cls('chip-title') + (loc && `${loc}-title`)}>{title}</span>
            </span>
          </div>
          <div className={cls('chip-time')}>{`${hhmm(o.start, tz)} – ${hhmm(o.end, tz)}`}</div>
          <div className={`chip-extra${loc && `${loc}-extra`}`}>{o.location ?? ''}</div>
        </div>
      </div>
      {resize}
    </div>
  );
}
