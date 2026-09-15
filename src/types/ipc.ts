// Hand-written mirror of src-tauri/src/commands/types.rs (docs/02-arquitectura.md section 5).
// Every change here updates the Rust side and regenerates src/types/fixtures with `cargo test`.
// src/types/ipc.test.ts imports the fixtures with these types so a drift fails typecheck.

export type AccountKind = 'local' | 'google' | 'ical';
export type SyncState = 'idle' | 'syncing' | 'error' | 'auth_required';
export type AccessRole = 'owner' | 'writer' | 'reader' | 'freeBusyReader';
export type ResponseStatus = 'needsAction' | 'declined' | 'tentative' | 'accepted';
export type EventStatus = 'confirmed' | 'tentative' | 'cancelled';
export type Transparency = 'opaque' | 'transparent';
export type EditScope = 'this' | 'following' | 'all';

export interface AccountInfo {
  id: string;
  kind: AccountKind;
  email: string | null;
  display_name: string;
  sort_order: number;
  sync_state: SyncState;
  sync_error: string | null;
  last_sync_at: number | null;
}

export interface Reminder {
  method: 'popup' | 'email';
  minutes: number;
}

export interface CalendarInfo {
  id: string;
  account_id: string;
  summary: string;
  description: string | null;
  color_bg: string;
  color_fg: string;
  access_role: AccessRole;
  is_primary: boolean;
  visible: boolean;
  hidden_remote: boolean;
  time_zone: string | null;
  default_reminders: Reminder[];
  sort_order: number;
  is_local: boolean;
}

export interface CalendarVisibility {
  account_id: string;
  id: string;
  visible: boolean;
}

export interface ViewOccurrence {
  id: string;
  event_id: string;
  calendar_id: string;
  account_id: string;
  title: string | null;
  /** UTC seconds. All-day: midnight UTC of the start date; `end` exclusive. */
  start: number;
  end: number;
  all_day: boolean;
  color_bg: string;
  color_fg: string;
  color_id: string | null;
  status: EventStatus;
  my_response: ResponseStatus | null;
  is_recurring: boolean;
  has_meet: boolean;
  attendee_count: number;
  conflict_with: string | null;
  also_in: string[];
  is_local: boolean;
  transparency: Transparency | null;
  /** Shown as the chip's third line, like Google. */
  location: string | null;
}

export interface ViewPayload {
  from: number;
  to: number;
  occurrences: ViewOccurrence[];
  calendars: CalendarVisibility[];
}

export interface AttendeeInfo {
  email: string;
  display_name: string | null;
  response_status: ResponseStatus;
  organizer: boolean;
  self: boolean;
  optional: boolean;
}

export interface AlsoIn {
  account_id: string;
  account_name: string;
  occurrence_id: string;
}

export interface ConflictInfo {
  occurrence_id: string;
  title: string | null;
  start: number;
  end: number;
  account_name: string;
}

export interface ConferencePhone {
  /** "(AR) +54 11 3986-3700" as Google labels it. */
  label: string;
  uri: string | null;
  pin: string | null;
  /** "More phone numbers" page. */
  more_url: string | null;
}

export interface EventDetail {
  occurrence_id: string;
  event_id: string;
  master_id: string | null;
  calendar_id: string;
  account_id: string;
  account_name: string;
  account_email: string | null;
  calendar_name: string;
  is_local: boolean;
  title: string | null;
  description: string | null;
  location: string | null;
  start: number;
  end: number;
  all_day: boolean;
  time_zone: string | null;
  status: EventStatus;
  color_bg: string;
  color_fg: string;
  color_id: string | null;
  transparency: Transparency | null;
  visibility: string | null;
  is_recurring: boolean;
  is_exception: boolean;
  recurrence_text: string | null;
  recurrence: string[];
  meet_link: string | null;
  conference_label: string | null;
  /** Dial-in entry of the conference, when Google provides one. */
  conference_phone: ConferencePhone | null;
  html_link: string | null;
  attendees: AttendeeInfo[];
  organizer: AttendeeInfo | null;
  my_response: ResponseStatus | null;
  use_default_reminders: boolean;
  reminders: Reminder[];
  also_in: AlsoIn[];
  conflict_with: ConflictInfo | null;
  can_edit: boolean;
  can_delete: boolean;
  can_rsvp: boolean;
}

export interface EventDraft {
  account_id: string;
  calendar_id: string;
  title: string | null;
  description: string | null;
  location: string | null;
  all_day: boolean;
  start: number | null;
  end: number | null;
  start_date: string | null;
  end_date: string | null;
  time_zone: string | null;
  recurrence: string[];
  attendees: string[];
  reminders: Reminder[] | null;
  color_id: string | null;
  add_meet: boolean;
  transparency: Transparency | null;
  visibility: string | null;
}

export interface ColorEntry {
  id: string;
  name: string;
  bg: string;
}

export interface ColorPalette {
  events: ColorEntry[];
}

export interface CalendarKey {
  account_id: string;
  calendar_id: string;
}

export interface Settings {
  primary_tz: string;
  secondary_tz: string | null;
  week_start: number;
  hour_format: '24' | '12';
  theme: 'system';
  default_calendar: CalendarKey | null;
  data_window_past_days: number;
  data_window_future_days: number;
  webhook_port: number;
  public_base_url: string | null;
  push_enabled: boolean;
  holidays_account: string | null;
  push_error: string | null;
}

// Events emitted from Rust.

export interface CalendarUpdated {
  from: number;
  to: number;
  calendar_ids: CalendarKey[];
}

export interface SyncStatus {
  account_id: string;
  state: SyncState;
  message: string | null;
  at: number;
}

export interface ShowEvent {
  occurrence_id: string;
}

export interface GoaAccount {
  id: string;
  identity: string;
  calendar_disabled: boolean;
}

export interface GoaStatus {
  accounts: GoaAccount[];
  prompt_done: boolean;
}

export interface PushTestResult {
  ok: boolean;
  message: string;
}

export const EVENT_CALENDAR_UPDATED = 'calendar:updated';
export const EVENT_SYNC_STATUS = 'sync:status';
export const EVENT_ACCOUNT_CHANGED = 'account:changed';
export const EVENT_WINDOW_SHOW_EVENT = 'window:show-event';
/** Emitted by Rust at every minute boundary; payload is the UTC timestamp (docs/99 F4-T4). */
export const EVENT_CLOCK_MINUTE = 'clock:minute';
