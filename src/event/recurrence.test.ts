import { describe, expect, it } from 'vitest';
import { buildRule, parseRule } from './recurrence';

describe('parseRule', () => {
  it('defaults to weekly on the given day', () => {
    expect(parseRule(null, 'TU')).toEqual({ freq: 'WEEKLY', interval: 1, byDay: ['TU'], until: null, count: null });
  });
  it('reads daily, monthly and yearly rules with an interval, tolerating the RRULE: prefix', () => {
    expect(parseRule('RRULE:FREQ=DAILY;INTERVAL=3', 'MO').freq).toBe('DAILY');
    expect(parseRule('FREQ=DAILY;INTERVAL=3', 'MO').interval).toBe(3);
    expect(parseRule('RRULE:FREQ=MONTHLY;BYDAY=2WE', 'MO')).toMatchObject({ freq: 'MONTHLY', byDay: ['WE'] });
    expect(parseRule('RRULE:FREQ=YEARLY', 'MO').freq).toBe('YEARLY');
  });
  it('reads weekly BYDAY lists, COUNT and UNTIL', () => {
    expect(parseRule('RRULE:FREQ=WEEKLY;BYDAY=TU,TH;COUNT=5', 'MO')).toEqual({ freq: 'WEEKLY', interval: 1, byDay: ['TU', 'TH'], until: null, count: 5 });
    expect(parseRule('RRULE:FREQ=WEEKLY;UNTIL=20261231T235959Z', 'MO').until).toBe('20261231T235959Z');
  });
  it('drops unknown parts and bad values', () => {
    expect(parseRule('RRULE:FREQ=HOURLY;INTERVAL=x;WKST=MO;COUNT=abc', 'FR')).toEqual({ freq: 'WEEKLY', interval: 1, byDay: ['FR'], until: null, count: null });
  });
});

describe('buildRule', () => {
  it('writes the parts in order and omits the defaults', () => {
    expect(buildRule({ freq: 'DAILY', interval: 1, byDay: ['MO'], until: null, count: null })).toBe('RRULE:FREQ=DAILY');
    expect(buildRule({ freq: 'WEEKLY', interval: 2, byDay: ['TU', 'TH'], until: null, count: 5 })).toBe('RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,TH;COUNT=5');
  });
  it('keeps BYDAY for weekly rules only and prefers UNTIL over COUNT', () => {
    expect(buildRule({ freq: 'MONTHLY', interval: 1, byDay: ['TU'], until: null, count: null })).toBe('RRULE:FREQ=MONTHLY');
    expect(buildRule({ freq: 'YEARLY', interval: 1, byDay: [], until: '20261231T235959Z', count: 3 })).toBe('RRULE:FREQ=YEARLY;UNTIL=20261231T235959Z');
  });
  it('round-trips through parseRule', () => {
    const rule = 'RRULE:FREQ=WEEKLY;INTERVAL=3;BYDAY=MO,WE,FR;UNTIL=20270101T235959Z';
    expect(buildRule(parseRule(rule, 'SU'))).toBe(rule);
  });
});
