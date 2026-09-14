import { useEffect, useState } from 'react';
import { ipc, onAccountChanged, onSyncStatus } from '../ipc';
import type { AccountInfo } from '../types/ipc';
import './App.css';

// Placeholder shell until F4-T3 (docs/08). Unstyled on purpose: no measured tokens exist yet.
// It only exposes the account commands so real accounts can be added and synced.
export function App() {
  const [accounts, setAccounts] = useState<AccountInfo[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [icalName, setIcalName] = useState('');
  const [icalUrl, setIcalUrl] = useState('');
  const [icalEmail, setIcalEmail] = useState('');

  useEffect(() => {
    ipc.listAccounts().then(setAccounts).catch((e: unknown) => setMessage(String(e)));
    const unlisten = [onAccountChanged(setAccounts), onSyncStatus((s) => setMessage(`${s.account_id}: ${s.state}${s.message ? ` (${s.message})` : ''}`))];
    return () => {
      for (const u of unlisten) u.then((fn) => fn()).catch(() => undefined);
    };
  }, []);

  const add = async () => {
    setBusy(true);
    setMessage('Waiting for the browser sign-in…');
    try {
      const a = await ipc.addAccount();
      setMessage(`Added ${a.email ?? a.id}; syncing.`);
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(false);
    }
  };

  const addIcal = async () => {
    setBusy(true);
    setMessage('Downloading the calendar…');
    try {
      const a = await ipc.addIcalCalendar(icalName, icalUrl, icalEmail || null);
      setMessage(`Added ${a.display_name} (read-only iCal).`);
      setIcalName('');
      setIcalUrl('');
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="app">
      <h1 className="app-title">Unified Google Calendar</h1>
      <ul>
        {accounts.map((a) => (
          <li key={a.id}>
            {a.display_name} · {a.kind} · {a.sync_state}
            {a.sync_error ? ` · ${a.sync_error}` : ''}
          </li>
        ))}
      </ul>
      <button type="button" onClick={add} disabled={busy}>
        Add Google account
      </button>{' '}
      <button type="button" onClick={() => ipc.syncNow().catch((e: unknown) => setMessage(String(e)))}>
        Sync now
      </button>
      <p>
        <input placeholder="Name (e.g. RappiCard)" value={icalName} onChange={(e) => setIcalName(e.target.value)} />{' '}
        <input placeholder="Secret iCal address (https://…/basic.ics)" value={icalUrl} onChange={(e) => setIcalUrl(e.target.value)} />{' '}
        <input placeholder="Your e-mail in that calendar (optional)" value={icalEmail} onChange={(e) => setIcalEmail(e.target.value)} />{' '}
        <button type="button" onClick={addIcal} disabled={busy || !icalName || !icalUrl}>
          Add iCal calendar
        </button>
      </p>
      {message ? <p>{message}</p> : null}
    </div>
  );
}
