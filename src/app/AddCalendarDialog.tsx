import { useEffect, useRef, useState } from 'react';
import type { MdDialog } from '@material/web/dialog/dialog.js';
import type { MdOutlinedTextField } from '@material/web/textfield/outlined-text-field.js';
import { ipc } from '../ipc';
import { useUi } from '../state/ui';
import './AddCalendarDialog.css';

// The "+" of "Other calendars" (docs/99, 2026-09-15): a read-only iCal calendar by secret
// address, or another Google account, without going through Settings.

const OAUTH_HINT = 'Create ~/.config/unified-google-calendar/oauth.json first (docs/09 section A)';
const fieldValue = (e: Event) => (e.target as MdOutlinedTextField).value;

export function AddCalendarDialog() {
  const settings = useUi((s) => s.settings);
  const dialog = useRef<MdDialog>(null);
  const [name, setName] = useState('');
  const [url, setUrl] = useState('');
  const [email, setEmail] = useState('');
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const close = () => dialog.current?.close();
  const oauth = settings?.oauth_configured ?? false;

  useEffect(() => {
    dialog.current?.show();
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
    <md-dialog className="add-calendar" ref={dialog} aria-label="Add calendar" onclosed={() => useUi.getState().closeDialog()}>
      <div slot="headline">Add calendar</div>
      <div slot="content" className="add-calendar-content">
        <p className="add-calendar-text md-typescale-body-medium">Subscribe to a calendar by its secret iCal address. It is read-only and refreshes with every sync.</p>
        <md-outlined-text-field label="Name" placeholder="RappiCard" value={name} autofocus oninput={(e) => setName(fieldValue(e))}></md-outlined-text-field>
        <md-outlined-text-field label="Secret iCal address" placeholder="https://…/basic.ics" value={url} oninput={(e) => setUrl(fieldValue(e))}></md-outlined-text-field>
        <md-outlined-text-field label="Your e-mail in that calendar (optional)" value={email} oninput={(e) => setEmail(fieldValue(e))}></md-outlined-text-field>
        <p className="add-calendar-text md-typescale-body-medium">Or add another Google account with all its calendars.</p>
        {message ? (
          <p className="add-calendar-message md-typescale-body-small" role="status">
            {message}
          </p>
        ) : null}
      </div>
      <div slot="actions">
        <md-text-button onclick={close}>Cancel</md-text-button>
        <md-text-button disabled={busy || !oauth} title={oauth ? undefined : OAUTH_HINT} onclick={() => void addGoogle()}>
          Add Google account
        </md-text-button>
        <md-filled-button disabled={busy || !name.trim() || !url.trim()} onclick={() => void addIcal()}>
          Add
        </md-filled-button>
      </div>
    </md-dialog>
  );
}
