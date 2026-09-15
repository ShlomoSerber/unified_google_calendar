import { format } from 'date-fns';
import { ipc } from '../ipc';
import { inZone } from '../lib/dates';
import { useUi, type ViewKind } from '../state/ui';
import './TopBar.css';

// Component 2 of docs/04 section 4. Every class is a measured node of calendar.google.com's
// header (docs/design/measurements/topbar-light.json, token-spec.json → measured.css), but the
// order and the set of controls are the user's (docs/99, 2026-09-15): drawer, title,
// previous/next, view selector, Today; then Support and Settings at the right. Search, the Calendar/Tasks
// switch, Google apps, the account avatar, the logo and the title were removed: they did nothing.

// Material Design icon paths (Apache 2.0), read from the SVGs calendar.google.com inlines
// (probed 2026-09-14 with scripts/measure/session.mjs on header[role=banner]).
const ICON = {
  menu: 'M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z',
  chevronLeft: 'M15.41 7.41L14 6l-6 6 6 6 1.41-1.41L10.83 12l4.58-4.59z',
  chevronRight: 'M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6-6-6z',
  help: 'M11 18h2v-2h-2v2zm1-16C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm0-14c-2.21 0-4 1.79-4 4h2c0-1.1.9-2 2-2s2 .9 2 2c0 2-3 1.75-3 5h2c0-2.25 3-2.5 3-5 0-2.21-1.79-4-4-4z',
  settings:
    'M13.85 22.25h-3.7c-.74 0-1.36-.54-1.45-1.27l-.27-1.89c-.27-.14-.53-.29-.79-.46l-1.8.72c-.7.26-1.47-.03-1.81-.65L2.2 15.53c-.35-.66-.2-1.44.36-1.88l1.53-1.19c-.01-.15-.02-.3-.02-.46 0-.15.01-.31.02-.46l-1.52-1.19c-.59-.45-.74-1.26-.37-1.88l1.85-3.19c.34-.62 1.11-.9 1.79-.63l1.81.73c.26-.17.52-.32.78-.46l.27-1.91c.09-.7.71-1.25 1.44-1.25h3.7c.74 0 1.36.54 1.45 1.27l.27 1.89c.27.14.53.29.79.46l1.8-.72c.71-.26 1.48.03 1.82.65l1.84 3.18c.36.66.2 1.44-.36 1.88l-1.52 1.19c.01.15.02.3.02.46s-.01.31-.02.46l1.52 1.19c.56.45.72 1.23.37 1.86l-1.86 3.22c-.34.62-1.11.9-1.8.63l-1.8-.72c-.26.17-.52.32-.78.46l-.27 1.91c-.1.68-.72 1.22-1.46 1.22zm-3.23-2h2.76l.37-2.55.53-.22c.44-.18.88-.44 1.34-.78l.45-.34 2.38.96 1.38-2.4-2.03-1.58.07-.56c.03-.26.06-.51.06-.78s-.03-.53-.06-.78l-.07-.56 2.03-1.58-1.39-2.4-2.39.96-.45-.35c-.42-.32-.87-.58-1.33-.77l-.52-.22-.37-2.55h-2.76l-.37 2.55-.53.21c-.44.19-.88.44-1.34.79l-.45.33-2.38-.95-1.39 2.39 2.03 1.58-.07.56a7 7 0 0 0-.06.79c0 .26.02.53.06.78l.07.56-2.03 1.58 1.38 2.4 2.39-.96.45.35c.43.33.86.58 1.33.77l.53.22.38 2.55z',
};

const VIEW_LABEL: Record<ViewKind, string> = { day: 'Day', week: 'Week', month: 'Month', year: 'Year', agenda: 'Schedule' };
const VIEW_UNIT: Record<ViewKind, string> = { day: 'day', week: 'week', month: 'month', year: 'year', agenda: 'week' };

/** "September 2026", or "Sep – Oct 2026" when the visible week spans two months. */
export function rangeTitle(view: ViewKind, date: number, tz: string, weekDays: number[]): string {
  const d = inZone(date, tz);
  if (view === 'week' || view === 'agenda') {
    const first = inZone(weekDays[0] ?? date, tz);
    const last = inZone(weekDays[weekDays.length - 1] ?? date, tz);
    if (first.getMonth() !== last.getMonth()) {
      const fy = first.getFullYear();
      const ly = last.getFullYear();
      return fy === ly ? `${format(first, 'MMM')} – ${format(last, 'MMM yyyy')}` : `${format(first, 'MMM yyyy')} – ${format(last, 'MMM yyyy')}`;
    }
    return format(first, 'MMMM yyyy');
  }
  if (view === 'day') return format(d, 'MMMM d, yyyy');
  if (view === 'year') return format(d, 'yyyy');
  return format(d, 'MMMM yyyy');
}

export interface TopBarProps {
  weekDays: number[];
}

export function TopBar({ weekDays }: TopBarProps) {
  const view = useUi((s) => s.view);
  const date = useUi((s) => s.date);
  const tz = useUi((s) => s.tz);
  const now = useUi((s) => s.now);
  const { today, prev, next, openDialog } = useUi.getState();
  const todayLabel = format(inZone(now, tz), 'EEEE, MMMM d');
  const unit = VIEW_UNIT[view];
  const title = rangeTitle(view, date, tz, weekDays);
  const toggleSidebar = () => useUi.getState().toggleSidebar();
  const openSettings = () => openDialog({ kind: 'settings' });
  const openViewMenu = () => openDialog({ kind: 'view-menu' });
  const openHelp = () => {
    ipc.openUrl('https://github.com/ShlomoSerber/unified_google_calendar#readme').catch(() => undefined);
  };

  return (
    <header className="topbar-root" role="banner">
      <div className="topbar-top-strip"></div>
      <div className="topbar-bar">
        <div className="topbar-left">
          <div className="topbar-drawer" role="button" aria-label="Main drawer" tabIndex={0} onClick={toggleSidebar}>
            <svg className="topbar-drawer-icon" viewBox="0 0 24 24" focusable="false">
              <path className="topbar-n6" d={ICON.menu} />
            </svg>
          </div>
        </div>
        <div className="topbar-center">
          <div className="topbar-n15">
            <div className="topbar-n44">
              <div className="topbar-n45">
                <div className="topbar-n46">
                  <div className="topbar-n48">
                    <div className="topbar-n49">
                      <div className="topbar-month-title">{title}</div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            <div className="topbar-n18">
              <span className="topbar-n26">
                <button className="topbar-prev" aria-label={`Previous ${unit}`} type="button" onClick={prev} data-tooltip={`Previous ${unit}`}>
                  <span className="topbar-n28 ugc-state ugc-state-icon"></span>
                  <span className="topbar-n29">
                    <span className="topbar-n30">
                      <svg className="topbar-prev-icon" viewBox="0 0 24 24" focusable="false">
                        <path className="topbar-n32" d={ICON.chevronLeft} />
                      </svg>
                    </span>
                  </span>
                  <div className="topbar-n33"></div>
                </button>
              </span>
              <span className="topbar-n35">
                <button className="topbar-next" aria-label={`Next ${unit}`} type="button" onClick={next} data-tooltip={`Next ${unit}`}>
                  <span className="topbar-n37 ugc-state ugc-state-icon"></span>
                  <span className="topbar-n38">
                    <span className="topbar-n39">
                      <svg className="topbar-next-icon" viewBox="0 0 24 24" focusable="false">
                        <path className="topbar-n41" d={ICON.chevronRight} />
                      </svg>
                    </span>
                  </span>
                  <div className="topbar-n42"></div>
                </button>
              </span>
            </div>
            <div className="topbar-n86">
              <div className="topbar-n87">
                <div className="topbar-n88">
                  <div className="topbar-n89">
                    <div className="topbar-n90">
                      <button className="topbar-view-button" type="button" aria-haspopup="menu" onClick={openViewMenu}>
                        <span className="topbar-n92 ugc-state"></span>
                        <span className="topbar-n93"></span>
                        <span className="topbar-view-stack" aria-hidden="true">
                          {Object.values(VIEW_LABEL).map((l) => (
                            <span className="topbar-view-label topbar-view-ghost" key={l}>{l}</span>
                          ))}
                          <span className="topbar-view-label">{VIEW_LABEL[view]}</span>
                        </span>
                        <span className="topbar-n95">
                          <i className="topbar-view-arrow">{"arrow_drop_down"}</i>
                        </span>
                      </button>
                    </div>
                  </div>
                  <div className="topbar-n97"></div>
                </div>
              </div>
            </div>
            <span className="topbar-n19">
              <div className="topbar-n20">
                <button className="topbar-today" aria-label={`Today, ${todayLabel}`} type="button" onClick={today} data-tooltip={todayLabel}>
                  <span className="topbar-n22 ugc-state"></span>
                  <span className="topbar-n23"></span>
                  <span className="topbar-today-label">{"Today"}</span>
                </button>
              </div>
            </span>
          </div>
          <div className="topbar-right">
            <div className="topbar-n52">
              <div className="topbar-n53">
                <div className="topbar-n54">
                  <div className="topbar-n63">
                    <span className="topbar-n64">
                      <button className="topbar-support" aria-label="Support" type="button" onClick={openHelp} data-tooltip={"Support"}>
                        <span className="topbar-n66 ugc-state ugc-state-icon"></span>
                        <span className="topbar-n67">
                          <span className="topbar-n68">
                            <svg className="topbar-support-icon" viewBox="0 0 24 24" focusable="false">
                              <path className="topbar-n70" d={ICON.help} />
                            </svg>
                          </span>
                        </span>
                        <div className="topbar-n71"></div>
                      </button>
                    </span>
                  </div>
                  <div className="topbar-n73">
                    <div className="topbar-n74">
                      <div className="topbar-n75">
                        <span className="topbar-n76">
                          <button className="topbar-settings" aria-label="Settings menu" type="button" onClick={openSettings} data-tooltip={"Settings menu"}>
                            <span className="topbar-n78 ugc-state ugc-state-icon"></span>
                            <span className="topbar-settings-icon">
                              <svg className="topbar-n80" viewBox="0 0 24 24" focusable="false">
                                <path className="topbar-n81" d={ICON.settings} />
                                <circle className="topbar-n82" cx="12" cy="12" r="3.5" />
                              </svg>
                            </span>
                            <div className="topbar-n83"></div>
                          </button>
                        </span>
                      </div>
                      <div className="topbar-n85"></div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
      <div className="topbar-bottom-strip"></div>
    </header>
  );
}
