import { describe, expect, it } from 'vitest';
import { chipBackground } from './colors';

describe('chipBackground', () => {
  it('maps the classic calendar colour the API reports to the measured tone', () => {
    // docs/design/measurements/palette.json: calendarList colorId 9 (#7bd148) paints #7cb342 / #85ad59.
    expect(chipBackground('#7BD148', 'light')).toBe('#7cb342');
    expect(chipBackground('#7bd148', 'dark')).toBe('#85ad59');
  });
  it('maps a modern event colour to its dark tone and leaves unknown colours alone', () => {
    expect(chipBackground('#039be5', 'dark')).toBe('#4b99d2');
    expect(chipBackground('#123456', 'light')).toBe('#123456');
  });
});
