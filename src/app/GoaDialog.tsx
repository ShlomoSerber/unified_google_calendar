import { useEffect, useRef, useState } from 'react';
import type { MdDialog } from '@material/web/dialog/dialog.js';
import { ipc } from '../ipc';
import { useUi } from '../state/ui';
import type { GoaStatus } from '../types/ipc';
import './WelcomeDialog.css';

// docs/06 section 4.6: offer to turn off Calendar in GNOME Online Accounts so the panel does
// not show every Google event twice. "No" leaves the manual step of docs/09 section D. The
// dialog answers only through its buttons.
export function GoaDialog() {
  const dialog = useRef<MdDialog>(null);
  const [goa, setGoa] = useState<GoaStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  useEffect(() => {
    dialog.current?.show();
    ipc.goaStatus().then(setGoa).catch((e: unknown) => setMessage(String(e)));
  }, []);
  const answer = async (disable: boolean) => {
    setBusy(true);
    try {
      await ipc.goaDisableCalendars(disable);
      dialog.current?.close();
    } catch (e) {
      setMessage(String(e));
      setBusy(false);
    }
  };
  const n = goa?.accounts.length ?? 0;
  return (
    <md-dialog className="welcome" ref={dialog} aria-label="Online Accounts" oncancel={(e) => e.preventDefault()} onclosed={() => useUi.getState().closeDialog()}>
      <div slot="headline">GNOME Online Accounts</div>
      <div slot="content" className="welcome-content md-typescale-body-medium">
        <p className="welcome-text">{`GNOME's calendar panel already shows ${n} Google account${n === 1 ? '' : 's'}. Turn off their Calendar in Online Accounts so events don't show twice? Mail and contacts keep working; it can be turned back on in Settings → Online Accounts.`}</p>
        {message ? (
          <p className="welcome-message md-typescale-body-small" role="status">
            {message}
          </p>
        ) : null}
      </div>
      <div slot="actions">
        <md-text-button disabled={busy} onclick={() => void answer(false)}>
          No
        </md-text-button>
        <md-filled-button disabled={busy} onclick={() => void answer(true)}>
          Yes
        </md-filled-button>
      </div>
    </md-dialog>
  );
}
