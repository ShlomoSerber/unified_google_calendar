import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

// The shell talks to Tauri on mount; in jsdom the IPC is replaced by inert stubs.
vi.mock('@tauri-apps/api/core', () => ({ invoke: () => new Promise(() => undefined) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: () => Promise.resolve(() => undefined) }));

import { App } from './App';

describe('App', () => {
  it('renders the top bar, the drawer and the week grid', () => {
    render(<App />);
    expect(screen.getByRole('banner')).toBeTruthy();
    expect(screen.getByRole('heading', { level: 1, name: 'Calendar' })).toBeTruthy();
    expect(screen.getByRole('button', { name: /^Today, / })).toBeTruthy();
    expect(screen.getByRole('grid', { name: /\d{4}$/ })).toBeTruthy(); // mini calendar
    expect(screen.getByRole('main')).toBeTruthy(); // the view's own main box
    expect(screen.getAllByRole('columnheader').filter((e) => e.tagName === 'DIV')).toHaveLength(7); // week header (the mini calendar has 7 <th>)
  });
});
