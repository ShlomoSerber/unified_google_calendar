import { useEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from 'react';
import { addMonths, format } from 'date-fns';
import { MonthGrid } from '../components/MonthGrid';
import { dayStart, inZone, toTs } from '../lib/dates';
import { useUi } from '../state/ui';
import { Tooltip } from '../app/Tooltip';
import { LAYOUT, layoutNumber } from '../styles/layout';
import './DatePicker.css';

// A date field of the event form (docs/11 section 7): a read-only outlined text field with a
// calendar icon that opens a popover with a month header and a MonthGrid. `value` and the
// result are UTC seconds of an instant inside the day; picking keeps the time of day.

const DATE_FORMAT = 'EEE, MMM d, yyyy';

export interface DatePickerProps {
  value: number;
  tz: string;
  label: string;
  onChange: (ts: number) => void;
}

export function DatePicker({ value, tz, label, onChange }: DatePickerProps) {
  const now = useUi((s) => s.now);
  const [open, setOpen] = useState(false);
  const [monthTs, setMonthTs] = useState(value);
  const [at, setAt] = useState<{ left: number; top: number } | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const toggle = () => {
    if (!open) {
      setMonthTs(value);
      const r = rootRef.current?.getBoundingClientRect();
      if (r) setAt({ left: r.left, top: r.bottom + layoutNumber(LAYOUT.popup_gap) });
    }
    setOpen(!open);
  };
  useEffect(() => {
    if (!open) return undefined;
    const onDown = (e: MouseEvent) => {
      if (rootRef.current && e.target instanceof Node && !rootRef.current.contains(e.target)) setOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    return () => document.removeEventListener('mousedown', onDown);
  }, [open]);
  // Escape closes the popover only, not the dialog around it.
  const onKeyDown = (e: KeyboardEvent) => {
    if (open && e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      setOpen(false);
    }
  };
  const monthLabel = format(inZone(monthTs, tz), 'MMMM yyyy');
  const shift = (n: number) => setMonthTs(toTs(addMonths(inZone(monthTs, tz), n)));
  const pick = (ts: number) => {
    const d = inZone(value, tz);
    const day = inZone(ts, tz);
    d.setFullYear(day.getFullYear(), day.getMonth(), day.getDate());
    onChange(toTs(d));
    setOpen(false);
  };
  const selected = { from: dayStart(value, tz), to: dayStart(value, tz) + 86_400 };
  const popoverStyle: CSSProperties = at ? { left: at.left, top: at.top } : {};
  return (
    <div className="date-picker" ref={rootRef} onKeyDown={onKeyDown}>
      <md-outlined-text-field label={label} value={format(inZone(value, tz), DATE_FORMAT)} readOnly onclick={toggle}>
        <md-icon-button
          slot="trailing-icon"
          aria-label={`Pick ${label}`}
          aria-haspopup="dialog"
          aria-expanded={open}
          onclick={(e) => {
            // The field's own click would toggle it back.
            e.stopPropagation();
            toggle();
          }}
        >
          <md-icon>calendar_today</md-icon>
        </md-icon-button>
      </md-outlined-text-field>
      {open ? (
        <div className="date-picker-popover" role="dialog" aria-label={`Pick ${label}`} style={popoverStyle}>
          <div className="date-picker-head">
            <span className="date-picker-month md-typescale-title-small">{monthLabel}</span>
            <Tooltip text="Previous month">
              <md-icon-button aria-label="Previous month" onclick={() => shift(-1)}>
                <md-icon>chevron_left</md-icon>
              </md-icon-button>
            </Tooltip>
            <Tooltip text="Next month">
              <md-icon-button aria-label="Next month" onclick={() => shift(1)}>
                <md-icon>chevron_right</md-icon>
              </md-icon-button>
            </Tooltip>
          </div>
          <MonthGrid monthTs={monthTs} tz={tz} todayTs={now} selected={selected} size="normal" onPick={pick} />
        </div>
      ) : null}
    </div>
  );
}
