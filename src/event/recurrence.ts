// The RRULE parts the custom recurrence dialog edits (FREQ, INTERVAL, BYDAY, UNTIL or COUNT).
// The backend expands the rules; the frontend only builds and parses them (docs/02).

export type Freq = 'DAILY' | 'WEEKLY' | 'MONTHLY' | 'YEARLY';

export interface RuleParts {
  freq: Freq;
  interval: number;
  byDay: string[];
  until: string | null;
  count: number | null;
}

/** Parse the RRULE parts the dialog edits; unknown parts are dropped, a `RRULE:` prefix is tolerated. */
export function parseRule(rrule: string | null, defaultDay: string): RuleParts {
  const parts: RuleParts = { freq: 'WEEKLY', interval: 1, byDay: [defaultDay], until: null, count: null };
  if (!rrule) return parts;
  for (const kv of rrule.replace(/^RRULE:/, '').split(';')) {
    const [k, v] = kv.split('=');
    if (!k || v === undefined) continue;
    if (k === 'FREQ' && (v === 'DAILY' || v === 'WEEKLY' || v === 'MONTHLY' || v === 'YEARLY')) parts.freq = v;
    if (k === 'INTERVAL') parts.interval = Math.max(1, Number(v) || 1);
    if (k === 'BYDAY') parts.byDay = v.split(',').map((d) => d.replace(/^-?\d+/, ''));
    if (k === 'UNTIL') parts.until = v;
    if (k === 'COUNT') parts.count = Number(v) || null;
  }
  return parts;
}

/** Build the RRULE line: BYDAY only for weekly rules, UNTIL wins over COUNT. */
export function buildRule(p: RuleParts): string {
  const out = [`FREQ=${p.freq}`];
  if (p.interval > 1) out.push(`INTERVAL=${p.interval}`);
  if (p.freq === 'WEEKLY' && p.byDay.length) out.push(`BYDAY=${p.byDay.join(',')}`);
  if (p.until) out.push(`UNTIL=${p.until}`);
  else if (p.count) out.push(`COUNT=${p.count}`);
  return `RRULE:${out.join(';')}`;
}
