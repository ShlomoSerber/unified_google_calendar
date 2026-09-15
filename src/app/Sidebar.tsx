import { MiniCalendar } from './MiniCalendar';
import { CalendarList } from './CalendarList';
import './Sidebar.css';

// The drawer (docs/design/measurements/sidebar-light.json). Between the mini calendar and the
// calendar list Google shows "Meet with…" (people search) and "Booking pages", Workspace
// features outside version 1; they were kept as inert boxes until the user asked for them to go
// (docs/99, 2026-09-15), so the calendar list now follows the mini calendar after spacer_b.

export function Sidebar() {
  return (
    <div className="sidebar-root">
      <div className="sidebar-inner">
        <div className="sidebar-head"></div>
        <div className="sidebar-scroll">
          <h1 className="sidebar-sr-title">Drawer</h1>
          <MiniCalendar />
          <div className="sidebar-spacer-b">
            <div className="sidebar-spacer-b-inner"></div>
          </div>
          <CalendarList />
        </div>
        <div className="sidebar-foot"></div>
      </div>
    </div>
  );
}
