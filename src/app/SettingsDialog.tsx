import { useEffect, useState } from 'react';
import { ipc } from '../ipc';
import { useUi } from '../state/ui';
import type { GoaStatus, Settings } from '../types/ipc';
import './SettingsDialog.css';

// F7-T6 (docs/07 section 5, docs/08): Settings. Google's settings page was not measured (it is a
// separate product surface); this dialog reuses the measured dialog tokens (title row, rows,
// comboboxes and footer buttons of the recurrence and scope dialogs).

const UNAVAILABLE = 'Not available in this version';

export function SettingsDialog() {
  const settings = useUi((s) => s.settings);
  const accounts = useUi((s) => s.accounts);
  const calendars = useUi((s) => s.calendars);
  const syncStatus = useUi((s) => s.syncStatus);
  const [draft, setDraftState] = useState<Settings | null>(null);
  // The form starts from the loaded settings (derived until the user edits).
  const current = draft ?? settings;
  const setDraft = (v: Settings) => setDraftState(v);
  const [goa, setGoa] = useState<GoaStatus | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [icalName, setIcalName] = useState('');
  const [icalUrl, setIcalUrl] = useState('');
  const [icalEmail, setIcalEmail] = useState('');
  const close = () => useUi.getState().closeDialog();

  useEffect(() => {
    ipc.goaStatus().then(setGoa).catch(() => undefined);
  }, []);

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
  const save = () =>
    run('Saving…', async () => {
      if (!current) return 'Nothing to save.';
      const saved = await ipc.setSettings(current);
      useUi.getState().setSettings(saved);
      setDraft(saved);
      return 'Settings saved.';
    });
  const testPush = () =>
    run('Testing the push URL…', async () => {
      const r = await ipc.testPush(current?.public_base_url ?? '');
      return r.message;
    });
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
      setIcalEmail('');
      return `Added ${a.display_name} (read-only iCal).`;
    });
  const remove = (id: string, name: string) => {
    if (!window.confirm(`Remove ${name} from this app? Its events stay in Google.`)) return;
    void run('Removing…', async () => {
      await ipc.removeAccount(id);
      return `Removed ${name}.`;
    });
  };
  const goaToggle = (disable: boolean) =>
    run(disable ? 'Turning off Calendar in Online Accounts…' : 'Updating…', async () => {
      setGoa(await ipc.goaDisableCalendars(disable));
      return disable ? "Calendar is off in GNOME's Online Accounts for your Google accounts." : 'Online Accounts left as they are.';
    });

  if (!current) return null;
  const form = current;
  const googleAccounts = accounts.filter((a) => a.kind === 'google');
  const writable = calendars.filter((c) => c.access_role === 'owner' || c.access_role === 'writer');

  return (
    <div className="settings-scrim" onMouseDown={close}>
      <div className="settings-root scope-root" role="dialog" aria-modal="true" aria-label="Settings" onMouseDown={(e) => e.stopPropagation()}>
        <h2 className="scope-title-row">
          <span className="scope-title">Settings</span>
        </h2>
        <div className="settings-body">
          <section className="settings-section">
            <div className="settings-heading">Accounts</div>
            <ul className="settings-list">
              {accounts.map((a) => {
                const s = syncStatus[a.id];
                const state = s?.state ?? a.sync_state;
                return (
                  <li className="settings-row" key={a.id}>
                    <span className="settings-name">{a.display_name}</span>
                    <span className="settings-meta">
                      {a.kind === 'google' ? (a.email ?? 'Google') : 'iCal · read-only'} · {state}
                      {a.sync_error ? ` · ${a.sync_error}` : ''}
                    </span>
                    {a.kind === 'google' && state === 'auth_required' ? (
                      <button className="settings-button" type="button" disabled={busy} onClick={addGoogle}>
                        Sign in again
                      </button>
                    ) : null}
                    <button className="settings-button" type="button" disabled={busy} onClick={() => remove(a.id, a.display_name)}>
                      Remove
                    </button>
                  </li>
                );
              })}
            </ul>
            <div className="settings-actions">
              <button className="settings-button" type="button" disabled={busy || !form.oauth_configured} title={form.oauth_configured ? undefined : 'Create ~/.config/unified-google-calendar/oauth.json first (docs/09 section A)'} onClick={addGoogle}>
                Add Google account
              </button>
              <button className="settings-button" type="button" disabled={busy} onClick={() => void run('Syncing…', async () => ipc.syncNow().then(() => 'Sync requested.'))}>
                Sync now
              </button>
            </div>
            <div className="settings-actions">
              <input className="settings-input" placeholder="Name (e.g. RappiCard)" value={icalName} onChange={(e) => setIcalName(e.target.value)} />
              <input className="settings-input settings-input-wide" placeholder="Secret iCal address (https://…/basic.ics)" value={icalUrl} onChange={(e) => setIcalUrl(e.target.value)} />
              <input className="settings-input" placeholder="Your e-mail in that calendar (optional)" value={icalEmail} onChange={(e) => setIcalEmail(e.target.value)} />
              <button className="settings-button" type="button" disabled={busy || !icalName || !icalUrl} onClick={addIcal}>
                Add iCal calendar
              </button>
            </div>
          </section>
          <section className="settings-section">
            <div className="settings-heading">Time zones</div>
            <label className="settings-field">
              <span className="settings-label">Primary time zone</span>
              <input className="settings-input" value={form.primary_tz} onChange={(e) => setDraft({ ...form, primary_tz: e.target.value })} />
            </label>
            <label className="settings-field">
              <span className="settings-label">Secondary time zone (empty for none)</span>
              <input className="settings-input" value={form.secondary_tz ?? ''} onChange={(e) => setDraft({ ...form, secondary_tz: e.target.value || null })} />
            </label>
          </section>
          <section className="settings-section">
            <div className="settings-heading">Events</div>
            <label className="settings-field">
              <span className="settings-label">Default calendar for new events</span>
              <select className="settings-select" value={form.default_calendar ? `${form.default_calendar.account_id}|${form.default_calendar.calendar_id}` : ''} onChange={(e) => { const [account_id, calendar_id] = e.target.value.split('|'); setDraft({ ...form, default_calendar: account_id && calendar_id ? { account_id, calendar_id } : null }); }}>
                <option value="">First writable calendar</option>
                {writable.map((c) => (
                  <option key={`${c.account_id}|${c.id}`} value={`${c.account_id}|${c.id}`}>{`${c.summary} · ${accounts.find((a) => a.id === c.account_id)?.display_name ?? ''}`}</option>
                ))}
              </select>
            </label>
            <label className="settings-field">
              <span className="settings-label">Account for the Argentina holidays calendar</span>
              <select className="settings-select" value={form.holidays_account ?? ''} onChange={(e) => setDraft({ ...form, holidays_account: e.target.value || null })}>
                <option value="">None</option>
                {googleAccounts.map((a) => (
                  <option key={a.id} value={a.id}>{a.display_name}</option>
                ))}
              </select>
            </label>
          </section>
          <section className="settings-section">
            <div className="settings-heading">Push notifications (Tailscale Funnel)</div>
            <label className="settings-field">
              <span className="settings-label">Public URL of this machine (https://…ts.net)</span>
              <input className="settings-input settings-input-wide" value={form.public_base_url ?? ''} placeholder="Leave empty to poll every 60 seconds" onChange={(e) => setDraft({ ...form, public_base_url: e.target.value || null })} />
            </label>
            <div className="settings-actions">
              <button className="settings-button" type="button" disabled={busy || !form.public_base_url} onClick={testPush}>
                Test
              </button>
              <span className="settings-meta">{form.push_enabled ? 'Push is on.' : 'Push is off: the app polls every 60 seconds.'}{form.push_error ? ` Last error: ${form.push_error}` : ''}</span>
            </div>
          </section>
          <section className="settings-section">
            <div className="settings-heading">GNOME Online Accounts</div>
            {goa && goa.accounts.length > 0 ? (
              <div className="settings-actions">
                <span className="settings-meta">{`GNOME's calendar panel also shows ${goa.accounts.length} Google account${goa.accounts.length === 1 ? '' : 's'}: ${goa.accounts.map((a) => `${a.identity}${a.calendar_disabled ? ' (Calendar off)' : ''}`).join(', ')}.`}</span>
                <button className="settings-button" type="button" disabled={busy || goa.accounts.every((a) => a.calendar_disabled)} onClick={() => goaToggle(true)}>
                  Turn off their Calendar
                </button>
              </div>
            ) : (
              <span className="settings-meta">No Google accounts in Online Accounts.</span>
            )}
          </section>
          <section className="settings-section">
            <span className="settings-meta" title={UNAVAILABLE}>Week starts on Monday, 24-hour clock, system theme (fixed in version 1).</span>
          </section>
          {message ? <p className="settings-message">{message}</p> : null}
        </div>
        <div className="scope-footer">
          <div className="scope-cancel-wrap">
            <button className="scope-cancel" type="button" onClick={close}>
              <span className="scope-cancel-ripple"></span>
              <span className="scope-cancel-hit"></span>
              <span className="scope-cancel-label">Close</span>
            </button>
          </div>
          <div className="scope-ok-wrap">
            <button className="scope-ok" type="button" disabled={busy} onClick={save}>
              <span className="scope-ok-ripple">
                <span className="scope-ok-ripple-inner"></span>
              </span>
              <span className="scope-ok-n34"></span>
              <span className="scope-ok-hit"></span>
              <span className="scope-ok-label">Save</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
