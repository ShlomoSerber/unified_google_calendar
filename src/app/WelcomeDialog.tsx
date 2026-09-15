import { useState } from 'react';
import { ipc } from '../ipc';
import { useUi } from '../state/ui';
import './WelcomeDialog.css';

// First run (docs/07 section 5): without oauth.json the steps of docs/09 section A; then the
// first Google account. Built from the measured dialog tokens (F7-T6).

const OAUTH_PATH = '~/.config/unified-google-calendar/oauth.json';

export function WelcomeDialog() {
  const settings = useUi((s) => s.settings);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const configured = settings?.oauth_configured ?? false;
  const recheck = () => {
    ipc.getSettings().then((s) => {
      useUi.getState().setSettings(s);
      if (!s.oauth_configured) setMessage(`${OAUTH_PATH} is still missing or malformed.`);
    }).catch((e: unknown) => setMessage(String(e)));
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
    <div className="welcome-scrim">
      <div className="welcome-root scope-root" role="dialog" aria-modal="true" aria-label="Welcome">
        <h2 className="scope-title-row">
          <span className="scope-title">{configured ? 'Add your first Google account' : 'Welcome to Unified Google Calendar'}</span>
        </h2>
        <div className="welcome-body">
          {configured ? (
            <p className="welcome-text">Sign in with the Google account you want to see first. More accounts and iCal calendars can be added later in Settings.</p>
          ) : (
            <>
              <p className="welcome-text">{`This app needs your own Google Cloud OAuth client. Follow docs/09-setup-usuario.md section A, then save the client id and secret as ${OAUTH_PATH}:`}</p>
              <pre className="welcome-code">{'{ "client_id": "…apps.googleusercontent.com", "client_secret": "…" }'}</pre>
              <ol className="welcome-steps">
                <li>Create a project in Google Cloud and enable the Google Calendar API.</li>
                <li>Configure the OAuth consent screen (external, published) with the calendar scopes.</li>
                <li>Create an OAuth client of type Desktop app and copy its id and secret into the file.</li>
              </ol>
            </>
          )}
          {message ? <p className="welcome-message">{message}</p> : null}
        </div>
        <div className="scope-footer">
          <div className="scope-ok-wrap">
            {configured ? (
              <button className="scope-ok" type="button" disabled={busy} onClick={() => void addAccount()}>
                <span className="scope-ok-ripple">
                  <span className="scope-ok-ripple-inner"></span>
                </span>
                <span className="scope-ok-n34"></span>
                <span className="scope-ok-hit"></span>
                <span className="scope-ok-label">Add Google account</span>
              </button>
            ) : (
              <button className="scope-ok" type="button" onClick={recheck}>
                <span className="scope-ok-ripple">
                  <span className="scope-ok-ripple-inner"></span>
                </span>
                <span className="scope-ok-n34"></span>
                <span className="scope-ok-hit"></span>
                <span className="scope-ok-label">I created the file, continue</span>
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
