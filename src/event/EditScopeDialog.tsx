import { useEffect, useRef, useState } from 'react';
import type { MdDialog } from '@material/web/dialog/dialog.js';
import { ipc } from '../ipc';
import { withNotice } from '../lib/notice';
import { useUi } from '../state/ui';
import type { EditScope, EventDraft } from '../types/ipc';
import './EditScopeDialog.css';

// Scope of an edit or a deletion of a recurring event (docs/03 section 4): this event, this
// and following, or all. Stacked over the popup or the form; applying closes both.

const OPTIONS: { scope: EditScope; label: string }[] = [
  { scope: 'this', label: 'This event' },
  { scope: 'following', label: 'This and following events' },
  { scope: 'all', label: 'All events' },
];

export interface EditScopeDialogProps {
  occurrenceId: string;
  action: 'update' | 'delete';
  draft?: EventDraft;
}

export function EditScopeDialog({ occurrenceId, action, draft }: EditScopeDialogProps) {
  const dialog = useRef<MdDialog>(null);
  const [scope, setScope] = useState<EditScope>('this');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const close = () => dialog.current?.close();
  useEffect(() => {
    dialog.current?.show();
  }, []);
  const apply = async () => {
    setBusy(true);
    try {
      useUi.getState().closeDialog(); // the popup or form underneath is done too
      if (action === 'delete') await withNotice('Deleting...', 'Event deleted', () => ipc.deleteEvent(occurrenceId, scope));
      else if (draft) await withNotice('Saving...', 'Event saved', () => ipc.updateEvent(occurrenceId, draft, scope));
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };
  return (
    <md-dialog className="edit-scope" ref={dialog} aria-label={action === 'delete' ? 'Delete recurring event' : 'Edit recurring event'} onclosed={() => useUi.getState().closeOverlay()}>
      <div slot="headline">{action === 'delete' ? 'Delete recurring event' : 'Edit recurring event'}</div>
      <div slot="content" className="edit-scope-content" role="radiogroup" aria-label="Scope">
        {OPTIONS.map((o) => (
          <label className="edit-scope-option md-typescale-body-medium" key={o.scope}>
            <md-radio name="scope" value={o.scope} touch-target="wrapper" checked={scope === o.scope} onchange={() => setScope(o.scope)}></md-radio>
            {o.label}
          </label>
        ))}
        {error ? (
          <div className="edit-scope-error md-typescale-body-small" role="alert">
            {error}
          </div>
        ) : null}
      </div>
      <div slot="actions">
        <md-text-button onclick={close}>Cancel</md-text-button>
        <md-filled-button disabled={busy} onclick={() => void apply()}>
          OK
        </md-filled-button>
      </div>
    </md-dialog>
  );
}
