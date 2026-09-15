import { useEffect, useRef, useState } from 'react';
import { addMonths, format } from 'date-fns';
import { dayAria } from '../app/MiniCalendar';
import { inZone, isSameDay, monthGrid, toTs } from '../lib/dates';
import { useUi } from '../state/ui';
import './DatePicker.css';

// A date field of the event form (docs/99, 2026-09-15): the date as text and, on click, a month
// grid built from the measured mini calendar (component 4, `sidebar-minical-*`), with its own
// month navigation. `value` and the result are UTC seconds of an instant inside the day.

const DOW: [string, string][] = [['M', 'Monday'], ['T', 'Tuesday'], ['W', 'Wednesday'], ['T', 'Thursday'], ['F', 'Friday'], ['S', 'Saturday'], ['S', 'Sunday']];
const CHEVRON_LEFT = 'M15.41 7.41L14 6l-6 6 6 6 1.41-1.41L10.83 12l4.58-4.59z';
const CHEVRON_RIGHT = 'M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6-6-6z';
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
  const rootRef = useRef<HTMLDivElement>(null);
  const toggle = () => {
    if (!open) setMonthTs(value);
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
  const month = inZone(monthTs, tz).getMonth();
  const monthLabel = format(inZone(monthTs, tz), 'MMMM yyyy');
  const rows = monthGrid(monthTs, tz, 6);
  const shift = (n: number) => setMonthTs(toTs(addMonths(inZone(monthTs, tz), n)));
  const pick = (ts: number) => {
    const d = inZone(value, tz);
    const day = inZone(ts, tz);
    d.setFullYear(day.getFullYear(), day.getMonth(), day.getDate());
    onChange(toTs(d));
    setOpen(false);
  };
  return (
    <div className="dp-root" ref={rootRef}>
      <button className="dp-field" type="button" aria-label={label} aria-haspopup="dialog" aria-expanded={open} onClick={toggle}>
        {format(inZone(value, tz), DATE_FORMAT)}
      </button>
      {open ? (
        <div className="dp-popover motion-menu" role="dialog" aria-label={`Pick ${label}`}>
          <div className="sidebar-minical dp-minical">
            <div className="sidebar-minical-box">
              <div className="sidebar-minical-head">
                <span className="sidebar-minical-month">{monthLabel}</span>
                <div className="sidebar-minical-nav">
                  {(['prev', 'next'] as const).map((dir) => (
                    <div className={`sidebar-minical-${dir}-cell`} key={dir}>
                      <span className={`sidebar-minical-${dir}-span`}>
                        <button className={`sidebar-minical-${dir}`} aria-label={dir === 'prev' ? 'Previous month' : 'Next month'} type="button" onClick={() => shift(dir === 'prev' ? -1 : 1)}>
                          <span className={`sidebar-minical-${dir}-ripple ugc-state ugc-state-icon`}></span>
                          <span className={`sidebar-minical-${dir}-icon-box`}>
                            <span className={`sidebar-minical-${dir}-icon-span`}>
                              <svg className={`sidebar-minical-${dir}-icon`} viewBox="0 0 24 24" focusable="false">
                                <path className={`sidebar-minical-${dir}-path`} d={dir === 'prev' ? CHEVRON_LEFT : CHEVRON_RIGHT} />
                              </svg>
                            </span>
                          </span>
                          <div className={`sidebar-minical-${dir}-overlay`}></div>
                        </button>
                      </span>
                    </div>
                  ))}
                </div>
              </div>
              <table className="sidebar-minical-grid" role="grid" aria-label={monthLabel}>
                <thead className="sidebar-minical-thead">
                  <tr className="sidebar-minical-head-row">
                    {DOW.map(([letter, name], i) => (
                      <th className="sidebar-minical-dow" key={i} aria-label={name}>
                        <span className="sidebar-minical-dow-span">
                          <div className="sidebar-minical-dow-label">{letter}</div>
                        </span>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody className="sidebar-minical-tbody">
                  {rows.map((row) => (
                    <tr className="sidebar-minical-row" key={row[0]}>
                      {row.map((ts) => {
                        const d = inZone(ts, tz);
                        const other = d.getMonth() !== month;
                        const today = isSameDay(ts, now, tz);
                        const selected = !today && isSameDay(ts, value, tz);
                        const kind = today ? '-today' : selected ? '-selected' : other ? '-other' : '';
                        const cellKind = today ? '-today' : other ? '-other' : '';
                        return (
                          <td className={`sidebar-minical-cell${cellKind}`} key={ts}>
                            <button className={`sidebar-minical-day${kind}`} aria-label={dayAria(ts, monthTs, now, tz)} type="button" onClick={() => pick(ts)}>
                              <span className={`sidebar-minical-day${kind}-ripple ugc-state ugc-state-primary`}></span>
                              <div className={`sidebar-minical-day${kind}-label`}>{d.getDate()}</div>
                            </button>
                          </td>
                        );
                      })}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
