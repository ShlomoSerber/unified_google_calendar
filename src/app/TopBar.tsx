import { format } from 'date-fns';
import appIcon from '../../src-tauri/icons/32x32.png';
import { ipc } from '../ipc';
import { inZone } from '../lib/dates';
import { useUi, type ViewKind } from '../state/ui';
import './TopBar.css';

// Component 2 of docs/04 section 4. The DOM mirrors calendar.google.com's header
// (docs/design/measurements/topbar-light.json); every class is a measured node
// (docs/design/token-spec.json → src/styles/measured.css). Controls without a version-1
// counterpart (Search, Tasks, Google apps) stay for layout parity and explain themselves.

// Material Design icon paths (Apache 2.0), read from the SVGs calendar.google.com inlines
// (probed 2026-09-14 with scripts/measure/session.mjs on header[role=banner]).
const ICON = {
  menu: 'M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z',
  chevronLeft: 'M15.41 7.41L14 6l-6 6 6 6 1.41-1.41L10.83 12l4.58-4.59z',
  chevronRight: 'M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6-6-6z',
  help: 'M11 18h2v-2h-2v2zm1-16C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm0-14c-2.21 0-4 1.79-4 4h2c0-1.1.9-2 2-2s2 .9 2 2c0 2-3 1.75-3 5h2c0-2.25 3-2.5 3-5 0-2.21-1.79-4-4-4z',
  settings:
    'M13.85 22.25h-3.7c-.74 0-1.36-.54-1.45-1.27l-.27-1.89c-.27-.14-.53-.29-.79-.46l-1.8.72c-.7.26-1.47-.03-1.81-.65L2.2 15.53c-.35-.66-.2-1.44.36-1.88l1.53-1.19c-.01-.15-.02-.3-.02-.46 0-.15.01-.31.02-.46l-1.52-1.19c-.59-.45-.74-1.26-.37-1.88l1.85-3.19c.34-.62 1.11-.9 1.79-.63l1.81.73c.26-.17.52-.32.78-.46l.27-1.91c.09-.7.71-1.25 1.44-1.25h3.7c.74 0 1.36.54 1.45 1.27l.27 1.89c.27.14.53.29.79.46l1.8-.72c.71-.26 1.48.03 1.82.65l1.84 3.18c.36.66.2 1.44-.36 1.88l-1.52 1.19c.01.15.02.3.02.46s-.01.31-.02.46l1.52 1.19c.56.45.72 1.23.37 1.86l-1.86 3.22c-.34.62-1.11.9-1.8.63l-1.8-.72c-.26.17-.52.32-.78.46l-.27 1.91c-.1.68-.72 1.22-1.46 1.22zm-3.23-2h2.76l.37-2.55.53-.22c.44-.18.88-.44 1.34-.78l.45-.34 2.38.96 1.38-2.4-2.03-1.58.07-.56c.03-.26.06-.51.06-.78s-.03-.53-.06-.78l-.07-.56 2.03-1.58-1.39-2.4-2.39.96-.45-.35c-.42-.32-.87-.58-1.33-.77l-.52-.22-.37-2.55h-2.76l-.37 2.55-.53.21c-.44.19-.88.44-1.34.79l-.45.33-2.38-.95-1.39 2.39 2.03 1.58-.07.56a7 7 0 0 0-.06.79c0 .26.02.53.06.78l.07.56-2.03 1.58 1.38 2.4 2.39-.96.45.35c.43.33.86.58 1.33.77l.53.22.38 2.55z',
  apps: 'M6,8c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM12,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM6,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM6,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM12,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM16,6c0,1.1 0.9,2 2,2s2,-0.9 2,-2 -0.9,-2 -2,-2 -2,0.9 -2,2zM12,8c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM18,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM18,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2z',
  calendar:
    'M320-400q-17 0-28.5-11.5T280-440q0-17 11.5-28.5T320-480q17 0 28.5 11.5T360-440q0 17-11.5 28.5T320-400Zm160 0q-17 0-28.5-11.5T440-440q0-17 11.5-28.5T480-480q17 0 28.5 11.5T520-440q0 17-11.5 28.5T480-400Zm160 0q-17 0-28.5-11.5T600-440q0-17 11.5-28.5T640-480q17 0 28.5 11.5T680-440q0 17-11.5 28.5T640-400ZM200-80q-33 0-56.5-23.5T120-160v-560q0-33 23.5-56.5T200-800h40v-80h80v80h320v-80h80v80h40q33 0 56.5 23.5T840-720v560q0 33-23.5 56.5T760-80H200Zm0-80h560v-400H200v400Zm0-480h560v-80H200v80Zm0 0v-80 80Z',
  tasks:
    'M480-80q-83 0-156-31.5T197-197q-54-54-85.5-127T80-480q0-83 31.5-156T197-763q54-54 127-85.5T480-880q65 0 123 19t107 53l-58 59q-38-24-81-37.5T480-800q-133 0-226.5 93.5T160-480q0 133 93.5 226.5T480-160q133 0 226.5-93.5T800-480q0-18-2-36t-6-35l65-65q11 32 17 66t6 70q0 83-31.5 156T763-197q-54 54-127 85.5T480-80Zm-56-216L254-466l56-56 114 114 400-401 56 56-456 457Z',
};

const VIEW_LABEL: Record<ViewKind, string> = { day: 'Day', week: 'Week', month: 'Month', agenda: 'Schedule' };
const VIEW_UNIT: Record<ViewKind, string> = { day: 'day', week: 'week', month: 'month', agenda: 'week' };

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
  return format(d, 'MMMM yyyy');
}

export interface TopBarProps {
  weekDays: number[];
}

export function TopBar({ weekDays }: TopBarProps) {
  const view = useUi((s) => s.view);
  const date = useUi((s) => s.date);
  const tz = useUi((s) => s.tz);
  const accounts = useUi((s) => s.accounts);
  const now = useUi((s) => s.now);
  const { today, prev, next, openDialog } = useUi.getState();
  const todayLabel = format(inZone(now, tz), 'EEEE, MMMM d');
  const unit = VIEW_UNIT[view];
  const title = rangeTitle(view, date, tz, weekDays);
  const primary = accounts.find((a) => a.kind === 'google') ?? accounts[0];
  const email = primary?.email ?? '';
  const orgName = email.includes('@') ? (email.split('@')[1] ?? '').split('.')[0]?.toUpperCase() ?? '' : '';
  const initial = (email || primary?.display_name || 'U').charAt(0).toUpperCase();
  const accountLabel = `Google Account: ${primary?.display_name ?? ''}  \n(${email})`;
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
          <div className="topbar-brand">
            <div className="topbar-n8">
              <span className="topbar-brand-link" aria-label="Calendar">
                <img className="topbar-logo" role="presentation" src={appIcon} alt="" />
                <span className="topbar-title" role="heading" aria-level={1}>{"Calendar"}</span>
              </span>
            </div>
          </div>
        </div>
        <div className="topbar-center">
          <div className="topbar-n13">
            <div className="topbar-n14">
              <div className="topbar-n15">
                <div className="topbar-n16">
                  <div className="topbar-n17">
                    <div className="topbar-n18">
                      <span className="topbar-n19">
                        <div className="topbar-n20">
                          <button className="topbar-today" aria-label={`Today, ${todayLabel}`} type="button" onClick={today}>
                            <span className="topbar-n22"></span>
                            <span className="topbar-n23"></span>
                            <span className="topbar-today-label">{"Today"}</span>
                          </button>
                        </div>
                        <div className="topbar-today-tooltip" role="tooltip">{todayLabel}</div>
                      </span>
                      <span className="topbar-n26">
                        <button className="topbar-prev" aria-label={`Previous ${unit}`} type="button" onClick={prev}>
                          <span className="topbar-n28"></span>
                          <span className="topbar-n29">
                            <span className="topbar-n30">
                              <svg className="topbar-prev-icon" viewBox="0 0 24 24" focusable="false">
                                <path className="topbar-n32" d={ICON.chevronLeft} />
                              </svg>
                            </span>
                          </span>
                          <div className="topbar-n33"></div>
                        </button>
                        <div className="topbar-prev-tooltip" role="tooltip">{`Previous ${unit}`}</div>
                      </span>
                      <span className="topbar-n35">
                        <button className="topbar-next" aria-label={`Next ${unit}`} type="button" onClick={next}>
                          <span className="topbar-n37"></span>
                          <span className="topbar-n38">
                            <span className="topbar-n39">
                              <svg className="topbar-next-icon" viewBox="0 0 24 24" focusable="false">
                                <path className="topbar-n41" d={ICON.chevronRight} />
                              </svg>
                            </span>
                          </span>
                          <div className="topbar-n42"></div>
                        </button>
                        <div className="topbar-next-tooltip" role="tooltip">{`Next ${unit}`}</div>
                      </span>
                    </div>
                  </div>
                  <div className="topbar-n44">
                    <div className="topbar-n45">
                      <div className="topbar-n46">
                        <span className="topbar-title-sr">{title}</span>
                        <div className="topbar-n48">
                          <div className="topbar-n49">
                            <div className="topbar-month-title">{title}</div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <div className="topbar-right">
            <div className="topbar-n52">
              <div className="topbar-n53">
                <div className="topbar-n54">
                  <div className="topbar-n55">
                    <span className="topbar-n56">
                      <button className="topbar-search" aria-label="Search" type="button" title="Search arrives in version 2">
                        <span className="topbar-n58"></span>
                        <span className="topbar-n59">
                          <i className="topbar-search-icon">{"search"}</i>
                        </span>
                        <div className="topbar-n61"></div>
                      </button>
                      <div className="topbar-search-tooltip" role="tooltip">{"Search"}</div>
                    </span>
                  </div>
                  <div className="topbar-n63">
                    <span className="topbar-n64">
                      <button className="topbar-support" aria-label="Support" type="button" onClick={openHelp}>
                        <span className="topbar-n66"></span>
                        <span className="topbar-n67">
                          <span className="topbar-n68">
                            <svg className="topbar-support-icon" viewBox="0 0 24 24" focusable="false">
                              <path className="topbar-n70" d={ICON.help} />
                            </svg>
                          </span>
                        </span>
                        <div className="topbar-n71"></div>
                      </button>
                      <div className="topbar-support-tooltip" role="tooltip">{"Support"}</div>
                    </span>
                  </div>
                  <div className="topbar-n73">
                    <div className="topbar-n74">
                      <div className="topbar-n75">
                        <span className="topbar-n76">
                          <button className="topbar-settings" aria-label="Settings menu" type="button" onClick={openSettings}>
                            <span className="topbar-n78"></span>
                            <span className="topbar-settings-icon">
                              <svg className="topbar-n80" viewBox="0 0 24 24" focusable="false">
                                <path className="topbar-n81" d={ICON.settings} />
                                <circle className="topbar-n82" cx="12" cy="12" r="3.5" />
                              </svg>
                            </span>
                            <div className="topbar-n83"></div>
                          </button>
                          <div className="topbar-settings-tooltip" role="tooltip">{"Settings menu"}</div>
                        </span>
                      </div>
                      <div className="topbar-n85"></div>
                    </div>
                  </div>
                  <div className="topbar-n86">
                    <div className="topbar-n87">
                      <div className="topbar-n88">
                        <div className="topbar-n89">
                          <div className="topbar-n90">
                            <button className="topbar-view-button" type="button" aria-haspopup="menu" onClick={openViewMenu}>
                              <span className="topbar-n92"></span>
                              <span className="topbar-n93"></span>
                              <span className="topbar-view-label">{VIEW_LABEL[view]}</span>
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
                  <div className="topbar-n98">
                    <div className="topbar-n99">
                      <span className="topbar-n100"></span>
                      <span className="topbar-n101">
                        <button className="topbar-switch-cal" aria-label="Switch to Calendar" type="button" aria-pressed="true">
                          <span className="topbar-n103"></span>
                          <div className="topbar-switch-cal-icon">
                            <svg className="topbar-n105" viewBox="0 -960 960 960" focusable="false">
                              <path className="topbar-n106" d={ICON.calendar} />
                            </svg>
                          </div>
                        </button>
                        <div className="topbar-switch-cal-tooltip" role="tooltip">{"Switch to Calendar"}</div>
                      </span>
                      <span className="topbar-n108"></span>
                      <span className="topbar-n109">
                        <button className="topbar-switch-tasks" aria-label="Switch to Tasks" type="button" title="Tasks arrive in version 2">
                          <span className="topbar-n111"></span>
                          <div className="topbar-switch-tasks-icon">
                            <svg className="topbar-n113" viewBox="0 -960 960 960" focusable="false">
                              <path className="topbar-n114" d={ICON.tasks} />
                            </svg>
                          </div>
                        </button>
                        <div className="topbar-switch-tasks-tooltip" role="tooltip">{"Switch to Tasks"}</div>
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div className="topbar-account">
          <div className="topbar-n117">
            <div className="topbar-n118">
              <div className="topbar-n119">
                <div className="topbar-n120">
                  <a className="topbar-apps" role="button" aria-label="Google apps" title="Google apps are not part of this app">
                    <svg className="topbar-apps-icon" viewBox="0 0 24 24" focusable="false">
                      <path className="topbar-n123" d={ICON.apps} />
                    </svg>
                  </a>
                </div>
              </div>
            </div>
            <div className="topbar-account-button" role="button" tabIndex={0} onClick={openSettings}>
              <div className="topbar-n126">
                <div className="topbar-n127">
                  <span className="topbar-org-logo topbar-org-name">{orgName}</span>
                </div>
              </div>
              <div className="topbar-n129">
                <div className="topbar-n130">
                  <a className="topbar-avatar" role="button" aria-label={accountLabel}>
                    <span className="topbar-n132">
                      <span className="topbar-avatar-img topbar-avatar-letter">{initial}</span>
                    </span>
                  </a>
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
