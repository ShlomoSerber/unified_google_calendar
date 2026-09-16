import { CreateButton } from './CreateButton';
import { MiniCalendar } from './MiniCalendar';
import { CalendarList } from './CalendarList';
import './Sidebar.css';

// The drawer (docs/11 section 7): Create, the mini calendar and the calendar list in a column.
export function Sidebar() {
  return (
    <div className="sidebar">
      <CreateButton />
      <MiniCalendar />
      <CalendarList />
    </div>
  );
}
