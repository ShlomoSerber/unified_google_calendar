import { useState } from 'react';
import { ipc } from '../ipc';
import { useUi } from '../state/ui';
import './AccountsPanel.css';

// Provisional settings panel (accounts) until F7-T6 measures Google's settings dialog. It only
// exposes the account commands: add a Google account, add a read-only iCal calendar, sync now,
// remove. Colours and sizes come from measured tokens of neighbouring components.
export function AccountsPanel() {
  const accounts = useUi((s) => s.accounts);
  const syncStatus = useUi((s) => s.syncStatus);
  const close = () => useUi.getState().closeDialog();
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [icalName, setIcalName] = useState('');
  const [icalUrl, setIcalUrl] = useState('');
  const [icalEmail, setIcalEmail] = useState('');

  const run = async (label: string, fn: () => Promise<string>) => {
    setBusy(true);
    setMessage(label);
    try {
      setMessage(await fn());
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(false);
    }
  };
  const addGoogle = () =>
    run('Waiting for the browser sign-in…', async () => {
      const a = await ipc.addAccount();
      return `Added ${a.email ?? a.id}; the first sync is running.`;
    });
  const addIcal = () =>
    run('Downloading the calendar…', async () => {
      const a = await ipc.addIcalCalendar(icalName, icalUrl, icalEmail || null);
      setIcalName('');
      setIcalUrl('');
      return `Added ${a.display_name} (read-only iCal).`;
    });
  const remove = (id: string, name: string) => {
    if (!window.confirm(`Remove ${name} from this app? Its events stay in Google.`)) return;
    void run('Removing…', async () => {
      await ipc.removeAccount(id);
      return `Removed ${name}.`;
    });
  };

  return (
    <div className="accounts-panel-scrim" onClick={close}>
      <div className="accounts-panel" role="dialog" aria-label="Settings" onClick={(e) => e.stopPropagation()}>
        <div className="accounts-panel-title">Accounts</div>
        <ul className="accounts-panel-list">
          {accounts.map((a) => {
            const s = syncStatus[a.id];
            return (
              <li className="accounts-panel-row" key={a.id}>
                <span className="accounts-panel-name">{a.display_name}</span>
                <span className="accounts-panel-meta">
                  {a.kind === 'google' ? (a.email ?? 'Google') : 'iCal (read-only)'} · {s?.state ?? a.sync_state}
                  {a.sync_error ? ` · ${a.sync_error}` : ''}
                </span>
                <button className="accounts-panel-button" type="button" disabled={busy} onClick={() => remove(a.id, a.display_name)}>
                  Remove
                </button>
              </li>
            );
          })}
        </ul>
        <div className="accounts-panel-actions">
          <button className="accounts-panel-button" type="button" disabled={busy} onClick={addGoogle}>
            Add Google account
          </button>
          <button className="accounts-panel-button" type="button" disabled={busy} onClick={() => void run('Syncing…', async () => ipc.syncNow().then(() => 'Sync requested.'))}>
            Sync now
          </button>
          <button className="accounts-panel-button" type="button" onClick={close}>
            Close
          </button>
        </div>
        <div className="accounts-panel-ical">
          <input className="accounts-panel-input" placeholder="Name (e.g. RappiCard)" value={icalName} onChange={(e) => setIcalName(e.target.value)} />
          <input className="accounts-panel-input" placeholder="Secret iCal address (https://…/basic.ics)" value={icalUrl} onChange={(e) => setIcalUrl(e.target.value)} />
          <input className="accounts-panel-input" placeholder="Your e-mail in that calendar (optional)" value={icalEmail} onChange={(e) => setIcalEmail(e.target.value)} />
          <button className="accounts-panel-button" type="button" disabled={busy || !icalName || !icalUrl} onClick={addIcal}>
            Add iCal calendar
          </button>
        </div>
        {message ? <p className="accounts-panel-message">{message}</p> : null}
      </div>
    </div>
  );
}
