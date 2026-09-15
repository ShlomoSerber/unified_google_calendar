// UI state only (docs/02 section 4, .claude/rules/frontend.md): current date, view, selection,
// open dialog, theme. Calendar data comes from `get_view` and is not duplicated here beyond
// the payload of the visible range.
import { create } from 'zustand';
import { addDays, addMonths } from 'date-fns';
import type { AccountInfo, CalendarInfo, EventDraft, Settings, SyncStatus } from '../types/ipc';
import { inZone, toTs } from '../lib/dates';

export type ViewKind = 'day' | 'week' | 'month' | 'agenda';

export type Dialog =
  | { kind: 'none' }
  | { kind: 'event'; occurrenceId: string; anchor: DOMRect | null }
  | { kind: 'quick-create'; startTs: number; endTs: number; allDay: boolean; anchor: DOMRect | null }
  | { kind: 'full-form'; occurrenceId: string | null; startTs: number; endTs: number; allDay: boolean; draft?: EventDraft }
  | { kind: 'edit-scope'; occurrenceId: string; action: 'update' | 'delete' }
  | { kind: 'settings' }
  | { kind: 'welcome' }
  | { kind: 'goa' }
  | { kind: 'view-menu' }
  | { kind: 'create-menu' };

export interface UiState {
  /** Anchor date of the visible range, UTC seconds (any instant inside the day). */
  date: number;
  /** Current time, UTC seconds; advanced by the Rust `clock:minute` event. */
  now: number;
  view: ViewKind;
  tz: string;
  secondaryTz: string | null;
  dialog: Dialog;
  sidebarOpen: boolean;
  settings: Settings | null;
  accounts: AccountInfo[];
  calendars: CalendarInfo[];
  syncStatus: Record<string, SyncStatus>;
  setView: (view: ViewKind) => void;
  setDate: (ts: number) => void;
  setNow: (ts: number) => void;
  today: () => void;
  next: () => void;
  prev: () => void;
  openDialog: (dialog: Dialog) => void;
  closeDialog: () => void;
  toggleSidebar: () => void;
  setSettings: (s: Settings) => void;
  setAccounts: (a: AccountInfo[]) => void;
  setCalendars: (c: CalendarInfo[]) => void;
  setSyncStatus: (s: SyncStatus) => void;
}

/** Step the anchor date by one unit of the view. */
export function step(date: number, view: ViewKind, direction: 1 | -1, tz: string): number {
  const d = inZone(date, tz);
  switch (view) {
    case 'day':
      return toTs(addDays(d, direction));
    case 'week':
      return toTs(addDays(d, 7 * direction));
    case 'month':
      return toTs(addMonths(d, direction));
    case 'agenda':
      return toTs(addDays(d, 7 * direction));
  }
}

export const useUi = create<UiState>((set, get) => ({
  date: Math.floor(Date.now() / 1000),
  now: Math.floor(Date.now() / 1000),
  view: 'week',
  tz: Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC',
  secondaryTz: null,
  dialog: { kind: 'none' },
  sidebarOpen: true,
  settings: null,
  accounts: [],
  calendars: [],
  syncStatus: {},
  setView: (view) => set({ view }),
  setDate: (date) => set({ date }),
  setNow: (now) => set({ now }),
  today: () => set({ date: get().now }),
  next: () => set({ date: step(get().date, get().view, 1, get().tz) }),
  prev: () => set({ date: step(get().date, get().view, -1, get().tz) }),
  openDialog: (dialog) => set({ dialog }),
  closeDialog: () => set({ dialog: { kind: 'none' } }),
  toggleSidebar: () => set({ sidebarOpen: !get().sidebarOpen }),
  setSettings: (settings) => set({ settings, tz: settings.primary_tz, secondaryTz: settings.secondary_tz }),
  setAccounts: (accounts) => set({ accounts }),
  setCalendars: (calendars) => set({ calendars }),
  setSyncStatus: (s) => set({ syncStatus: { ...get().syncStatus, [s.account_id]: s } }),
}));

/** Worst sync state across accounts, for the top bar indicator (docs/05 section 5). */
export function overallSyncState(accounts: AccountInfo[]): AccountInfo['sync_state'] {
  const order: AccountInfo['sync_state'][] = ['auth_required', 'error', 'syncing', 'idle'];
  for (const s of order) if (accounts.some((a) => a.kind === 'google' && a.sync_state === s)) return s;
  return 'idle';
}
