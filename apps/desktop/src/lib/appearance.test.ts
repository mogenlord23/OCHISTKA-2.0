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

  it('offers exactly the owner photo set, every tile a thumbnail, no solid colours', () => {
    expect(WALLPAPER_PHOTOS).toHaveLength(146);
    expect(WALLPAPERS).toHaveLength(146);
    expect(WALLPAPERS.every((tile) => tile.thumb && /^photo-[0-9a-f]{16}$/.test(tile.id))).toBe(
      true,
    );
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
      if (def.cursor.pattern !== 'plain' && def.cursor.pattern !== 'neon') {
        expect(markup).toContain(`id="p-${def.id}"`);
      }
    }
  });

  it('has 53 accent colours as plain hex swatches, each unique', () => {
    expect(COLOR_SCHEMES).toHaveLength(53);
    expect(new Set(ACCENT_COLORS).size).toBe(53);
    for (const tile of COLOR_SCHEMES) expect(tile.preview).toMatch(/^#[0-9A-F]{6}$/);
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
