import { useEffect } from 'react';
import { ipc, onAccountChanged, onClockMinute, onSyncStatus } from '../ipc';
import { dayStart, weekDays } from '../lib/dates';
import { useUi } from '../state/ui';
import { TopBar } from './TopBar';
import { Sidebar } from './Sidebar';
import { CreateButton } from './CreateButton';
import { WeekView } from '../views/week/WeekView';
import { MonthView } from '../views/month/MonthView';
import { AgendaView } from '../views/agenda/AgendaView';
import { AccountsPanel } from './AccountsPanel';
import { EventPopup } from '../event/EventPopup';
import { QuickCreate } from '../event/QuickCreate';
import './App.css';

// Shell: top bar, sidebar and the view router (docs/02 section 4 `src/app/`).
export function App() {
  const view = useUi((s) => s.view);
  const date = useUi((s) => s.date);
  const tz = useUi((s) => s.tz);
  const sidebarOpen = useUi((s) => s.sidebarOpen);
  const dialog = useUi((s) => s.dialog);
  const days = weekDays(date, tz);

  useEffect(() => {
    const { setAccounts, setCalendars, setNow, setSettings, setSyncStatus } = useUi.getState();
    ipc.getSettings().then(setSettings).catch(() => undefined);
    ipc.listAccounts().then(setAccounts).catch(() => undefined);
    ipc.listCalendars().then(setCalendars).catch(() => undefined);
    const subs = [
      onAccountChanged((a) => {
        setAccounts(a);
        ipc.listCalendars().then(setCalendars).catch(() => undefined);
      }),
      onSyncStatus(setSyncStatus),
      onClockMinute(setNow),
    ];
    return () => {
      for (const s of subs) s.then((fn) => fn()).catch(() => undefined);
    };
  }, []);

  return (
    <div className="app">
      <TopBar weekDays={days} />
      <div className="app-body">
        {sidebarOpen ? <Sidebar /> : null}
        {sidebarOpen ? <CreateButton /> : null}
        <div className="app-main">
          {view === 'week' ? <WeekView days={days} /> : null}
          {view === 'day' ? <WeekView days={[dayStart(date, tz)]} mode="day" /> : null}
          {view === 'month' ? <MonthView /> : null}
          {view === 'agenda' ? <AgendaView /> : null}
        </div>
      </div>
      {dialog.kind === 'settings' ? <AccountsPanel /> : null}
      {dialog.kind === 'event' ? <EventPopup occurrenceId={dialog.occurrenceId} anchor={dialog.anchor} /> : null}
      {dialog.kind === 'quick-create' ? <QuickCreate startTs={dialog.startTs} endTs={dialog.endTs} allDay={dialog.allDay} anchor={dialog.anchor} /> : null}
    </div>
  );
}
