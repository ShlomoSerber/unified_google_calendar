import { describe, expect, it } from 'vitest';
import type { ViewOccurrence } from '../../types/ipc';
import { layoutAllDay, layoutDay } from './layout';

const DAY = Date.UTC(2026, 8, 14) / 1000;
function occ(id: string, startMin: number, endMin: number, all_day = false): ViewOccurrence {
  return {
    id,
    event_id: id,
    calendar_id: 'c',
    account_id: 'a',
    title: id,
    start: DAY + startMin * 60,
    end: DAY + endMin * 60,
    all_day,
    color_bg: '#000',
    color_fg: '#fff',
    color_id: null,
    status: 'confirmed',
    my_response: null,
    is_recurring: false,
    has_meet: false,
    attendee_count: 0,
    conflict_with: null,
    also_in: [],
    is_local: false,
    transparency: null,
    location: null,
  };
}

describe('layoutDay', () => {
  it('places non-overlapping chips in one column each', () => {
    const p = layoutDay([occ('a', 540, 555), occ('b', 600, 630)], { dayStart: DAY, minMinutes: 15 });
    expect(p.map((x) => [x.occurrence.id, x.column, x.columns, x.top, x.height])).toEqual([
      ['a', 0, 1, 540, 15],
      ['b', 0, 1, 600, 30],
    ]);
  });

  it('two overlapping chips share two columns', () => {
    const p = layoutDay([occ('B', 630, 690), occ('A', 600, 660)], { dayStart: DAY, minMinutes: 15 });
    expect(p.map((x) => [x.occurrence.id, x.column, x.columns, x.span])).toEqual([
      ['A', 0, 2, 1],
      ['B', 1, 2, 1],
    ]);
  });

  it('three staggered chips form three columns', () => {
    const p = layoutDay([occ('A', 840, 900), occ('B', 855, 915), occ('C', 870, 930)], {
      dayStart: DAY,
      minMinutes: 15,
    });
    expect(p.map((x) => [x.occurrence.id, x.column, x.columns])).toEqual([
      ['A', 0, 3],
      ['B', 1, 3],
      ['C', 2, 3],
    ]);
  });

  it('a chip spans free columns to its right', () => {
    // A and B overlap; C starts after A ends but overlaps B, so C goes to column 0 and spans 1.
    // D overlaps nothing else in column 1 after B: D in column 0 spanning both columns? D is in a new cluster.
    const p = layoutDay([occ('A', 600, 660), occ('B', 630, 720), occ('C', 660, 700), occ('D', 800, 830)], {
      dayStart: DAY,
      minMinutes: 15,
    });
    const by = Object.fromEntries(p.map((x) => [x.occurrence.id, x]));
    expect(by.A?.column).toBe(0);
    expect(by.B?.column).toBe(1);
    expect(by.C?.column).toBe(0);
    expect(by.C?.span).toBe(1);
    expect(by.D?.columns).toBe(1);
  });

  it('short events get the minimum height and cross-midnight chips are clipped', () => {
    const p = layoutDay([occ('short', 600, 605), occ('late', 1400, 1500)], { dayStart: DAY, minMinutes: 15 });
    expect(p[0]?.height).toBe(15);
    expect(p[1]?.top).toBe(1400);
    expect(p[1]?.height).toBe(40);
  });
});

describe('layoutAllDay', () => {
  it('assigns rows without overlap and spans days', () => {
    const days = Array.from({ length: 7 }, (_, i) => DAY + i * 86_400);
    const one = occ('one', 0, 1440, true);
    const two = occ('two', 1440, 2 * 1440, true);
    const three = occ('three', 1440, 4 * 1440, true);
    const rows = layoutAllDay([one, two, three], days);
    const by = Object.fromEntries(rows.map((r) => [r.occurrence.id, r]));
    expect(by.one).toMatchObject({ startCol: 0, span: 1, row: 0 });
    expect(by.three).toMatchObject({ startCol: 1, span: 3, row: 0 });
    expect(by.two).toMatchObject({ startCol: 1, span: 1, row: 1 });
  });
});
