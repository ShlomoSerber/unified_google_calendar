import { render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

// The shell talks to Tauri on mount; in jsdom the IPC is replaced by inert stubs.
vi.mock('@tauri-apps/api/core', () => ({ invoke: () => new Promise(() => undefined) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: () => Promise.resolve(() => undefined) }));

import { App } from './App';

describe('App', () => {
  it('renders the top bar, the drawer and the week grid', () => {
    const { container } = render(<App />);
    expect(screen.getByRole('banner')).toBeTruthy();
    expect(container.querySelector('.topbar-range')?.textContent).toMatch(/\d{4}$/); // range title
    // <md-*> elements are inert in jsdom (vitest.setup.ts): the Today button is found by its label.
    expect(container.querySelector('md-outlined-button[aria-label^="Today, "]')).toBeTruthy();
    expect(container.querySelectorAll('md-menu-item')).toHaveLength(5); // view menu
    expect(screen.getByRole('grid', { name: /\d{4}$/ })).toBeTruthy(); // mini calendar
    expect(screen.getByRole('main')).toBeTruthy(); // the view's own main box
    expect(within(screen.getByRole('main')).getAllByRole('columnheader')).toHaveLength(7); // week header
  });
});
