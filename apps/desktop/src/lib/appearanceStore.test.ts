import { beforeEach, describe, expect, it } from 'vitest';
import {
  APPEARANCE_STORAGE_KEY,
  applyAppCursor,
  defaultChoice,
  loadChoice,
  saveChoice,
  storedCursorId,
} from './appearanceStore';

class MemoryStorage {
  private data = new Map<string, string>();
  getItem(key: string) {
    return this.data.get(key) ?? null;
  }
  setItem(key: string, value: string) {
    this.data.set(key, value);
  }
}

class BrokenStorage {
  getItem(): string | null {
    throw new Error('blocked');
  }
  setItem(): void {
    throw new Error('blocked');
  }
}

describe('appearance storage', () => {
  let storage: MemoryStorage;

  beforeEach(() => {
    storage = new MemoryStorage();
  });

  it('returns defaults when nothing is stored', () => {
    expect(loadChoice(storage)).toEqual(defaultChoice());
  });

  it('stores one kind and keeps the others', () => {
    expect(saveChoice('wallpaper', 'solid-black', storage)).toBe(true);
    expect(saveChoice('cursor', 'cursor-red', storage)).toBe(true);
    expect(loadChoice(storage)).toEqual({
      ...defaultChoice(),
      wallpaper: 'solid-black',
      cursor: 'cursor-red',
    });
  });

  it('ignores unknown ids and rejects them on save', () => {
    expect(saveChoice('color', 'nope', storage)).toBe(false);
    storage.setItem(
      APPEARANCE_STORAGE_KEY,
      JSON.stringify({ wallpaper: 'ghost', color: 'color-03' }),
    );
    expect(loadChoice(storage)).toEqual({ ...defaultChoice(), color: 'color-03' });
  });

  it('falls back to defaults on broken JSON', () => {
    storage.setItem(APPEARANCE_STORAGE_KEY, '{not json');
    expect(loadChoice(storage)).toEqual(defaultChoice());
  });

  it('does not throw when storage is unavailable', () => {
    expect(loadChoice(new BrokenStorage())).toEqual(defaultChoice());
    expect(saveChoice('wallpaper', 'solid-black', new BrokenStorage())).toBe(false);
    expect(loadChoice(null)).toEqual(defaultChoice());
    expect(saveChoice('wallpaper', 'solid-black', null)).toBe(false);
  });

  it('reports a cursor only when the user chose one', () => {
    expect(storedCursorId(storage)).toBeNull();
    saveChoice('cursor', 'cursor-red', storage);
    expect(storedCursorId(storage)).toBe('cursor-red');
    expect(storedCursorId(null)).toBeNull();
  });

  it('applies and clears the app cursor style', () => {
    applyAppCursor('cursor-red');
    expect(document.getElementById('ochistka-app-cursor')).not.toBeNull();
    applyAppCursor(null);
    expect(document.getElementById('ochistka-app-cursor')).toBeNull();
  });
});
