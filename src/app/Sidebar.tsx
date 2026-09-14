import { MiniCalendar } from './MiniCalendar';
import { CalendarList } from './CalendarList';
import './Sidebar.css';

// The drawer (docs/design/measurements/sidebar-light.json). Between the mini calendar and the
// calendar list Google shows "Meet with…" (people search) and "Booking pages", Workspace
// features outside version 1 (docs/01 section 3). They stay as inert boxes so every measured
// offset below them holds; each one says it is unavailable (docs/99 entry F4-T3).
const UNAVAILABLE = 'Not available in this version';
const PEOPLE_ICON =
  'M9 13.75c-2.34 0-7 1.17-7 3.5V19h14v-1.75c0-2.33-4.66-3.5-7-3.5zM4.34 17c.84-.58 2.87-1.25 4.66-1.25s3.82.67 4.66 1.25H4.34zM9 12c1.93 0 3.5-1.57 3.5-3.5S10.93 5 9 5 5.5 6.57 5.5 8.5 7.07 12 9 12zm0-5c.83 0 1.5.67 1.5 1.5S9.83 10 9 10s-1.5-.67-1.5-1.5S8.17 7 9 7zm7.04 6.81c1.16.84 1.96 1.96 1.96 3.44V19h4v-1.75c0-2.02-3.5-3.17-5.96-3.44zM15 12c1.93 0 3.5-1.57 3.5-3.5S16.93 5 15 5c-.54 0-1.04.13-1.5.35.63.89 1 1.98 1 3.15s-.37 2.26-1 3.15c.46.22.96.35 1.5.35z';

export function Sidebar() {
  return (
    <div className="sidebar-root">
      <div className="sidebar-inner">
        <div className="sidebar-head"></div>
        <div className="sidebar-scroll">
          <h1 className="sidebar-sr-title">Drawer</h1>
          <MiniCalendar />
          <div className="sidebar-spacer-a"></div>
          <div className="sidebar-people" role="search" aria-disabled="true" title={UNAVAILABLE}>
            <div className="sidebar-people-title">{'Meet with…'}</div>
            <div className="sidebar-people-row">
              <div className="sidebar-people-box">
                <div className="sidebar-people-n249"></div>
                <div className="sidebar-people-field">
                  <div className="sidebar-people-field-inner">
                    <div className="sidebar-people-field-row">
                      <span className="sidebar-people-input-span">
                        <input className="sidebar-people-input" role="combobox" aria-label="Search for people to meet" disabled />
                      </span>
                      <div className="sidebar-people-underline"></div>
                    </div>
                    <span className="sidebar-people-n256"></span>
                  </div>
                </div>
              </div>
              <div className="sidebar-people-overlay">
                <svg className="sidebar-people-icon" viewBox="0 0 24 24" focusable="false">
                  <path className="sidebar-people-icon-path" d={PEOPLE_ICON} />
                </svg>
                <div className="sidebar-people-placeholder">{'Search for people'}</div>
              </div>
            </div>
          </div>
          <div className="sidebar-booking">
            <h2 className="sidebar-booking-sr">Bookable pages</h2>
            <div className="sidebar-booking-box">
              <div className="sidebar-booking-panel" role="complementary" aria-label="Booking pages">
                <div className="sidebar-booking-inner">
                  <button className="sidebar-booking-button" type="button" disabled title={UNAVAILABLE}>
                    <span className="sidebar-booking-ripple"></span>
                    <div className="sidebar-booking-n268">
                      <div className="sidebar-booking-row">
                        <div className="sidebar-booking-label">{'Booking pages'}</div>
                      </div>
                    </div>
                  </button>
                  <span className="sidebar-booking-add-span">
                    <button className="sidebar-booking-add" aria-label="Create appointment schedule" type="button" disabled title={UNAVAILABLE}>
                      <span className="sidebar-booking-add-ripple"></span>
                      <span className="sidebar-booking-add-icon-box">
                        <i className="sidebar-booking-add-icon">add</i>
                      </span>
                      <div className="sidebar-booking-add-overlay"></div>
                    </button>
                    <div className="sidebar-booking-add-tooltip" role="tooltip">
                      {'Create appointment schedule'}
                    </div>
                  </span>
                </div>
                <div className="sidebar-booking-foot">
                  <div className="sidebar-booking-foot-inner"></div>
                </div>
              </div>
            </div>
          </div>
          <div className="sidebar-spacer-b">
            <div className="sidebar-spacer-b-inner"></div>
          </div>
          <CalendarList />
        </div>
        <div className="sidebar-foot"></div>
      </div>
    </div>
  );
}
