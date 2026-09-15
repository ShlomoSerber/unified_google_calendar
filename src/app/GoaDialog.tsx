import { useEffect, useState } from 'react';
import { ipc } from '../ipc';
import { useUi } from '../state/ui';
import type { GoaStatus } from '../types/ipc';
import './WelcomeDialog.css';

// docs/06 section 4.6: offer to turn off Calendar in GNOME Online Accounts so the panel does
// not show every Google event twice. "No" leaves the manual step of docs/09 section D.
export function GoaDialog() {
  const [goa, setGoa] = useState<GoaStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const close = () => useUi.getState().closeDialog();
  useEffect(() => {
    ipc.goaStatus().then(setGoa).catch((e: unknown) => setMessage(String(e)));
  }, []);
  const answer = async (disable: boolean) => {
    setBusy(true);
    try {
      await ipc.goaDisableCalendars(disable);
      close();
    } catch (e) {
      setMessage(String(e));
      setBusy(false);
    }
  };
  const n = goa?.accounts.length ?? 0;
  return (
    <div className="welcome-scrim">
      <div className="welcome-root scope-root" role="dialog" aria-modal="true" aria-label="Online Accounts">
        <h2 className="scope-title-row">
          <span className="scope-title">GNOME Online Accounts</span>
        </h2>
        <div className="welcome-body">
          <p className="welcome-text">{`GNOME's calendar panel already shows ${n} Google account${n === 1 ? '' : 's'}. Turn off their Calendar in Online Accounts so events don't show twice? Mail and contacts keep working; it can be turned back on in Settings → Online Accounts.`}</p>
          {message ? <p className="welcome-message">{message}</p> : null}
        </div>
        <div className="scope-footer">
          <div className="scope-cancel-wrap">
            <button className="scope-cancel" type="button" disabled={busy} onClick={() => void answer(false)}>
              <span className="scope-cancel-ripple"></span>
              <span className="scope-cancel-hit"></span>
              <span className="scope-cancel-label">No</span>
            </button>
          </div>
          <div className="scope-ok-wrap">
            <button className="scope-ok" type="button" disabled={busy} onClick={() => void answer(true)}>
              <span className="scope-ok-ripple">
                <span className="scope-ok-ripple-inner"></span>
              </span>
              <span className="scope-ok-n34"></span>
              <span className="scope-ok-hit"></span>
              <span className="scope-ok-label">Yes</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
