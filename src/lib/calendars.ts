import type { CalendarInfo, CalendarKey } from '../types/ipc';

/** Calendars the user can write to, the default calendar first. */
export function writableCalendars(calendars: CalendarInfo[], defaultKey: CalendarKey | null): CalendarInfo[] {
  const list = calendars.filter((c) => c.access_role === 'owner' || c.access_role === 'writer').sort((a, b) => a.sort_order - b.sort_order);
  if (!defaultKey) return list;
  return [...list].sort((a, b) => Number(!(b.account_id === defaultKey.account_id && b.id === defaultKey.calendar_id)) - Number(!(a.account_id === defaultKey.account_id && a.id === defaultKey.calendar_id)));
}
