import { useEffect, useRef, useState } from 'react';
import type { MdDialog } from '@material/web/dialog/dialog.js';
import { ipc } from '../ipc';
import { useUi } from '../state/ui';
import './WelcomeDialog.css';

// First run (docs/07 section 5): without oauth.json the steps of docs/09 section A; then the
// first Google account. The dialog cannot be dismissed with Escape or the scrim.

const OAUTH_PATH = '~/.config/unified-google-calendar/oauth.json';

export function WelcomeDialog() {
  const settings = useUi((s) => s.settings);
  const dialog = useRef<MdDialog>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const configured = settings?.oauth_configured ?? false;
  useEffect(() => {
    dialog.current?.show();
  }, []);
  const recheck = () => {
    ipc
      .getSettings()
      .then((s) => {
        useUi.getState().setSettings(s);
        if (!s.oauth_configured) setMessage(`${OAUTH_PATH} is still missing or malformed.`);
      })
      .catch((e: unknown) => setMessage(String(e)));
  };
  const addAccount = async () => {
    setBusy(true);
    setMessage('Waiting for the browser sign-in…');
    try {
      const a = await ipc.addAccount();
      setMessage(`Added ${a.email ?? a.id}; the first sync is running.`);
      useUi.getState().closeDialog();
      // Step 3 of the first run: the Online Accounts question.
      const goa = await ipc.goaStatus();
      if (!goa.prompt_done && goa.accounts.some((g) => !g.calendar_disabled)) useUi.getState().openDialog({ kind: 'goa' });
    } catch (e) {
      setMessage(String(e));
      setBusy(false);
    }
  };
  return (
    <md-dialog className="welcome" ref={dialog} aria-label="Welcome" oncancel={(e) => e.preventDefault()}>
      <div slot="headline">{configured ? 'Add your first Google account' : 'Welcome to Unified Google Calendar'}</div>
      <div slot="content" className="welcome-content md-typescale-body-medium">
        {configured ? (
          <p className="welcome-text">Sign in with the Google account you want to see first. More accounts and iCal calendars can be added later in Settings.</p>
        ) : (
          <>
            <p className="welcome-text">{`This app needs your own Google Cloud OAuth client. Follow docs/09-setup-usuario.md section A, then save the client id and secret as ${OAUTH_PATH}:`}</p>
            <pre className="welcome-code md-typescale-body-small">{'{ "client_id": "…apps.googleusercontent.com", "client_secret": "…" }'}</pre>
            <ol className="welcome-steps">
              <li>Create a project in Google Cloud and enable the Google Calendar API.</li>
              <li>Configure the OAuth consent screen (external, published) with the calendar scopes.</li>
              <li>Create an OAuth client of type Desktop app and copy its id and secret into the file.</li>
            </ol>
          </>
        )}
        {message ? (
          <p className="welcome-message md-typescale-body-small" role="status">
            {message}
          </p>
        ) : null}
      </div>
      <div slot="actions">
        {configured ? (
          <md-filled-button disabled={busy} onclick={() => void addAccount()}>
            Add Google account
          </md-filled-button>
        ) : (
          <md-filled-button onclick={recheck}>I created the file, continue</md-filled-button>
        )}
      </div>
    </md-dialog>
  );
}
