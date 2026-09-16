import { useEffect, useRef, useState } from 'react';
import { addMonths, format, parse } from 'date-fns';
import type { MdDialog } from '@material/web/dialog/dialog.js';
import type { MdOutlinedSelect } from '@material/web/select/outlined-select.js';
import type { MdOutlinedTextField } from '@material/web/textfield/outlined-text-field.js';
import { fromIsoDate, inZone, toTs } from '../lib/dates';
import { useUi } from '../state/ui';
import { DatePicker } from './DatePicker';
import { buildRule, parseRule, type Freq } from './recurrence';
import './RecurrenceDialog.css';

// "Custom recurrence" (docs/11 section 7): repeat every N day/week/month/year, the weekdays as
// filter chips for weekly rules, and an end (never, on a date, after N occurrences). The rule
// reaches the open form through the store (ui.pendingRrule).

const FREQ_LABEL: Record<Freq, { singular: string; plural: string }> = {
  DAILY: { singular: 'day', plural: 'days' },
  WEEKLY: { singular: 'week', plural: 'weeks' },
  MONTHLY: { singular: 'month', plural: 'months' },
  YEARLY: { singular: 'year', plural: 'years' },
};
const FREQS: Freq[] = ['DAILY', 'WEEKLY', 'MONTHLY', 'YEARLY'];
const DAYS = [
  { code: 'MO', letter: 'M', name: 'Monday' },
  { code: 'TU', letter: 'T', name: 'Tuesday' },
  { code: 'WE', letter: 'W', name: 'Wednesday' },
  { code: 'TH', letter: 'T', name: 'Thursday' },
  { code: 'FR', letter: 'F', name: 'Friday' },
  { code: 'SA', letter: 'S', name: 'Saturday' },
  { code: 'SU', letter: 'S', name: 'Sunday' },
];

const numberValue = (e: Event, min: number) => Math.max(min, Number((e.target as MdOutlinedTextField).value) || min);

export interface RecurrenceDialogProps {
  rrule: string | null;
  startTs: number;
}

export function RecurrenceDialog({ rrule, startTs }: RecurrenceDialogProps) {
  const tz = useUi((s) => s.tz);
  const dialog = useRef<MdDialog>(null);
  const startDay = DAYS[(inZone(startTs, tz).getDay() + 6) % 7]?.code ?? 'MO';
  const initial = parseRule(rrule, startDay);
  const [freq, setFreq] = useState<Freq>(initial.freq);
  const [interval, setInterval] = useState(initial.interval);
  const [byDay, setByDay] = useState<string[]>(initial.byDay);
  const [ends, setEnds] = useState<'never' | 'on' | 'after'>(initial.until ? 'on' : initial.count ? 'after' : 'never');
  const [count, setCount] = useState(initial.count ?? 13);
  // The proposed end date is three months after the start.
  const [untilTs, setUntilTs] = useState(initial.until ? fromIsoDate(format(parse(initial.until.slice(0, 8), 'yyyyMMdd', new Date()), 'yyyy-MM-dd'), tz) + 12 * 3600 : toTs(addMonths(inZone(startTs, tz), 3)));
  const close = () => dialog.current?.close();
  useEffect(() => {
    dialog.current?.show();
  }, []);
  // The list never empties: a click that would remove the last day is reverted by the chip.
  const toggleDay = (code: string) => (e: Event) => {
    if (byDay.includes(code)) {
      if (byDay.length > 1) setByDay(byDay.filter((d) => d !== code));
      else e.preventDefault();
    } else {
      setByDay([...byDay, code]);
    }
  };
  const done = () => {
    const until = ends === 'on' ? `${format(inZone(untilTs, tz), 'yyyyMMdd')}T235959Z` : null;
    const rule = buildRule({ freq, interval, byDay, until, count: ends === 'after' ? count : null });
    useUi.getState().setPendingRrule(rule);
    close();
  };
  return (
    <md-dialog className="recurrence" ref={dialog} aria-label="Custom recurrence" onclosed={() => useUi.getState().closeOverlay()}>
      <div slot="headline">Custom recurrence</div>
      <div slot="content" className="recurrence-content">
        <div className="recurrence-row">
          <span className="md-typescale-body-medium">Repeat every</span>
          <md-outlined-text-field className="recurrence-number" label="Interval" type="number" min="1" value={String(interval)} oninput={(e) => setInterval(numberValue(e, 1))}></md-outlined-text-field>
          <md-outlined-select label="Frequency" value={freq} onchange={(e) => setFreq((e.target as MdOutlinedSelect).value as Freq)}>
            {FREQS.map((f) => (
              <md-select-option value={f} key={f}>
                <div slot="headline">{interval === 1 ? FREQ_LABEL[f].singular : FREQ_LABEL[f].plural}</div>
              </md-select-option>
            ))}
          </md-outlined-select>
        </div>
        {freq === 'WEEKLY' ? (
          <div className="recurrence-block">
            <span className="md-typescale-body-medium">Repeat on</span>
            <md-chip-set aria-label="Repeat on">
              {DAYS.map((d) => (
                <md-filter-chip key={d.code} label={d.letter} aria-label={d.name} selected={byDay.includes(d.code)} onclick={toggleDay(d.code)}></md-filter-chip>
              ))}
            </md-chip-set>
          </div>
        ) : null}
        <div className="recurrence-block" role="radiogroup" aria-label="Ends">
          <span className="md-typescale-body-medium">Ends</span>
          <label className="recurrence-option md-typescale-body-medium">
            <md-radio name="ends" value="never" touch-target="wrapper" checked={ends === 'never'} onchange={() => setEnds('never')}></md-radio>
            Never
          </label>
          <div className="recurrence-option">
            <label className="recurrence-option-label md-typescale-body-medium">
              <md-radio name="ends" value="on" touch-target="wrapper" checked={ends === 'on'} onchange={() => setEnds('on')}></md-radio>
              On
            </label>
            <DatePicker
              value={untilTs}
              tz={tz}
              label="End date"
              onChange={(ts) => {
                setUntilTs(ts);
                setEnds('on');
              }}
            />
          </div>
          <div className="recurrence-option">
            <label className="recurrence-option-label md-typescale-body-medium">
              <md-radio name="ends" value="after" touch-target="wrapper" checked={ends === 'after'} onchange={() => setEnds('after')}></md-radio>
              After
            </label>
            <md-outlined-text-field
              className="recurrence-number"
              label="Count"
              type="number"
              min="1"
              value={String(count)}
              oninput={(e) => {
                setCount(numberValue(e, 1));
                setEnds('after');
              }}
            ></md-outlined-text-field>
            <span className="md-typescale-body-medium">occurrences</span>
          </div>
        </div>
      </div>
      <div slot="actions">
        <md-text-button onclick={close}>Cancel</md-text-button>
        <md-filled-button onclick={done}>Done</md-filled-button>
      </div>
    </md-dialog>
  );
}
