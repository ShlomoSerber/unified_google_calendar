import { dayStart } from '../lib/dates';
import { useUi } from '../state/ui';
import './CreateButton.css';

// Component 3 of docs/04 section 4. Google places this button outside the drawer, absolutely
// positioned on the page (docs/design/measurements/create_button-light.json rootRect), so the
// shell renders it next to the sidebar, not inside it. Classes mirror the dump nodes. The
// menu (Event / Task / Out of office) is gone: only events exist in version 1, so the button
// reads "Create event" and opens the full form directly (docs/99, 2026-09-15).
export function CreateButton() {
  const createEvent = () => {
    const { date, tz } = useUi.getState();
    // Same default as Google's menu: the next full hour of the anchor day, one hour long.
    const now = Math.floor(Date.now() / 1000);
    const start = dayStart(date, tz) + (Math.floor((now - dayStart(now, tz)) / 3600) + 1) * 3600;
    useUi.getState().openDialog({ kind: 'full-form', occurrenceId: null, startTs: start, endTs: start + 3600, allDay: false });
  };
  return (
    <div className="create-button-anchor">
      <div className="create-button-shadow">
        <button className="create-button-root" type="button" onClick={createEvent}>
          <span className="create-button-ripple ugc-state ugc-state-primary"></span>
          <div className="create-button-box">
            <div className="create-button-label">
              <i className="create-button-icon">add</i>
              {'Create event'}
            </div>
          </div>
        </button>
      </div>
    </div>
  );
}
