// Fetches the visible range with `get_view` and refreshes it when `calendar:updated`
// intersects it (docs/02 section 3.2). The payload is the only calendar data in the UI.
import { useEffect, useRef, useState } from 'react';
import { ipc, intersects, onCalendarUpdated } from '../ipc';
import type { ViewPayload } from '../types/ipc';

export const VIEW_MARGIN_SECONDS = 7 * 86_400;

export interface ViewData {
  payload: ViewPayload | null;
  error: string | null;
  reload: () => void;
}

export function useViewData(from: number, to: number, tz: string): ViewData {
  const [payload, setPayload] = useState<ViewPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [version, setVersion] = useState(0);
  const range = useRef({ from, to });
  const rFrom = from - VIEW_MARGIN_SECONDS;
  const rTo = to + VIEW_MARGIN_SECONDS;

  useEffect(() => {
    range.current = { from: rFrom, to: rTo };
  }, [rFrom, rTo]);

  useEffect(() => {
    let cancelled = false;
    ipc
      .getView(rFrom, rTo, tz)
      .then((p) => {
        if (!cancelled) {
          setPayload(p);
          setError(null);
        }
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [rFrom, rTo, tz, version]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onCalendarUpdated((u) => {
      if (u.calendar_ids.length === 0 || intersects(u, range.current.from, range.current.to)) {
        setVersion((v) => v + 1);
      }
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => undefined);
    return () => unlisten?.();
  }, []);

  return { payload, error, reload: () => setVersion((v) => v + 1) };
}
