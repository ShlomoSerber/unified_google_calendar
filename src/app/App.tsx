import { useEffect } from 'react';
import { ipc, onAccountChanged, onClockMinute, onSyncStatus } from '../ipc';
import { format } from 'date-fns';
import { dayStart, inZone, weekDays } from '../lib/dates';
import { installRipples, ms, usePresence } from '../lib/motion';
import { useUi, type Dialog, type ViewKind } from '../state/ui';
import { MOTION } from '../styles/legacy-motion';
import '../styles/motion.css';
import { TopBar } from './TopBar';
import { Sidebar } from './Sidebar';
import { CreateButton } from './CreateButton';
import { WeekView } from '../views/week/WeekView';
import { MonthView } from '../views/month/MonthView';
import { AgendaView } from '../views/agenda/AgendaView';
import { YearView } from '../views/year/YearView';
import { SettingsDialog } from './SettingsDialog';
import { WelcomeDialog } from './WelcomeDialog';
import { GoaDialog } from './GoaDialog';
import { AddCalendarDialog } from './AddCalendarDialog';
import { EventPopup } from '../event/EventPopup';
import { QuickCreate } from '../event/QuickCreate';
import { FullForm } from '../event/FullForm';
import { EditScopeDialog } from '../event/EditScopeDialog';
import { RecurrenceDialog } from '../event/RecurrenceDialog';
import { ViewSelector } from './ViewSelector';
import { ViewStage } from './ViewStage';
import { Tooltip } from './Tooltip';
import { Snackbar } from './Snackbar';
import './App.css';

/** First instant of the visible range: a change of it slides the grid (ViewStage). */
function rangeStart(view: ViewKind, date: number, days: number[], tz: string): string {
  if (view === 'week') return String(days[0]);
  if (view === 'month') return format(inZone(date, tz), 'yyyy-MM');
  if (view === 'year') return format(inZone(date, tz), 'yyyy');
  return String(dayStart(date, tz));
}

// Dialog motion (docs/04 section 10): a closed dialog stays mounted, exiting, for its close
// duration. Popups fade in 150 ms, menus in 75 ms, pages (full form, settings) in 200 ms.
const dialogOpen = (d: Dialog) => d.kind !== 'none';
function dialogExitMs(d: Dialog): number {
  switch (d.kind) {
    case 'event':
    case 'quick-create':
      return ms(MOTION.popup_close_duration);
    case 'view-menu':
      return ms(MOTION.menu_close_duration);
    case 'full-form':
    case 'settings':
    case 'welcome':
    case 'goa':
    case 'add-calendar':
      return ms(MOTION.view_fade_duration);
    default:
      return 0;
  }
}

// Shell: top bar, sidebar and the view router (docs/02 section 4 `src/app/`).
export function App() {
  const view = useUi((s) => s.view);
  const date = useUi((s) => s.date);
  const tz = useUi((s) => s.tz);
  const sidebarOpen = useUi((s) => s.sidebarOpen);
  const navDir = useUi((s) => s.navDir);
  const dialog = useUi((s) => s.dialog);
  const overlay = useUi((s) => s.overlay);
  const days = weekDays(date, tz);
  const { shown: dlg, exiting } = usePresence(dialog, dialogOpen, dialogExitMs);
  const pageClass = exiting ? 'motion-layer motion-page-exit' : 'motion-layer motion-page';

  useEffect(() => installRipples(), []);

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
        <div className={sidebarOpen ? 'app-drawer' : 'app-drawer app-drawer-closed'} aria-hidden={!sidebarOpen}>
          <Sidebar />
        </div>
        <div className={sidebarOpen ? 'app-create' : 'app-create app-create-hidden'} aria-hidden={!sidebarOpen}>
          <CreateButton />
        </div>
        <div className="app-main">
          <ViewStage view={view} rangeKey={`${view}:${rangeStart(view, date, days, tz)}`} navDir={navDir}>
            {view === 'week' ? <WeekView days={days} /> : view === 'day' ? <WeekView days={[dayStart(date, tz)]} mode="day" /> : view === 'month' ? <MonthView /> : view === 'year' ? <YearView /> : <AgendaView />}
          </ViewStage>
        </div>
      </div>
      {dlg.kind === 'settings' ? <div className={pageClass}><SettingsDialog /></div> : null}
      {dlg.kind === 'welcome' ? <div className={pageClass}><WelcomeDialog /></div> : null}
      {dlg.kind === 'goa' ? <div className={pageClass}><GoaDialog /></div> : null}
      {dlg.kind === 'add-calendar' ? <div className={pageClass}><AddCalendarDialog /></div> : null}
      {dlg.kind === 'event' ? <EventPopup occurrenceId={dlg.occurrenceId} anchor={dlg.anchor} exiting={exiting} /> : null}
      {dlg.kind === 'quick-create' ? <QuickCreate startTs={dlg.startTs} endTs={dlg.endTs} allDay={dlg.allDay} anchor={dlg.anchor} exiting={exiting} /> : null}
      {dlg.kind === 'full-form' ? <div className={pageClass}><FullForm occurrenceId={dlg.occurrenceId} startTs={dlg.startTs} endTs={dlg.endTs} allDay={dlg.allDay} draft={dlg.draft} /></div> : null}
      {overlay.kind === 'edit-scope' ? <EditScopeDialog occurrenceId={overlay.occurrenceId} action={overlay.action} draft={overlay.draft} /> : null}
      {overlay.kind === 'recurrence' ? <RecurrenceDialog rrule={overlay.rrule} startTs={overlay.startTs} /> : null}
      {dlg.kind === 'view-menu' ? <ViewSelector exiting={exiting} /> : null}
      <Snackbar />
      <Tooltip />
    </div>
  );
}
