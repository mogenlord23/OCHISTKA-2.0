import { describe, expect, it } from 'vitest';
import { SECTION_COLORS } from './tokens';
import {
  APPEARANCE_KINDS,
  COLOR_SCHEMES,
  CURSORS,
  CURSOR_DEFS,
  cursorSvg,
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

  it('offers the owner photos with thumbnails and solid colours', () => {
    expect(WALLPAPER_PHOTOS).toHaveLength(146);
    expect(WALLPAPERS.filter((tile) => tile.thumb).length).toBe(146);
    expect(WALLPAPERS.filter((tile) => !tile.thumb).every((t) => /^#/.test(t.preview))).toBe(true);
  });

  it('has 35 cursor schemes: 12 plain and 23 patterned, drawn as SVG', () => {
    expect(CURSOR_DEFS).toHaveLength(35);
    expect(CURSOR_DEFS.filter((d) => d.cursor.pattern === 'plain')).toHaveLength(12);
    const patterns = new Set(CURSOR_DEFS.map((d) => d.cursor.pattern));
    expect([...patterns].sort()).toEqual(['checker', 'cross', 'dots', 'plain', 'stripes']);
    const svg = cursorSvg(CURSOR_DEFS[20].cursor, 'x');
    expect(svg).toContain('<pattern id="p-x"');
    expect(svg).toContain('stroke="#fff"');
    expect(cursorSvg(CURSOR_DEFS[0].cursor, 'y')).not.toContain('<pattern');
  });

  it('takes colour tiles from the section palette tokens', () => {
    expect(COLOR_SCHEMES.slice(0, 7).map((tile) => tile.preview)).toEqual(
      Object.values(SECTION_COLORS),
    );
    expect(COLOR_SCHEMES).toHaveLength(7 + 30);
  });

  it('defaults to the first tile of each kind', () => {
    expect(defaultTileId('wallpaper')).toBe(WALLPAPERS[0].id);
    expect(defaultTileId('cursor')).toBe('cursor-white');
    expect(defaultTileId('color')).toBe(COLOR_SCHEMES[0].id);
  });
});
