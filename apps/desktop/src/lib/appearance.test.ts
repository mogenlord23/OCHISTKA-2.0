import { describe, expect, it } from 'vitest';
import {
  APPEARANCE_KINDS,
  COLOR_SCHEMES,
  CURSORS,
  CURSOR_DEFS,
  cursorSvg,
  ACCENT_COLORS,
  ARROW_PATH,
  WALLPAPERS,
  SOLID_WALLPAPERS,
  defaultTileId,
  tilesFor,
} from './appearance';
import { WALLPAPER_PHOTOS } from './appearanceWallpapers';

describe('appearance tiles', () => {
  it('has unique ids inside each kind and a label on every tile', () => {
    for (const kind of APPEARANCE_KINDS) {
      const tiles = tilesFor(kind);
      expect(tiles.length).toBeGreaterThan(0);
      expect(new Set(tiles.map((tile) => tile.id)).size).toBe(tiles.length);
      for (const tile of tiles) {
        expect(tile.label.trim().length).toBeGreaterThan(0);
        expect(tile.description.trim().length).toBeGreaterThan(0);
      }
    }
  });

  it('puts the solid styled wallpapers first, then the owner photo set with thumbnails', () => {
    expect(WALLPAPER_PHOTOS).toHaveLength(146);
    expect(SOLID_WALLPAPERS).toHaveLength(7);
    expect(WALLPAPERS).toHaveLength(SOLID_WALLPAPERS.length + 146);
    expect(WALLPAPERS.slice(0, SOLID_WALLPAPERS.length)).toEqual(SOLID_WALLPAPERS);
    for (const tile of SOLID_WALLPAPERS) {
      expect(tile.id).toMatch(/^solid-/);
      expect(tile.preview).toMatch(/^#[0-9A-F]{6}$/);
      expect(tile.thumb).toBeUndefined();
    }
    expect(SOLID_WALLPAPERS[0].id).toMatch(/^solid-acid-/);
    expect(SOLID_WALLPAPERS.some((t) => t.id === 'solid-black')).toBe(true);
    expect(SOLID_WALLPAPERS.some((t) => t.id === 'solid-white')).toBe(true);
    expect(SOLID_WALLPAPERS.some((t) => t.id === 'solid-red')).toBe(true);
    expect(SOLID_WALLPAPERS.filter((t) => t.id.startsWith('solid-acid-'))).toHaveLength(4);
    const photos = WALLPAPERS.slice(SOLID_WALLPAPERS.length);
    expect(photos.every((tile) => tile.thumb && /^photo-[0-9a-f]{16}$/.test(tile.id))).toBe(true);
  });

  it('has 53 cursor schemes in 11 styles: 12 plain and 41 patterned, drawn as SVG', () => {
    expect(CURSOR_DEFS).toHaveLength(53);
    expect(CURSOR_DEFS.filter((d) => d.cursor.pattern === 'plain')).toHaveLength(12);
    const patterns = new Set(CURSOR_DEFS.map((d) => d.cursor.pattern));
    expect([...patterns].sort()).toEqual(
      [
        'checker',
        'cross',
        'diamonds',
        'dots',
        'gradient',
        'neon',
        'plain',
        'rings',
        'split',
        'stripes',
        'zigzag',
      ].sort(),
    );
    const svg = cursorSvg(CURSOR_DEFS[20].cursor, 'x');
    expect(svg).toContain('id="p-x"');
    expect(svg).toContain('stroke="#fff"');
    expect(svg).toContain('viewBox="0 0 64 64"');
    expect(svg).toContain(ARROW_PATH);
    expect(cursorSvg(CURSOR_DEFS[0].cursor, 'y')).not.toContain('<pattern');
    for (const def of CURSOR_DEFS) {
      const markup = cursorSvg(def.cursor, def.id);
      expect(markup).toContain(ARROW_PATH);
      // Карточки выбора статичны: ни анимации, ни вращения.
      expect(markup).not.toMatch(/<animate|animateTransform|rotate\(|@keyframes/);
      if (def.cursor.pattern !== 'plain' && def.cursor.pattern !== 'neon') {
        expect(markup).toContain(`id="p-${def.id}"`);
      }
    }
  });

  it('has unique accent colours (no near twins), hues grouped, white and black last', () => {
    expect(COLOR_SCHEMES).toHaveLength(ACCENT_COLORS.length);
    expect(ACCENT_COLORS.length).toBeGreaterThanOrEqual(60);
    expect(new Set(ACCENT_COLORS.map((c) => c.toUpperCase())).size).toBe(ACCENT_COLORS.length);
    const rgb = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
    for (let i = 0; i < ACCENT_COLORS.length; i++) {
      for (let j = i + 1; j < ACCENT_COLORS.length; j++) {
        const [a, b] = [rgb(ACCENT_COLORS[i]), rgb(ACCENT_COLORS[j])];
        const distance = Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
        expect(distance, `${ACCENT_COLORS[i]} ~ ${ACCENT_COLORS[j]}`).toBeGreaterThanOrEqual(20);
      }
    }
    for (const tile of COLOR_SCHEMES) expect(tile.preview).toMatch(/^#[0-9A-F]{6}$/);
    expect(COLOR_SCHEMES.find(({ id }) => id === 'color-01')?.preview).toBe('#B4FF00');
    expect(ACCENT_COLORS.slice(-7)).toContain('#FFFFFF');
    expect(ACCENT_COLORS.at(-1)).toBe('#000000');
  });

  it('shares the arrow outline with the Rust cursor renderer', () => {
    expect(ARROW_PATH.startsWith('M12,7')).toBe(true);
  });

  it('defaults to the first tile of each kind', () => {
    expect(defaultTileId('wallpaper')).toBe(WALLPAPERS[0].id);
    expect(defaultTileId('cursor')).toBe('cursor-white');
    expect(defaultTileId('color')).toBe(COLOR_SCHEMES[0].id);
  });
});
