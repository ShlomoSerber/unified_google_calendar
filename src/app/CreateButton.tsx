import { dayStart } from '../lib/dates';
import { useUi } from '../state/ui';
import './CreateButton.css';

// Extended FAB of the drawer (docs/11 section 7). Only events exist in version 1, so it reads
// "Create event" and opens the full form directly (docs/99, 2026-09-15).
export function CreateButton() {
  const createEvent = () => {
    const { date, tz } = useUi.getState();
    // The next full hour of the anchor day, one hour long.
    const now = Math.floor(Date.now() / 1000);
    const start = dayStart(date, tz) + (Math.floor((now - dayStart(now, tz)) / 3600) + 1) * 3600;
    useUi.getState().openDialog({ kind: 'full-form', occurrenceId: null, startTs: start, endTs: start + 3600, allDay: false });
  };
  return (
    <md-fab className="create-fab" variant="primary" label="Create event" lowered onclick={createEvent}>
      <md-icon slot="icon">add</md-icon>
    </md-fab>
  );
}
