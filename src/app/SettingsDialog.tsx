import { useEffect, useRef, useState } from 'react';
import type { MdDialog } from '@material/web/dialog/dialog.js';
import type { MdOutlinedSelect } from '@material/web/select/outlined-select.js';
import type { MdOutlinedTextField } from '@material/web/textfield/outlined-text-field.js';
import { ipc } from '../ipc';
import { useUi } from '../state/ui';
import type { GoaStatus, Settings } from '../types/ipc';
import './SettingsDialog.css';

// Settings (docs/07 section 5, docs/11 section 7): a 640 px md-dialog with sections for
// accounts, iCal calendars, time zones, events, push notifications and GNOME Online Accounts.

const UNAVAILABLE = 'Not available in this version';
const OAUTH_HINT = 'Create ~/.config/unified-google-calendar/oauth.json first (docs/09 section A)';
/** md-select drops an empty-string value once its options render, so "none" options use this. */
const NONE = 'none';
const fieldValue = (e: Event) => (e.target as MdOutlinedTextField).value;
const selectValue = (e: Event) => (e.target as MdOutlinedSelect).value;

export function SettingsDialog() {
  const settings = useUi((s) => s.settings);
  const accounts = useUi((s) => s.accounts);
  const calendars = useUi((s) => s.calendars);
  const syncStatus = useUi((s) => s.syncStatus);
  const dialog = useRef<MdDialog>(null);
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
  const close = () => dialog.current?.close();

  useEffect(() => {
    dialog.current?.show();
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

  const form = current;
  const googleAccounts = accounts.filter((a) => a.kind === 'google');
  const writable = calendars.filter((c) => c.access_role === 'owner' || c.access_role === 'writer');

  return (
    <md-dialog className="settings" ref={dialog} aria-label="Settings" onclosed={() => useUi.getState().closeDialog()}>
      <div slot="headline">Settings</div>
      <div slot="content" className="settings-content">
        {form ? (
          <>
            <section className="settings-section">
              <h3 className="settings-heading md-typescale-title-medium">Accounts</h3>
              <md-list aria-label="Accounts">
                {accounts.map((a) => {
                  const s = syncStatus[a.id];
                  const state = s?.state ?? a.sync_state;
                  return (
                    <md-list-item key={a.id}>
                      <div slot="headline">{a.display_name}</div>
                      <div slot="supporting-text">{`${a.kind === 'google' ? (a.email ?? 'Google') : a.kind === 'ical' ? 'iCal · read-only' : 'This computer'} · ${state}${a.sync_error ? ` · ${a.sync_error}` : ''}`}</div>
                      <div slot="end" className="settings-item-actions">
                        {a.kind === 'google' && state === 'auth_required' ? (
                          <md-text-button disabled={busy} onclick={addGoogle}>
                            Sign in again
                          </md-text-button>
                        ) : null}
                        {a.kind !== 'local' ? (
                          <md-text-button disabled={busy} onclick={() => remove(a.id, a.display_name)}>
                            Remove
                          </md-text-button>
                        ) : null}
                      </div>
                    </md-list-item>
                  );
                })}
              </md-list>
              <div className="settings-actions">
                <md-text-button disabled={busy || !form.oauth_configured} title={form.oauth_configured ? undefined : OAUTH_HINT} onclick={addGoogle}>
                  <md-icon slot="icon">add</md-icon>
                  Add Google account
                </md-text-button>
                <md-text-button disabled={busy} onclick={() => void run('Syncing…', async () => ipc.syncNow().then(() => 'Sync requested.'))}>
                  <md-icon slot="icon">sync</md-icon>
                  Sync now
                </md-text-button>
              </div>
            </section>
            <section className="settings-section">
              <h3 className="settings-heading md-typescale-title-medium">iCal calendar</h3>
              <md-outlined-text-field label="Name" placeholder="RappiCard" value={icalName} oninput={(e) => setIcalName(fieldValue(e))}></md-outlined-text-field>
              <md-outlined-text-field label="Secret iCal address" placeholder="https://…/basic.ics" value={icalUrl} oninput={(e) => setIcalUrl(fieldValue(e))}></md-outlined-text-field>
              <md-outlined-text-field label="Your e-mail in that calendar (optional)" value={icalEmail} oninput={(e) => setIcalEmail(fieldValue(e))}></md-outlined-text-field>
              <div className="settings-actions">
                <md-text-button disabled={busy || !icalName || !icalUrl} onclick={addIcal}>
                  Add
                </md-text-button>
              </div>
            </section>
            <section className="settings-section">
              <h3 className="settings-heading md-typescale-title-medium">Time zones</h3>
              <md-outlined-text-field label="Primary time zone" value={form.primary_tz} oninput={(e) => setDraft({ ...form, primary_tz: fieldValue(e) })}></md-outlined-text-field>
              <md-outlined-text-field label="Secondary time zone" supporting-text="Empty for none" value={form.secondary_tz ?? ''} oninput={(e) => setDraft({ ...form, secondary_tz: fieldValue(e) || null })}></md-outlined-text-field>
            </section>
            <section className="settings-section">
              <h3 className="settings-heading md-typescale-title-medium">Events</h3>
              <md-outlined-select
                label="Default calendar for new events"
                value={form.default_calendar ? `${form.default_calendar.account_id}|${form.default_calendar.calendar_id}` : NONE}
                onchange={(e) => {
                  const [account_id, calendar_id] = selectValue(e).split('|');
                  setDraft({ ...form, default_calendar: account_id && calendar_id ? { account_id, calendar_id } : null });
                }}
              >
                <md-select-option value={NONE}>
                  <div slot="headline">First writable calendar</div>
                </md-select-option>
                {writable.map((c) => (
                  <md-select-option key={`${c.account_id}|${c.id}`} value={`${c.account_id}|${c.id}`}>
                    <div slot="headline">{`${c.summary} · ${accounts.find((a) => a.id === c.account_id)?.display_name ?? ''}`}</div>
                  </md-select-option>
                ))}
              </md-outlined-select>
              <md-outlined-select label="Account for the Argentina holidays calendar" value={form.holidays_account ?? NONE} onchange={(e) => setDraft({ ...form, holidays_account: selectValue(e) === NONE ? null : selectValue(e) })}>
                <md-select-option value={NONE}>
                  <div slot="headline">None</div>
                </md-select-option>
                {googleAccounts.map((a) => (
                  <md-select-option key={a.id} value={a.id}>
                    <div slot="headline">{a.display_name}</div>
                  </md-select-option>
                ))}
              </md-outlined-select>
            </section>
            <section className="settings-section">
              <h3 className="settings-heading md-typescale-title-medium">Push notifications (Tailscale Funnel)</h3>
              <div className="settings-actions">
                <md-outlined-text-field className="settings-grow" label="Public URL of this machine" placeholder="https://…ts.net" supporting-text="Leave empty to poll every 60 seconds" value={form.public_base_url ?? ''} oninput={(e) => setDraft({ ...form, public_base_url: fieldValue(e) || null })}></md-outlined-text-field>
                <md-text-button disabled={busy || !form.public_base_url} onclick={testPush}>
                  Test
                </md-text-button>
              </div>
              <p className="settings-note md-typescale-body-small">
                {form.push_enabled ? 'Push is on.' : 'Push is off: the app polls every 60 seconds.'}
                {form.push_error ? ` Last error: ${form.push_error}` : ''}
              </p>
            </section>
            <section className="settings-section">
              <h3 className="settings-heading md-typescale-title-medium">GNOME Online Accounts</h3>
              {goa && goa.accounts.length > 0 ? (
                <>
                  <p className="settings-note md-typescale-body-small">{`GNOME's calendar panel also shows ${goa.accounts.length} Google account${goa.accounts.length === 1 ? '' : 's'}: ${goa.accounts.map((a) => `${a.identity}${a.calendar_disabled ? ' (Calendar off)' : ''}`).join(', ')}.`}</p>
                  <div className="settings-actions">
                    <md-text-button disabled={busy || goa.accounts.every((a) => a.calendar_disabled)} onclick={() => goaToggle(true)}>
                      Turn off their Calendar
                    </md-text-button>
                  </div>
                </>
              ) : (
                <p className="settings-note md-typescale-body-small">No Google accounts in Online Accounts.</p>
              )}
            </section>
            <p className="settings-note md-typescale-body-small" title={UNAVAILABLE}>
              Week starts on Monday, 24-hour clock, system theme (fixed in version 1).
            </p>
            {message ? (
              <p className="settings-message md-typescale-body-small" role="status">
                {message}
              </p>
            ) : null}
          </>
        ) : (
          <p className="settings-note md-typescale-body-medium">Loading settings…</p>
        )}
      </div>
      <div slot="actions">
        <md-text-button onclick={close}>Close</md-text-button>
        <md-filled-button disabled={busy || !form} onclick={save}>
          Save
        </md-filled-button>
      </div>
    </md-dialog>
  );
}
