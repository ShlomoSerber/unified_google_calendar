import { useEffect, useState } from 'react';
import { ipc } from '../ipc';
import { useUi } from '../state/ui';
import type { EditScope, EventDraft } from '../types/ipc';
import './EditScopeDialog.css';

// Component 19 of docs/04 section 4 (docs/design/measurements/edit_scope_dialog-light.json):
// this / this and following / all, applied per docs/03 section 4. Google hides "This and
// following events" on the first occurrence of a series (the measured dump); the app always
// offers the three scopes of docs/03 (edit_scope_dialog.md).

export interface EditScopeDialogProps {
  occurrenceId: string;
  action: 'update' | 'delete';
  draft?: EventDraft;
}

export function EditScopeDialog({ occurrenceId, action, draft }: EditScopeDialogProps) {
  const [scope, setScope] = useState<EditScope>('this');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const options: { scope: EditScope; label: string }[] = [
    { scope: 'this', label: 'This event' },
    { scope: 'following', label: 'This and following events' },
    { scope: 'all', label: 'All events' },
  ];
  const close = () => useUi.getState().closeOverlay();
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopImmediatePropagation();
        close();
      }
    };
    document.addEventListener('keydown', onKey, true);
    return () => document.removeEventListener('keydown', onKey, true);
  }, []);
  const apply = async () => {
    setBusy(true);
    try {
      if (action === 'delete') await ipc.deleteEvent(occurrenceId, scope);
      else if (draft) await ipc.updateEvent(occurrenceId, draft, scope);
      useUi.getState().closeDialog(); // the popup or form underneath is done too
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };
  return (
    <div className="scope-scrim-page" onMouseDown={close}>
      <div className="scope-root" role="dialog" aria-modal="true" onMouseDown={(e) => e.stopPropagation()}>
        <span className="scope-scrim">
          <span className="scope-scrim-inner"></span>
        </span>
        <h2 className="scope-title-row">
          <span className="scope-title">{action === 'delete' ? 'Delete recurring event' : 'Edit recurring event'}</span>
        </h2>
        <div className="scope-body">
          <div className="scope-group" role="radiogroup">
            {options.map((o, i) => {
              const p = i === 0 ? 'scope-radio' : 'scope-radio2';
              const checked = scope === o.scope;
              return (
                <label className={i === 0 ? 'scope-option' : 'scope-option2'} key={o.scope}>
                  <div className={`${p}-cell`}>
                    <div className={p}>
                      <input className={`${p}-input`} type="radio" name="scope" checked={checked} onChange={() => setScope(o.scope)} />
                      <div className={`${p}-ring`}>
                        <div className={`${p}-outer`}></div>
                        {checked ? <div className="scope-radio-dot"></div> : null}
                      </div>
                      <span className={`${p}-ripple`}></span>
                    </div>
                  </div>
                  <div className={i === 0 ? 'scope-option-label' : 'scope-option2-label'}>{o.label}</div>
                </label>
              );
            })}
          </div>
        </div>
        <div className="scope-footer">
          <div className="scope-cancel-wrap">
            <button className="scope-cancel" type="button" onClick={close}>
              <span className="scope-cancel-ripple"></span>
              <span className="scope-cancel-hit"></span>
              <span className="scope-cancel-label">{"Cancel"}</span>
            </button>
          </div>
          <div className="scope-ok-wrap">
            <button className="scope-ok" type="button" onClick={() => void apply()} disabled={busy}>
              <span className="scope-ok-ripple">
                <span className="scope-ok-ripple-inner"></span>
              </span>
              <span className="scope-ok-n34"></span>
              <span className="scope-ok-hit"></span>
              <span className="scope-ok-label">{"OK"}</span>
            </button>
          </div>
        </div>
      </div>
      {error ? <div className="scope-error" role="alert">{error}</div> : null}
    </div>
  );
}
