import { useEffect, useState } from 'react';
import { ipc } from '../ipc';
import { useUi } from '../state/ui';
import './AddCalendarDialog.css';

// The "+" of "Other calendars" (docs/99, 2026-09-15): a dialog of its own for a read-only iCal
// calendar by secret address, or a Google account, instead of sending the user to Settings.
// Built from the measured dialog tokens (scope-*, rec-*, settings-*), like the Welcome dialog.

export function AddCalendarDialog() {
  const settings = useUi((s) => s.settings);
  const [name, setName] = useState('');
  const [url, setUrl] = useState('');
  const [email, setEmail] = useState('');
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const close = () => useUi.getState().closeDialog();
  const oauth = settings?.oauth_configured ?? false;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, []);

  const run = async (label: string, fn: () => Promise<string>, done: boolean) => {
    setBusy(true);
    setMessage(label);
    try {
      const m = await fn();
      if (done) close();
      else setMessage(m);
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(false);
    }
  };
  const addIcal = () =>
    run(
      'Downloading the calendar…',
      async () => {
        const a = await ipc.addIcalCalendar(name.trim(), url.trim(), email.trim() || null);
        return `Added ${a.display_name}.`;
      },
      true,
    );
  const addGoogle = () =>
    run(
      'Waiting for the browser sign-in…',
      async () => {
        const a = await ipc.addAccount();
        return `Added ${a.email ?? a.id}; the first sync is running.`;
      },
      true,
    );

  return (
    <div className="addcal-scrim" onMouseDown={close}>
      <div className="addcal-root scope-root" role="dialog" aria-modal="true" aria-label="Add calendar" onMouseDown={(e) => e.stopPropagation()}>
        <h2 className="scope-title-row">
          <span className="scope-title">Add calendar</span>
        </h2>
        <div className="addcal-body">
          <p className="addcal-text">Subscribe to a calendar by its secret iCal address. It is read-only and refreshes with every sync.</p>
          <input className="settings-input addcal-input" placeholder="Name (e.g. RappiCard)" value={name} onChange={(e) => setName(e.target.value)} autoFocus />
          <input className="settings-input addcal-input" placeholder="Secret iCal address (https://…/basic.ics)" value={url} onChange={(e) => setUrl(e.target.value)} />
          <input className="settings-input addcal-input" placeholder="Your e-mail in that calendar (optional)" value={email} onChange={(e) => setEmail(e.target.value)} />
          <p className="addcal-text addcal-or">Or add another Google account with all its calendars.</p>
          {message ? <p className="addcal-message">{message}</p> : null}
        </div>
        <div className="scope-footer">
          <div className="scope-cancel-wrap">
            <button className="scope-cancel" type="button" onClick={close}>
              <span className="scope-cancel-ripple ugc-state"></span>
              <span className="scope-cancel-hit"></span>
              <span className="scope-cancel-label">{'Cancel'}</span>
            </button>
          </div>
          <div className="scope-ok-wrap">
            <button className="scope-ok" type="button" disabled={busy || !name.trim() || !url.trim()} onClick={() => void addIcal()}>
              <span className="scope-ok-ripple">
                <span className="scope-ok-ripple-inner"></span>
              </span>
              <span className="scope-ok-n34"></span>
              <span className="scope-ok-hit"></span>
              <span className="scope-ok-label">{'Add'}</span>
            </button>
          </div>
          <div className="scope-ok-wrap addcal-google-wrap">
            <button className="scope-ok" type="button" disabled={busy || !oauth} title={oauth ? undefined : 'Create ~/.config/unified-google-calendar/oauth.json first (docs/09 section A)'} onClick={() => void addGoogle()}>
              <span className="scope-ok-ripple">
                <span className="scope-ok-ripple-inner"></span>
              </span>
              <span className="scope-ok-n34"></span>
              <span className="scope-ok-hit"></span>
              <span className="scope-ok-label">{'Add Google account'}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
