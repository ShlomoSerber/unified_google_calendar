import { useState, type CSSProperties, type ReactNode } from 'react';
import { ipc } from '../ipc';
import { chipColors, useTheme } from '../lib/colors';
import { useUi } from '../state/ui';
import type { AccountInfo, CalendarInfo } from '../types/ipc';
import { Tooltip } from './Tooltip';
import './CalendarList.css';

// The calendar list of the drawer (docs/11 section 7), grouped by account (docs/08 F4-T3):
// every Google account gets a collapsible section titled with the account; iCal and local
// calendars share "Other calendars", whose "+" opens the Add calendar dialog. Each row is a
// pill with a checkbox in the calendar's colour (docs/11 section 6).

interface RowProps {
  calendar: CalendarInfo;
  label: string;
  typeLabel: string | null;
}

function CalendarRow({ calendar, label, typeLabel }: RowProps) {
  const theme = useTheme();
  const on = calendar.visible;
  const color = { '--data-calendar-color': chipColors(calendar.color_bg, theme).color } as CSSProperties;
  // Optimistic toggle with rollback if the backend refuses.
  const toggle = () => {
    const next = !calendar.visible;
    const { calendars, setCalendars } = useUi.getState();
    setCalendars(calendars.map((c) => (c.account_id === calendar.account_id && c.id === calendar.id ? { ...c, visible: next } : c)));
    ipc.setCalendarVisible(calendar.account_id, calendar.id, next).catch(() => {
      const state = useUi.getState();
      state.setCalendars(state.calendars.map((c) => (c.account_id === calendar.account_id && c.id === calendar.id ? { ...c, visible: !next } : c)));
    });
  };
  return (
    <label className="calendar-list-row" role="listitem" style={color}>
      <md-ripple></md-ripple>
      <md-checkbox touch-target="wrapper" aria-label={label} checked={on} onchange={toggle}></md-checkbox>
      <span className="calendar-list-name md-typescale-body-medium">{label}</span>
      {typeLabel ? <span className="calendar-list-type md-typescale-label-small">{typeLabel}</span> : null}
    </label>
  );
}

interface SectionProps {
  title: string;
  tooltip: string | null;
  addButton: boolean;
  children: ReactNode;
}

function Section({ title, tooltip, addButton, children }: SectionProps) {
  const [open, setOpen] = useState(true);
  const openAddCalendar = () => useUi.getState().openDialog({ kind: 'add-calendar' });
  const toggle = (
    <button className="calendar-list-toggle md-typescale-label-medium" type="button" aria-expanded={open} onClick={() => setOpen(!open)}>
      <md-ripple></md-ripple>
      <md-focus-ring></md-focus-ring>
      <span className="calendar-list-toggle-label">{title}</span>
      <md-icon>{open ? 'expand_less' : 'expand_more'}</md-icon>
    </button>
  );
  return (
    <div className="calendar-list-section">
      <div className="calendar-list-head">
        {tooltip ? <Tooltip text={tooltip}>{toggle}</Tooltip> : toggle}
        {addButton ? (
          <Tooltip text="Add other calendars">
            <md-icon-button aria-label="Add other calendars" onclick={openAddCalendar}>
              <md-icon>add</md-icon>
            </md-icon-button>
          </Tooltip>
        ) : null}
      </div>
      {open ? (
        <div className="calendar-list-rows" role="list" aria-label={title}>
          {children}
        </div>
      ) : null}
    </div>
  );
}

const byOrder = <T extends { sort_order: number }>(a: T, b: T) => a.sort_order - b.sort_order;

export function accountTooltip(a: AccountInfo): string {
  if (a.kind === 'google') return a.email ? `Google account · ${a.email}` : 'Google account';
  return 'iCal calendar · read-only';
}

export function CalendarList() {
  const accounts = useUi((s) => s.accounts);
  const calendars = useUi((s) => s.calendars);
  const google = accounts.filter((a) => a.kind === 'google').sort(byOrder);
  const others = accounts.filter((a) => a.kind !== 'google').sort(byOrder);
  const calendarsOf = (a: AccountInfo) => calendars.filter((c) => c.account_id === a.id && !c.hidden_remote).sort(byOrder);

  return (
    <div className="calendar-list">
      {google.map((a) => (
        <Section key={a.id} title={a.display_name} tooltip={accountTooltip(a)} addButton={false}>
          {calendarsOf(a).map((c) => (
            <CalendarRow key={c.id} calendar={c} label={c.summary} typeLabel={null} />
          ))}
        </Section>
      ))}
      <Section title="Other calendars" tooltip={null} addButton={true}>
        {others.flatMap((a) => calendarsOf(a).map((c) => <CalendarRow key={`${a.id}/${c.id}`} calendar={c} label={c.summary} typeLabel={a.kind === 'ical' ? 'iCal' : null} />))}
      </Section>
    </div>
  );
}
