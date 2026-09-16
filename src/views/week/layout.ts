// Column layout of overlapping timed chips in a day column.
// Overlapping chips form a cluster; within a cluster each chip takes the first free column;
// a chip's width spans to the next occupied column to its right. This is the classic
// Google Calendar layout; the overlap offset (chip_width_factor) is applied in WeekView
// through tokens, not here.
import type { ViewOccurrence } from '../../types/ipc';

export interface Placed {
  occurrence: ViewOccurrence;
  /** Column index inside the cluster. */
  column: number;
  /** Columns in the cluster. */
  columns: number;
  /** How many columns this chip may span (1 when a later chip sits to its right). */
  span: number;
  /** Minutes from midnight of the column day, clipped to the day. */
  top: number;
  /** Minutes, at least `minMinutes`. */
  height: number;
}

export interface LayoutOptions {
  /** Start of the day column in seconds. */
  dayStart: number;
  /** Minimum rendered duration in minutes. */
  minMinutes: number;
}

function overlaps(a: ViewOccurrence, b: ViewOccurrence): boolean {
  return a.start < b.end && b.start < a.end;
}

/** Lay out timed occurrences of one day. Input order does not matter. */
export function layoutDay(items: ViewOccurrence[], opts: LayoutOptions): Placed[] {
  const sorted = [...items]
    .filter((o) => !o.all_day)
    .sort((a, b) => a.start - b.start || b.end - b.start - (a.end - a.start) || a.id.localeCompare(b.id));
  const out: Placed[] = [];
  let cluster: { columns: ViewOccurrence[][]; placed: Placed[]; end: number } | null = null;

  const finish = () => {
    if (!cluster) return;
    const n = cluster.columns.length;
    for (const p of cluster.placed) {
      p.columns = n;
      let span = 1;
      for (let c = p.column + 1; c < n; c++) {
        const busy = cluster.columns[c]?.some((o) => overlaps(o, p.occurrence));
        if (busy) break;
        span++;
      }
      p.span = span;
      out.push(p);
    }
    cluster = null;
  };

  for (const o of sorted) {
    if (cluster && o.start >= cluster.end) finish();
    if (!cluster) cluster = { columns: [], placed: [], end: o.end };
    const c = cluster;
    let col = c.columns.findIndex((list) => !list.some((x) => overlaps(x, o)));
    if (col === -1) {
      col = c.columns.length;
      c.columns.push([]);
    }
    c.columns[col]?.push(o);
    c.end = Math.max(c.end, o.end);
    const startMin = Math.max(0, (o.start - opts.dayStart) / 60);
    const endMin = Math.min(24 * 60, (o.end - opts.dayStart) / 60);
    c.placed.push({
      occurrence: o,
      column: col,
      columns: 1,
      span: 1,
      top: startMin,
      height: Math.max(opts.minMinutes, endMin - startMin),
    });
  }
  finish();
  return out;
}

/** All-day (and multi-day) chips: rows so that overlapping spans never share a row. */
export interface AllDayRow {
  occurrence: ViewOccurrence;
  /** First day column (0..6) and number of columns covered inside the visible week. */
  startCol: number;
  span: number;
  row: number;
}

export function layoutAllDay(items: ViewOccurrence[], dayStarts: number[]): AllDayRow[] {
  const first = dayStarts[0] ?? 0;
  const last = (dayStarts[dayStarts.length - 1] ?? 0) + 86_400;
  const spans = items
    .filter((o) => (o.all_day || o.end - o.start >= 86_400) && o.start < last && o.end > first)
    .map((o) => {
      const s = Math.max(o.start, first);
      const e = Math.min(o.end, last);
      const startCol = dayStarts.findIndex((d, i) => s >= d && s < (dayStarts[i + 1] ?? last));
      const endCol = dayStarts.findIndex((d, i) => e > d && e <= (dayStarts[i + 1] ?? last));
      return { occurrence: o, startCol: Math.max(0, startCol), span: Math.max(1, endCol - startCol + 1) };
    })
    .sort((a, b) => a.startCol - b.startCol || b.span - a.span || a.occurrence.id.localeCompare(b.occurrence.id));
  const rows: boolean[][] = [];
  const out: AllDayRow[] = [];
  for (const sp of spans) {
    let row = 0;
    for (;;) {
      const r = rows[row] ?? (rows[row] = Array<boolean>(dayStarts.length).fill(false));
      let free = true;
      for (let c = sp.startCol; c < sp.startCol + sp.span; c++) if (r[c]) free = false;
      if (free) {
        for (let c = sp.startCol; c < sp.startCol + sp.span; c++) r[c] = true;
        break;
      }
      row++;
    }
    out.push({ ...sp, row });
  }
  return out;
}
