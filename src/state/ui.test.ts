import { describe, expect, it } from 'vitest';
import { overallSyncState, step, useUi } from './ui';
import { isoDate } from '../lib/dates';
import type { AccountInfo } from '../types/ipc';

const BA = 'America/Argentina/Buenos_Aires';
const WED = Date.UTC(2026, 8, 16, 18, 30) / 1000;

describe('ui store', () => {
  it('steps by view', () => {
    expect(isoDate(step(WED, 'day', 1, BA), BA)).toBe('2026-09-17');
    expect(isoDate(step(WED, 'week', -1, BA), BA)).toBe('2026-09-09');
    expect(isoDate(step(WED, 'month', 1, BA), BA)).toBe('2026-10-16');
    expect(isoDate(step(WED, 'agenda', 1, BA), BA)).toBe('2026-09-23');
  });

  it('navigates and manages dialogs', () => {
    useUi.setState({ date: WED, view: 'week', tz: BA });
    useUi.getState().next();
    expect(isoDate(useUi.getState().date, BA)).toBe('2026-09-23');
    useUi.getState().setView('month');
    useUi.getState().prev();
    expect(isoDate(useUi.getState().date, BA)).toBe('2026-08-23');
    useUi.getState().openDialog({ kind: 'settings' });
    expect(useUi.getState().dialog.kind).toBe('settings');
    useUi.getState().closeDialog();
    expect(useUi.getState().dialog.kind).toBe('none');
    useUi.getState().today();
    expect(Math.abs(useUi.getState().date - Date.now() / 1000)).toBeLessThan(5);
  });

  it('overall sync state takes the worst account', () => {
    const acc = (sync_state: AccountInfo['sync_state'], kind: AccountInfo['kind'] = 'google'): AccountInfo => ({
      id: sync_state,
      kind,
      email: null,
      display_name: '',
      sort_order: 0,
      sync_state,
      sync_error: null,
      last_sync_at: null,
    });
    expect(overallSyncState([acc('idle'), acc('syncing')])).toBe('syncing');
    expect(overallSyncState([acc('error'), acc('syncing')])).toBe('error');
    expect(overallSyncState([acc('auth_required'), acc('error')])).toBe('auth_required');
    expect(overallSyncState([acc('error', 'local')])).toBe('idle');
  });
});
