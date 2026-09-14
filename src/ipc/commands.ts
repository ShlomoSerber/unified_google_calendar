// Typed wrappers of the IPC commands (docs/02-arquitectura.md section 5).
// Argument names must match the Rust command parameters (snake_case is kept as-is by Tauri
// when the Rust side uses `rename_all = "snake_case"`; our commands take snake_case names).
import { invoke } from '@tauri-apps/api/core';
import type {
  AccountInfo,
  CalendarInfo,
  ColorPalette,
  EditScope,
  EventDetail,
  EventDraft,
  GoaStatus,
  PushTestResult,
  ResponseStatus,
  Settings,
  ViewPayload,
} from '../types/ipc';

export const ipc = {
  listAccounts: () => invoke<AccountInfo[]>('list_accounts'),
  addAccount: () => invoke<AccountInfo>('add_account'),
  removeAccount: (accountId: string) => invoke<void>('remove_account', { accountId }),
  addIcalCalendar: (name: string, url: string, email: string | null) =>
    invoke<AccountInfo>('add_ical_calendar', { name, url, email }),
  listCalendars: () => invoke<CalendarInfo[]>('list_calendars'),
  setCalendarVisible: (accountId: string, calendarId: string, visible: boolean) =>
    invoke<void>('set_calendar_visible', { accountId, calendarId, visible }),
  getView: (from: number, to: number, tz: string) => invoke<ViewPayload>('get_view', { from, to, tz }),
  getEvent: (occurrenceId: string) => invoke<EventDetail>('get_event', { occurrenceId }),
  createEvent: (draft: EventDraft) => invoke<EventDetail>('create_event', { draft }),
  updateEvent: (occurrenceId: string, draft: EventDraft, scope: EditScope) =>
    invoke<EventDetail>('update_event', { occurrenceId, draft, scope }),
  deleteEvent: (occurrenceId: string, scope: EditScope) =>
    invoke<void>('delete_event', { occurrenceId, scope }),
  rsvp: (occurrenceId: string, status: ResponseStatus, sendUpdates: boolean) =>
    invoke<EventDetail>('rsvp', { occurrenceId, status, sendUpdates }),
  moveEventAccount: (occurrenceId: string, targetAccountId: string, targetCalendarId: string) =>
    invoke<EventDetail>('move_event_account', { occurrenceId, targetAccountId, targetCalendarId }),
  getColors: () => invoke<ColorPalette>('get_colors'),
  getSettings: () => invoke<Settings>('get_settings'),
  setSettings: (settings: Settings) => invoke<Settings>('set_settings', { settings }),
  syncNow: () => invoke<void>('sync_now'),
  openUrl: (url: string) => invoke<void>('open_url', { url }),
  testPush: (url: string) => invoke<PushTestResult>('test_push', { url }),
  goaStatus: () => invoke<GoaStatus>('goa_status'),
  goaDisableCalendars: (disable: boolean) => invoke<GoaStatus>('goa_disable_calendars', { disable }),
};

export type Ipc = typeof ipc;
