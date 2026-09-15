import { describe, expect, it } from 'vitest';
import {
  allDaySpanDays,
  fromIsoDate,
  hhmm,
  isoDate,
  minutesOfDay,
  monthGrid,
  weekDays,
  weekStart,
  zoneOffsetDiff,
} from './dates';

const BA = 'America/Argentina/Buenos_Aires';
// 2026-09-16 (Wednesday) 15:30 in Buenos Aires = 18:30Z
const WED = Date.UTC(2026, 8, 16, 18, 30) / 1000;

describe('dates', () => {
  it('weeks start on Monday in the primary zone', () => {
    const start = weekStart(WED, BA);
    expect(isoDate(start, BA)).toBe('2026-09-14');
    expect(hhmm(start, BA)).toBe('00:00');
    const days = weekDays(WED, BA);
    expect(days.map((d) => isoDate(d, BA))).toEqual([
      '2026-09-14',
      '2026-09-15',
      '2026-09-16',
      '2026-09-17',
      '2026-09-18',
      '2026-09-19',
      '2026-09-20',
    ]);
    // A Sunday belongs to the week that started the previous Monday.
    const sun = Date.UTC(2026, 8, 20, 12) / 1000;
    expect(isoDate(weekStart(sun, BA), BA)).toBe('2026-09-14');
  });

  it('formats in 24 h and computes minutes of day', () => {
    expect(hhmm(WED, BA)).toBe('15:30');
    expect(minutesOfDay(WED, BA)).toBe(930);
    expect(hhmm(WED, 'America/Mexico_City')).toBe('12:30');
  });

  it('month grid has Monday-first rows covering the month', () => {
    const grid = monthGrid(WED, BA);
    expect(grid).toHaveLength(5); // September 2026 spans five Monday-first weeks (like Google's month view)
    expect(grid[0]?.map((d) => isoDate(d, BA))[0]).toBe('2026-08-31');
    expect(grid[4]?.map((d) => isoDate(d, BA))[6]).toBe('2026-10-04');
    const six = monthGrid(WED, BA, 6);
    expect(six).toHaveLength(6);
    expect(six[5]?.map((d) => isoDate(d, BA))[6]).toBe('2026-10-11');
  });

  it('iso date round trip and all-day spans', () => {
    const t = fromIsoDate('2026-09-14', BA);
    expect(isoDate(t, BA)).toBe('2026-09-14');
    expect(hhmm(t, BA)).toBe('00:00');
    expect(allDaySpanDays(0, 2 * 86_400)).toBe(2);
    expect(allDaySpanDays(0, 0)).toBe(1);
  });

  it('secondary zone offset', () => {
    expect(zoneOffsetDiff(WED, BA, 'America/Mexico_City')).toBe(-180);
    expect(zoneOffsetDiff(WED, BA, BA)).toBe(0);
  });
});
