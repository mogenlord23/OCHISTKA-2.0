import { describe, expect, it } from 'vitest';
import {
  APPEARANCE_KINDS,
  COLOR_SCHEMES,
  CURSORS,
  CURSOR_DEFS,
  cursorSvg,
  ACCENT_COLORS,
  ARROW_POINTS,
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

  it('has 36 cursor schemes: 12 plain, 23 patterned and one animated, drawn as SVG', () => {
    expect(CURSOR_DEFS).toHaveLength(36);
    expect(CURSOR_DEFS.filter((d) => d.cursor.pattern === 'plain')).toHaveLength(12);
    const patterns = new Set(CURSOR_DEFS.map((d) => d.cursor.pattern));
    expect([...patterns].sort()).toEqual(['checker', 'cross', 'dots', 'plain', 'pulse', 'stripes']);
    const svg = cursorSvg(CURSOR_DEFS[20].cursor, 'x');
    expect(svg).toContain('<pattern id="p-x"');
    expect(svg).toContain('stroke="#fff"');
    expect(svg).toContain('viewBox="0 0 32 32"');
    expect(cursorSvg(CURSOR_DEFS[0].cursor, 'y')).not.toContain('<pattern');
  });

  it('draws the animated scheme with SMIL only when motion is allowed, at real 32 px', () => {
    const pulse = CURSOR_DEFS.find((d) => d.cursor.pattern === 'pulse');
    expect(pulse).toBeTruthy();
    const moving = cursorSvg(pulse!.cursor, 'a');
    expect(moving).toContain('<animate');
    expect(moving).toContain('width="32"');
    expect(cursorSvg(pulse!.cursor, 'b', 32, false)).not.toContain('<animate');
    expect(cursorSvg(CURSOR_DEFS[0].cursor, 'c')).not.toContain('<animate');
  });

  it('has 53 accent colours as plain hex swatches, each unique', () => {
    expect(COLOR_SCHEMES).toHaveLength(53);
    expect(new Set(ACCENT_COLORS).size).toBe(53);
    for (const tile of COLOR_SCHEMES) expect(tile.preview).toMatch(/^#[0-9A-F]{6}$/);
  });

  it('centres the arrow in 32×32 with the tip as the first point', () => {
    const xs = ARROW_POINTS.map((p) => p[0]);
    const ys = ARROW_POINTS.map((p) => p[1]);
    expect(Math.min(...xs) + Math.max(...xs)).toBeCloseTo(32, 5);
    expect(Math.min(...ys) + Math.max(...ys)).toBeCloseTo(32, 5);
    expect(ARROW_POINTS[0][0]).toBe(Math.min(...xs));
    expect(ARROW_POINTS[0][1]).toBe(Math.min(...ys));
  });

  it('defaults to the first tile of each kind', () => {
    expect(defaultTileId('wallpaper')).toBe(WALLPAPERS[0].id);
    expect(defaultTileId('cursor')).toBe('cursor-white');
    expect(defaultTileId('color')).toBe(COLOR_SCHEMES[0].id);
  });
});
