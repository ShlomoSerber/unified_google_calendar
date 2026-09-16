import { describe, expect, it } from 'vitest';
import { chipColors } from './colors';

describe('chipColors', () => {
  it('maps a classic calendar colour of the API to its M3 custom-color roles, light and dark', () => {
    // docs/design/m3/theme.json custom_colors → src/styles/palette.ts: tones 40/90/10 and 80/30/90.
    expect(chipColors('#7BD148', 'light')).toEqual({ color: '#2d6c00', container: '#a0f96b', onContainer: '#092100' });
    expect(chipColors('#7bd148', 'dark')).toEqual({ color: '#85dc51', container: '#205100', onContainer: '#a0f96b' });
  });
  it('maps a modern event colour and falls back to a color-mix for an unknown one', () => {
    expect(chipColors('#039be5', 'dark').color).toBe('#90cdff');
    expect(chipColors('#123456', 'light')).toEqual({
      color: '#123456',
      container: 'color-mix(in srgb, #123456 24%, var(--md-sys-color-surface))',
      onContainer: 'var(--md-sys-color-on-surface)',
    });
  });
});
