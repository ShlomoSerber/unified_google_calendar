import { useRef } from 'react';
import { format } from 'date-fns';
import type { MdMenu } from '@material/web/menu/menu.js';
import { ipc } from '../ipc';
import { inZone } from '../lib/dates';
import { useUi, type ViewKind } from '../state/ui';
import { Tooltip } from './Tooltip';
import './TopBar.css';

// The top bar (docs/11 section 7 and section 9): drawer toggle, range title, previous/next,
// Today, the view button with its menu, then Help (README) and Settings at the right. The set
// of controls is the user's (docs/99, 2026-09-15): nothing that does not work in version 1.

const VIEW_LABEL: Record<ViewKind, string> = { day: 'Day', week: 'Week', month: 'Month', year: 'Year', agenda: 'Schedule' };
const VIEW_UNIT: Record<ViewKind, string> = { day: 'day', week: 'week', month: 'month', year: 'year', agenda: 'week' };

// Only the views that exist: no "4 days", no shortcut letters (keyboard shortcuts are version
// 2) and no toggles (docs/99, 2026-09-15).
const VIEWS: { label: string; view: ViewKind }[] = [
  { label: 'Day', view: 'day' },
  { label: 'Week', view: 'week' },
  { label: 'Month', view: 'month' },
  { label: 'Year', view: 'year' },
  { label: 'Schedule', view: 'agenda' },
];

/** "September 2026", or "Sep – Oct 2026" when the visible week spans two months. */
export function rangeTitle(view: ViewKind, date: number, tz: string, weekDays: number[]): string {
  const d = inZone(date, tz);
  if (view === 'week' || view === 'agenda') {
    const first = inZone(weekDays[0] ?? date, tz);
    const last = inZone(weekDays[weekDays.length - 1] ?? date, tz);
    if (first.getMonth() !== last.getMonth()) {
      const fy = first.getFullYear();
      const ly = last.getFullYear();
      return fy === ly ? `${format(first, 'MMM')} – ${format(last, 'MMM yyyy')}` : `${format(first, 'MMM yyyy')} – ${format(last, 'MMM yyyy')}`;
    }
    return format(first, 'MMMM yyyy');
  }
  if (view === 'day') return format(d, 'MMMM d, yyyy');
  if (view === 'year') return format(d, 'yyyy');
  return format(d, 'MMMM yyyy');
}

export interface TopBarProps {
  weekDays: number[];
}

export function TopBar({ weekDays }: TopBarProps) {
  const view = useUi((s) => s.view);
  const date = useUi((s) => s.date);
  const tz = useUi((s) => s.tz);
  const now = useUi((s) => s.now);
  const { today, prev, next, openDialog } = useUi.getState();
  const menu = useRef<MdMenu>(null);
  const todayLabel = format(inZone(now, tz), 'EEEE, MMMM d');
  const unit = VIEW_UNIT[view];
  const title = rangeTitle(view, date, tz, weekDays);
  const toggleSidebar = () => useUi.getState().toggleSidebar();
  const openSettings = () => openDialog({ kind: 'settings' });
  const toggleViewMenu = () => {
    if (menu.current) menu.current.open = !menu.current.open;
  };
  const openHelp = () => {
    ipc.openUrl('https://github.com/ShlomoSerber/unified_google_calendar#readme').catch(() => undefined);
  };

  return (
    <header className="topbar" role="banner">
      <Tooltip text="Main drawer">
        <md-icon-button aria-label="Main drawer" onclick={toggleSidebar}>
          <md-icon>menu</md-icon>
        </md-icon-button>
      </Tooltip>
      <h1 className="topbar-range md-typescale-title-large">{title}</h1>
      <Tooltip text={`Previous ${unit}`}>
        <md-icon-button aria-label={`Previous ${unit}`} onclick={prev}>
          <md-icon>chevron_left</md-icon>
        </md-icon-button>
      </Tooltip>
      <Tooltip text={`Next ${unit}`}>
        <md-icon-button aria-label={`Next ${unit}`} onclick={next}>
          <md-icon>chevron_right</md-icon>
        </md-icon-button>
      </Tooltip>
      <Tooltip text={todayLabel}>
        <md-outlined-button aria-label={`Today, ${todayLabel}`} onclick={today}>
          Today
        </md-outlined-button>
      </Tooltip>
      <span className="topbar-view">
        <md-outlined-button id="topbar-view-button" aria-haspopup="menu" trailing-icon onclick={toggleViewMenu}>
          <md-icon slot="icon">arrow_drop_down</md-icon>
          {VIEW_LABEL[view]}
        </md-outlined-button>
        <md-menu ref={menu} anchor="topbar-view-button" positioning="fixed" aria-label="View">
          {VIEWS.map((v) => (
            <md-menu-item key={v.view} onclick={() => useUi.getState().setView(v.view)}>
              <div slot="headline">{v.label}</div>
            </md-menu-item>
          ))}
        </md-menu>
      </span>
      <span className="topbar-grow"></span>
      <Tooltip text="Help">
        <md-icon-button aria-label="Help" onclick={openHelp}>
          <md-icon>help</md-icon>
        </md-icon-button>
      </Tooltip>
      <Tooltip text="Settings">
        <md-icon-button aria-label="Settings" onclick={openSettings}>
          <md-icon>settings</md-icon>
        </md-icon-button>
      </Tooltip>
    </header>
  );
}
