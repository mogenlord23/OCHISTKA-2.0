import { describe, expect, it } from 'vitest';
import { SECTION_COLORS } from './tokens';
import {
  APPEARANCE_KINDS,
  COLOR_SCHEMES,
  CURSORS,
  WALLPAPERS,
  defaultTileId,
  tilesFor,
} from './appearance';

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

  it('draws wallpapers only as CSS gradients, with no image files', () => {
    for (const tile of WALLPAPERS) {
      expect(tile.preview).toMatch(/gradient\(/);
      expect(tile.preview).not.toMatch(/url\(|\.png|\.jpe?g|\.webp/i);
    }
  });

  it('names cursors without any cursor file', () => {
    expect(CURSORS.length).toBeGreaterThanOrEqual(3);
    for (const tile of CURSORS) {
      expect(tile.id).toMatch(/^[a-z-]+$/);
    }
  });

  it('takes colour tiles from the section palette tokens', () => {
    expect(COLOR_SCHEMES.map((tile) => tile.preview)).toEqual(Object.values(SECTION_COLORS));
    for (const tile of COLOR_SCHEMES) {
      expect(tile.id in SECTION_COLORS).toBe(true);
    }
  });

  it('defaults to the first tile of each kind', () => {
    expect(defaultTileId('wallpaper')).toBe(WALLPAPERS[0].id);
    expect(defaultTileId('cursor')).toBe(CURSORS[0].id);
    expect(defaultTileId('color')).toBe(COLOR_SCHEMES[0].id);
  });
});
