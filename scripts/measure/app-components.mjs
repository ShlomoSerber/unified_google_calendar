// Root selectors and preparatory actions for the app side of every measured component.
// Names and states mirror scripts/measure/capture.mjs so the diff pairs line up.
const WEEK_TS = Date.UTC(2026, 8, 14, 15) / 1000; // Monday 2026-09-14 12:00 Buenos Aires

const base = [{ type: 'view', view: 'week' }, { type: 'date', ts: WEEK_TS }, { type: 'calendars', only: ['UGC Fixtures'] }, { type: 'wait', ms: 600 }];
const showAll = [{ type: 'calendars', only: [] }];

export const APP_COMPONENTS = {
  topbar: { root: 'header[role=banner]', actions: base },
  sidebar: { root: '.sidebar-root', actions: base },
  create_button: {
    root: '.create-button-root',
    actions: base,
    states: {
      open: { root: '.create-menu-root', actions: [...base, { type: 'click', selector: '.create-button-root' }, { type: 'wait', ms: 400 }], reset: [{ type: 'click', selector: '.create-button-root' }] },
    },
  },
  mini_calendar: {
    root: '.sidebar-minical',
    actions: base,
    states: {
      other_month: { actions: [...base, { type: 'click', selector: '.sidebar-minical-next' }, { type: 'wait', ms: 300 }], reset: [{ type: 'click', selector: '.sidebar-minical-prev' }] },
      next_week: { actions: [...base, { type: 'date', ts: WEEK_TS + 9 * 86_400 }, { type: 'wait', ms: 300 }], reset: [{ type: 'date', ts: WEEK_TS }] },
    },
  },
  calendar_list: {
    root: '.sidebar-list-panel',
    actions: base,
    states: {
      hover: { actions: [...base, { type: 'hover', selector: '.sidebar-list-row-inner' }, { type: 'wait', ms: 300 }], reset: [{ type: 'unhover', selector: '.sidebar-list-row-inner' }] },
    },
  },
  week_header: { root: '.week-header-root', actions: base },
  allday_row: { root: '.allday-root', actions: base },
  // The reference dumps of the grid are taken at the 07:00 scroll position (capture.mjs GRID_SCROLL).
  hour_grid: { root: '.week-root', actions: [...base, { type: 'scroll', selector: '.week-scroller', y: 420 }, { type: 'wait', ms: 200 }] },
  now_line: { root: '.week-now-line', actions: base, states: { dot: { root: '.week-now-dot' } } },
  event_chip: {
    root: '[data-title="Weekend"]',
    actions: [...base, { type: 'scroll', selector: '.week-scroller', y: 420 }, { type: 'wait', ms: 200 }],
    states: {
      sixty: { root: '[data-title="Sixty"]' },
      fifteen: { root: '[data-title="Fifteen"]' },
      thirty: { root: '[data-title="Thirty"]' },
      forty_five: { root: '[data-title="Forty-five"]' },
      ninety: { root: '[data-title="Ninety"]' },
      overlap_a: { root: '[data-title="Overlap A"]' },
      overlap_b: { root: '[data-title="Overlap B"]' },
      triple_a: { root: '[data-title="Triple A"]' },
      triple_b: { root: '[data-title="Triple B"]' },
      triple_c: { root: '[data-title="Triple C"]' },
      all_day: { root: '[data-title="All-day one"]' },
      with_location: { root: '[data-title="With Meet"]' },
      hover: { root: '[data-title="Weekend"]', actions: [...base, { type: 'hover', selector: '[data-title="Weekend"]' }, { type: 'wait', ms: 300 }], reset: [{ type: 'unhover', selector: '[data-title="Weekend"]' }] },
      past: { root: '[data-title="Sixty"]' },
      // Google's reference is the attendee copy on the primary calendar (shown together with the
      // fixtures calendar; the app deduplicates both copies into one chip, docs/03 section 4).
      tentative: { root: '[data-title="Tentative"]', actions: [...base, { type: 'calendars', only: ['UGC Fixtures', 'shlomo.serber@greelow.com'] }, { type: 'scroll', selector: '.week-scroller', y: 420 }, { type: 'wait', ms: 400 }] },
      declined: { root: '[data-title="Declined"]', actions: [...base, { type: 'calendars', only: ['UGC Fixtures', 'shlomo.serber@greelow.com'] }, { type: 'scroll', selector: '.week-scroller', y: 420 }, { type: 'wait', ms: 400 }] },
    },
  },
};

// Phase 7 views: the same reference week; the day and month views open on 2026-09-14.
APP_COMPONENTS.day_view = { root: '.day-main', actions: [{ type: 'view', view: 'day' }, { type: 'date', ts: WEEK_TS }, { type: 'calendars', only: ['UGC Fixtures'] }, { type: 'wait', ms: 600 }, { type: 'scroll', selector: '.week-scroller', y: 420 }, { type: 'wait', ms: 200 }], reset: [{ type: 'view', view: 'week' }] };
APP_COMPONENTS.month_view = { root: '.month-main', actions: [{ type: 'view', view: 'month' }, { type: 'date', ts: WEEK_TS }, { type: 'calendars', only: ['UGC Fixtures'] }, { type: 'wait', ms: 800 }], reset: [{ type: 'view', view: 'week' }] };
APP_COMPONENTS.agenda_view = { root: '.agenda-main', actions: [{ type: 'view', view: 'agenda' }, { type: 'date', ts: WEEK_TS }, { type: 'calendars', only: ['UGC Fixtures'] }, { type: 'wait', ms: 800 }], reset: [{ type: 'view', view: 'week' }] };

const gridBase = [...base, { type: 'scroll', selector: '.week-scroller', y: 420 }, { type: 'wait', ms: 200 }];
const popupOn = (title) => [...gridBase, { type: 'click', selector: `[data-title="${title}"]` }, { type: 'wait', ms: 1200 }];
APP_COMPONENTS.event_popup = {
  root: '.popup-root',
  actions: popupOn('Weekend'),
  reset: [{ type: 'key', key: 'Escape' }],
  states: {
    meet: { actions: popupOn('With Meet'), reset: [{ type: 'key', key: 'Escape' }] },
    guests: { actions: popupOn('With guests'), reset: [{ type: 'key', key: 'Escape' }] },
    recurring: { actions: popupOn('Weekly repeat'), reset: [{ type: 'key', key: 'Escape' }] },
    left_chip: { actions: popupOn('Sixty'), reset: [{ type: 'key', key: 'Escape' }] },
  },
};

// Quick create: a real click on Thursday 16:00 (same slot as capture.mjs QC_X/QC_Y).
APP_COMPONENTS.quick_create = {
  root: '.qc-root',
  actions: [...gridBase, { type: 'click_at', x: 366.34 + 9 + 149.8 * 3 + 75, y: 196 + 16 * 60 - 420 + 10 }, { type: 'wait', ms: 800 }],
  reset: [{ type: 'key', key: 'Escape' }],
  states: {
    with_title: { actions: [...gridBase, { type: 'click_at', x: 366.34 + 9 + 149.8 * 3 + 75, y: 196 + 16 * 60 - 420 + 10 }, { type: 'wait', ms: 500 }, { type: 'type', text: 'Title' }, { type: 'wait', ms: 300 }], reset: [{ type: 'key', key: 'Escape' }] },
  },
};

/** Actions that leave the app as the user had it (every calendar visible). */
export const RESTORE = showAll;
