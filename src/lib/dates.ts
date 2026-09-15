// Date helpers for the views. Weeks start on Monday, 24 h format (docs/01 R4.10).
// All timestamps are UTC seconds; rendering happens in the primary IANA zone.
import { TZDate } from '@date-fns/tz';
import { addDays, differenceInCalendarDays, startOfDay, startOfMonth, endOfMonth } from 'date-fns';

export type Tz = string;

export const DAY_SECONDS = 86_400;

/** A `Date` representing `ts` seconds in `tz`. */
export function inZone(ts: number, tz: Tz): TZDate {
  return new TZDate(ts * 1000, tz);
}

export function toTs(d: Date): number {
  return Math.floor(d.getTime() / 1000);
}

/** Local midnight of the day containing `ts`, as a timestamp. */
export function dayStart(ts: number, tz: Tz): number {
  return toTs(startOfDay(inZone(ts, tz)));
}

/** Monday 00:00 of the week containing `ts`. */
export function weekStart(ts: number, tz: Tz): number {
  const d = inZone(ts, tz);
  const dow = (d.getDay() + 6) % 7; // Monday = 0
  return toTs(startOfDay(addDays(d, -dow)));
}

/** Seven day starts from Monday. */
export function weekDays(ts: number, tz: Tz): number[] {
  const start = inZone(weekStart(ts, tz), tz);
  return Array.from({ length: 7 }, (_, i) => toTs(startOfDay(addDays(start, i))));
}

/** Rows of seven days covering the month of `ts`, Monday first: as many weeks as the month
 *  spans (four to six), like Google's month view; the mini calendar always shows six. */
export function monthGrid(ts: number, tz: Tz, rowCount?: number): number[][] {
  const first = startOfMonth(inZone(ts, tz));
  const last = endOfMonth(first);
  const gridStart = inZone(weekStart(toTs(first), tz), tz);
  const rows: number[][] = [];
  const needed = rowCount ?? Math.ceil((differenceInCalendarDays(last, gridStart) + 1) / 7);
  for (let r = 0; r < needed; r++) {
    rows.push(Array.from({ length: 7 }, (_, i) => toTs(startOfDay(addDays(gridStart, r * 7 + i)))));
  }
  return rows;
}

export function isSameDay(a: number, b: number, tz: Tz): boolean {
  return differenceInCalendarDays(inZone(a, tz), inZone(b, tz)) === 0;
}

/** `YYYY-MM-DD` of `ts` in `tz`. */
export function isoDate(ts: number, tz: Tz): string {
  const d = inZone(ts, tz);
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${d.getFullYear()}-${m}-${day}`;
}

/** Timestamp of local midnight for a `YYYY-MM-DD` in `tz`. */
export function fromIsoDate(date: string, tz: Tz): number {
  const [y, m, d] = date.split('-').map(Number);
  return toTs(new TZDate(y ?? 1970, (m ?? 1) - 1, d ?? 1, 0, 0, 0, tz));
}

/** `HH:MM` in `tz`, 24 h. */
export function hhmm(ts: number, tz: Tz): string {
  const d = inZone(ts, tz);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

/** Minutes since local midnight, fractional. */
export function minutesOfDay(ts: number, tz: Tz): number {
  const d = inZone(ts, tz);
  return d.getHours() * 60 + d.getMinutes() + d.getSeconds() / 60;
}

/** Offset in minutes between two zones at instant `ts` (secondary minus primary). */
export function zoneOffsetDiff(ts: number, primary: Tz, secondary: Tz): number {
  const a = new TZDate(ts * 1000, primary);
  const b = new TZDate(ts * 1000, secondary);
  return a.getTimezoneOffset() - b.getTimezoneOffset();
}

/** All-day span: number of calendar days an all-day occurrence covers (end exclusive). */
export function allDaySpanDays(start: number, end: number): number {
  return Math.max(1, Math.round((end - start) / DAY_SECONDS));
}

export function monthRange(ts: number, tz: Tz): { from: number; to: number } {
  const d = inZone(ts, tz);
  return { from: toTs(startOfMonth(d)), to: toTs(endOfMonth(d)) + 1 };
}
