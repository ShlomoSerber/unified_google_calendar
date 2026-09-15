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
import { SettingsDialog } from './SettingsDialog';
import { WelcomeDialog } from './WelcomeDialog';
import { GoaDialog } from './GoaDialog';
import { EventPopup } from '../event/EventPopup';
import { QuickCreate } from '../event/QuickCreate';
import { FullForm } from '../event/FullForm';
import { EditScopeDialog } from '../event/EditScopeDialog';
import { RecurrenceDialog } from '../event/RecurrenceDialog';
import { ViewSelector } from './ViewSelector';
import './App.css';

// Shell: top bar, sidebar and the view router (docs/02 section 4 `src/app/`).
export function App() {
  const view = useUi((s) => s.view);
  const date = useUi((s) => s.date);
  const tz = useUi((s) => s.tz);
  const sidebarOpen = useUi((s) => s.sidebarOpen);
  const dialog = useUi((s) => s.dialog);
  const overlay = useUi((s) => s.overlay);
  const days = weekDays(date, tz);

  useEffect(() => {
    const { setAccounts, setCalendars, setNow, setSettings, setSyncStatus } = useUi.getState();
    ipc.getSettings().then(setSettings).catch(() => undefined);
    ipc
      .listAccounts()
      .then((a) => {
        setAccounts(a);
        // First run (docs/07 section 5): no account yet → welcome.
        if (a.length === 0 && useUi.getState().dialog.kind === 'none') useUi.getState().openDialog({ kind: 'welcome' });
      })
      .catch(() => undefined);
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
      {dialog.kind === 'settings' ? <SettingsDialog /> : null}
      {dialog.kind === 'welcome' ? <WelcomeDialog /> : null}
      {dialog.kind === 'goa' ? <GoaDialog /> : null}
      {dialog.kind === 'event' ? <EventPopup occurrenceId={dialog.occurrenceId} anchor={dialog.anchor} /> : null}
      {dialog.kind === 'quick-create' ? <QuickCreate startTs={dialog.startTs} endTs={dialog.endTs} allDay={dialog.allDay} anchor={dialog.anchor} /> : null}
      {dialog.kind === 'full-form' ? <FullForm occurrenceId={dialog.occurrenceId} startTs={dialog.startTs} endTs={dialog.endTs} allDay={dialog.allDay} draft={dialog.draft} /> : null}
      {overlay.kind === 'edit-scope' ? <EditScopeDialog occurrenceId={overlay.occurrenceId} action={overlay.action} draft={overlay.draft} /> : null}
      {overlay.kind === 'recurrence' ? <RecurrenceDialog rrule={overlay.rrule} startTs={overlay.startTs} /> : null}
      {dialog.kind === 'view-menu' ? <ViewSelector /> : null}
    </div>
  );
}
