import { useEffect, useState } from 'react';
import { addMonths, format, parse } from 'date-fns';
import { inZone } from '../lib/dates';
import { useUi } from '../state/ui';
import './RecurrenceDialog.css';

// Component 18 of docs/04 section 4 (docs/design/measurements/recurrence_dialog-light.json):
// "Custom recurrence". Builds an RRULE (FREQ, INTERVAL, BYDAY, UNTIL or COUNT) that the form
// receives through the store (ui.pendingRrule).

type Freq = 'DAILY' | 'WEEKLY' | 'MONTHLY' | 'YEARLY';
const NEXT_FREQ: Record<Freq, Freq> = { DAILY: 'WEEKLY', WEEKLY: 'MONTHLY', MONTHLY: 'YEARLY', YEARLY: 'DAILY' };
const FREQ_LABEL: Record<Freq, { singular: string; plural: string }> = {
  DAILY: { singular: 'day', plural: 'days' },
  WEEKLY: { singular: 'week', plural: 'weeks' },
  MONTHLY: { singular: 'month', plural: 'months' },
  YEARLY: { singular: 'year', plural: 'years' },
};
const DAYS = [
  { code: 'MO', letter: 'M', name: 'Monday' },
  { code: 'TU', letter: 'T', name: 'Tuesday' },
  { code: 'WE', letter: 'W', name: 'Wednesday' },
  { code: 'TH', letter: 'T', name: 'Thursday' },
  { code: 'FR', letter: 'F', name: 'Friday' },
  { code: 'SA', letter: 'S', name: 'Saturday' },
  { code: 'SU', letter: 'S', name: 'Sunday' },
];
const DATE_FORMAT = 'MMM d, yyyy';

export interface RuleParts {
  freq: Freq;
  interval: number;
  byDay: string[];
  until: string | null;
  count: number | null;
}

/** Parse the RRULE parts the dialog edits; unknown parts are dropped. */
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

export function buildRule(p: RuleParts): string {
  const out = [`FREQ=${p.freq}`];
  if (p.interval > 1) out.push(`INTERVAL=${p.interval}`);
  if (p.freq === 'WEEKLY' && p.byDay.length) out.push(`BYDAY=${p.byDay.join(',')}`);
  if (p.until) out.push(`UNTIL=${p.until}`);
  else if (p.count) out.push(`COUNT=${p.count}`);
  return `RRULE:${out.join(';')}`;
}

export interface RecurrenceDialogProps {
  rrule: string | null;
  startTs: number;
}

export function RecurrenceDialog({ rrule, startTs }: RecurrenceDialogProps) {
  const tz = useUi((s) => s.tz);
  const startDay = DAYS[(inZone(startTs, tz).getDay() + 6) % 7]?.code ?? 'MO';
  const initial = parseRule(rrule, startDay);
  const [freq, setFreq] = useState<Freq>(initial.freq);
  const [interval, setInterval] = useState(initial.interval);
  const [byDay, setByDay] = useState<string[]>(initial.byDay);
  const [ends, setEnds] = useState<'never' | 'on' | 'after'>(initial.until ? 'on' : initial.count ? 'after' : 'never');
  const [count, setCount] = useState(initial.count ?? 13);
  // Google proposes an end date three months after the start.
  const [untilText, setUntilText] = useState(initial.until ? format(parse(initial.until.slice(0, 8), 'yyyyMMdd', new Date()), DATE_FORMAT) : format(addMonths(inZone(startTs, tz), 3), DATE_FORMAT));
  const close = () => useUi.getState().closeOverlay();
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopImmediatePropagation();
        close();
      }
    };
    document.addEventListener('keydown', onKey, true);
    return () => document.removeEventListener('keydown', onKey, true);
  }, []);
  const toggleDay = (code: string) => setByDay(byDay.includes(code) ? (byDay.length > 1 ? byDay.filter((d) => d !== code) : byDay) : [...byDay, code]);
  const done = () => {
    let until: string | null = null;
    if (ends === 'on') {
      const d = parse(untilText, DATE_FORMAT, new Date());
      if (!Number.isNaN(d.getTime())) until = `${format(d, 'yyyyMMdd')}T235959Z`;
    }
    const rule = buildRule({ freq, interval, byDay, until, count: ends === 'after' ? count : null });
    const ui = useUi.getState();
    ui.setPendingRrule(rule);
    ui.closeOverlay();
  };
  return (
    <div className="rec-scrim-page" onMouseDown={close}>
      <div className="rec-root" role="dialog" aria-modal="true" onMouseDown={(e) => e.stopPropagation()}>
        <span className="rec-scrim">
          <span className="rec-scrim-inner"></span>
        </span>
        <h2 className="rec-title-row">
          <span className="rec-title">{"Custom recurrence"}</span>
        </h2>
        <div className="rec-body">
          <div className="rec-form">
            <div className="rec-every-row">{"Repeat every"}
              <div className="rec-every-box">
                <div className="rec-every-field-wrap">
                  <span className="rec-n10">
                    <span className="rec-n11">
                      <div className="rec-every-field">
                        <div className="rec-every-line">
                          <span className="rec-every-span">
                            <input className="rec-every-input" aria-label={`${FREQ_LABEL[freq].plural[0]?.toUpperCase() ?? ''}${FREQ_LABEL[freq].plural.slice(1)} to repeat`} type="number" min={1} value={interval} onChange={(e) => setInterval(Math.max(1, Number(e.target.value) || 1))} />
                          </span>
                          <div className="rec-every-underline"></div>
                        </div>
                      </div>
                    </span>
                    <div className="rec-n17" role="tooltip"></div>
                  </span>
                </div>
                <div className="rec-every-spin">
                  <span className="rec-inc-span">
                    <button className="rec-inc" aria-label="Increment" type="button" onClick={() => setInterval(interval + 1)}>
                      <span className="rec-n21"></span>
                      <span className="rec-n22">
                        <span className="rec-n23">
                          <svg className="rec-n24">
                            <path className="rec-n25" />
                          </svg>
                        </span>
                      </span>
                      <div className="rec-n26"></div>
                    </button>
                    <div className="rec-n27" role="tooltip">{"Increment"}</div>
                  </span>
                  <span className="rec-dec-span">
                    <button className="rec-dec" aria-label="Decrement" type="button" onClick={() => setInterval(Math.max(1, interval - 1))}>
                      <span className="rec-n30"></span>
                      <span className="rec-n31">
                        <span className="rec-n32">
                          <svg className="rec-n33">
                            <path className="rec-n34" />
                          </svg>
                        </span>
                      </span>
                      <div className="rec-n35"></div>
                    </button>
                    <div className="rec-n36" role="tooltip">{"Decrement"}</div>
                  </span>
                </div>
              </div>
              <div className="rec-freq-wrap">
                <div className="rec-freq-box">
                  <div className="rec-freq-combo" role="combobox" aria-label="Frequency" tabIndex={0} onClick={() => setFreq(NEXT_FREQ[freq])}>
                    <span className="rec-freq-text-wrap">
                      <span className="rec-freq-text">{interval === 1 ? FREQ_LABEL[freq].singular : FREQ_LABEL[freq].plural}</span>
                    </span>
                    <span className="rec-freq-arrow">
                      <svg className="rec-n43" role="presentation" viewBox="0 0 24 24">
                        <polygon className="rec-n44" points="7,10 12,15 17,10"></polygon>
                        <polygon className="rec-n45" points="7,10 12,15 17,10"></polygon>
                      </svg>
                    </span>
                    <span className="rec-freq-n46"></span>
                    <div className="rec-freq-overlay"></div>
                  </div>
                </div>
              </div>
            </div>
            <div className="rec-on-block" hidden={freq !== 'WEEKLY'}>
              <div className="rec-on-label">{"Repeat on"}</div>
              <div className="rec-days">
                {DAYS.map((d) => (
                  <span className="rec-day-cell" key={d.code}>
                    <span className="rec-day-span">
                      <div className="rec-day-div">
                        <button className={byDay.includes(d.code) ? 'rec-day-on' : 'rec-day'} aria-label={d.name} aria-pressed={byDay.includes(d.code)} type="button" onClick={() => toggleDay(d.code)}>
                          <span className="rec-day-ripple">
                            <span className="rec-day-ripple-inner"></span>
                          </span>
                          <span className="rec-day-n57"></span>
                          <span className={byDay.includes(d.code) ? 'rec-day-on-letter' : 'rec-day-letter'}>{d.letter}</span>
                        </button>
                      </div>
                    </span>
                  </span>
                ))}
              </div>
            </div>
            <div className="rec-ends-label">{"Ends"}</div>
            <div className="rec-ends-group" role="radiogroup">
              <div className="rec-never-row">
                <div className="rec-never-radio-cell">
                  <div className="rec-never-radio">
                    <input className="rec-n126" aria-label="Recurrence never ends." type="radio" name="ends" checked={ends === 'never'} onChange={() => setEnds('never')} />
                    <div className="rec-never-ring">
                      <div className="rec-never-outer"></div>
                      {ends === 'never' ? <div className="rec-never-dot"></div> : null}
                    </div>
                    <span className="rec-never-ripple"></span>
                  </div>
                </div>
                <label className="rec-never-label">{"Never"}</label>
              </div>
              <div className="rec-on-row">
                <div className="rec-on-radio-cell">
                  <div className="rec-on-radio">
                    <input className="rec-n135" aria-label={`Recurrence ends on ${untilText}. Tab to change end date.`} type="radio" name="ends" checked={ends === 'on'} onChange={() => setEnds('on')} />
                    <div className="rec-on-ring">
                      <div className="rec-on-outer"></div>
                    {ends === 'on' ? <div className="rec-never-dot"></div> : null}
                    </div>
                    <span className="rec-on-ripple"></span>
                  </div>
                </div>
                <label className="rec-on-date-label">{"On"}</label>
                <div className="rec-on-date-wrap">
                  <div className="rec-n141">
                    <div className="rec-n142">
                      <span className="rec-n143">
                        <span className="rec-n144">
                          <div className="rec-on-date-field">
                            <div className="rec-on-date-line">
                              <span className="rec-on-date-span">
                                <input className="rec-on-date-input" aria-label="Date on which the recurrence ends" value={untilText} onChange={(e) => setUntilText(e.target.value)} onFocus={() => setEnds('on')} />
                              </span>
                              <div className="rec-on-date-underline"></div>
                            </div>
                          </div>
                        </span>
                        <div className="rec-n150" role="tooltip"></div>
                      </span>
                    </div>
                  </div>
                </div>
              </div>
              <div className="rec-after-row">
                <div className="rec-after-radio-cell">
                  <div className="rec-after-radio">
                    <input className="rec-n154" aria-label={`Recurrence ends after ${count} occurrences. Tab to change number of occurrences.`} type="radio" name="ends" checked={ends === 'after'} onChange={() => setEnds('after')} />
                    <div className="rec-after-ring">
                      <div className="rec-after-outer"></div>
                    {ends === 'after' ? <div className="rec-never-dot"></div> : null}
                    </div>
                    <span className="rec-after-ripple"></span>
                  </div>
                </div>
                <label className="rec-after-label">{"After"}</label>
                <div className="rec-after-wrap">
                  <div className="rec-after-box">
                    <span className="rec-n161">
                      <span className="rec-n162">
                        <div className="rec-after-field">
                          <div className="rec-after-line">
                            <span className="rec-after-span">
                              <input className="rec-after-input" aria-label="Occurrence count" type="number" min={1} value={count} onChange={(e) => setCount(Math.max(1, Number(e.target.value) || 1))} onFocus={() => setEnds('after')} />
                              <span className="rec-after-unit">{"occurrences"}</span>
                            </span>
                            <div className="rec-after-underline"></div>
                          </div>
                        </div>
                      </span>
                      <div className="rec-n169" role="tooltip"></div>
                    </span>
                  </div>
                  <div className="rec-after-spin">
                    <span className="rec-after-inc-span">
                      <button className="rec-after-inc" aria-label="Increment" type="button" onClick={() => { setEnds('after'); setCount(count + 1); }}>
                        <span className="rec-n173"></span>
                        <span className="rec-n174">
                          <span className="rec-n175">
                            <svg className="rec-n176">
                              <path className="rec-n177" />
                            </svg>
                          </span>
                        </span>
                        <div className="rec-n178"></div>
                      </button>
                      <div className="rec-n179" role="tooltip">{"Increment"}</div>
                    </span>
                    <span className="rec-after-dec-span">
                      <button className="rec-after-dec" aria-label="Decrement" type="button" onClick={() => { setEnds('after'); setCount(Math.max(1, count - 1)); }}>
                        <span className="rec-n182"></span>
                        <span className="rec-n183">
                          <span className="rec-n184">
                            <svg className="rec-n185">
                              <path className="rec-n186" />
                            </svg>
                          </span>
                        </span>
                        <div className="rec-n187"></div>
                      </button>
                      <div className="rec-n188" role="tooltip">{"Decrement"}</div>
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div className="rec-footer">
          <div className="rec-cancel-wrap">
            <button className="rec-cancel" type="button" onClick={close}>
              <span className="rec-cancel-ripple"></span>
              <span className="rec-cancel-hit"></span>
              <span className="rec-cancel-label">{"Cancel"}</span>
            </button>
          </div>
          <div className="rec-done-wrap">
            <button className="rec-done" type="button" onClick={done}>
              <span className="rec-done-ripple">
                <span className="rec-done-ripple-inner"></span>
              </span>
              <span className="rec-done-n199"></span>
              <span className="rec-done-hit"></span>
              <span className="rec-done-label">{"Done"}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
