// Typed wrappers of the events Rust emits (docs/02-arquitectura.md section 5).
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  EVENT_ACCOUNT_CHANGED,
  EVENT_CALENDAR_UPDATED,
  EVENT_SYNC_STATUS,
  EVENT_WINDOW_SHOW_EVENT,
  type AccountInfo,
  type CalendarUpdated,
  type ShowEvent,
  type SyncStatus,
} from '../types/ipc';

export const onCalendarUpdated = (cb: (p: CalendarUpdated) => void): Promise<UnlistenFn> =>
  listen<CalendarUpdated>(EVENT_CALENDAR_UPDATED, (e) => cb(e.payload));

export const onSyncStatus = (cb: (p: SyncStatus) => void): Promise<UnlistenFn> =>
  listen<SyncStatus>(EVENT_SYNC_STATUS, (e) => cb(e.payload));

export const onAccountChanged = (cb: (p: AccountInfo[]) => void): Promise<UnlistenFn> =>
  listen<AccountInfo[]>(EVENT_ACCOUNT_CHANGED, (e) => cb(e.payload));

export const onShowEvent = (cb: (p: ShowEvent) => void): Promise<UnlistenFn> =>
  listen<ShowEvent>(EVENT_WINDOW_SHOW_EVENT, (e) => cb(e.payload));

/** Whether a `calendar:updated` range intersects the visible one. */
export function intersects(update: CalendarUpdated, from: number, to: number): boolean {
  return update.from < to && update.to > from;
}
