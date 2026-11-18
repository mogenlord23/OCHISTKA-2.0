import { beforeEach, describe, expect, it } from 'vitest';
import { APPEARANCE_STORAGE_KEY, defaultChoice, loadChoice, saveChoice } from './appearanceStore';

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
    expect(saveChoice('wallpaper', 'ocean', storage)).toBe(true);
    expect(saveChoice('cursor', 'large', storage)).toBe(true);
    expect(loadChoice(storage)).toEqual({
      ...defaultChoice(),
      wallpaper: 'ocean',
      cursor: 'large',
    });
  });

  it('ignores unknown ids and rejects them on save', () => {
    expect(saveChoice('color', 'nope', storage)).toBe(false);
    storage.setItem(APPEARANCE_STORAGE_KEY, JSON.stringify({ wallpaper: 'ghost', color: 'vpn' }));
    expect(loadChoice(storage)).toEqual({ ...defaultChoice(), color: 'vpn' });
  });

  it('falls back to defaults on broken JSON', () => {
    storage.setItem(APPEARANCE_STORAGE_KEY, '{not json');
    expect(loadChoice(storage)).toEqual(defaultChoice());
  });

  it('does not throw when storage is unavailable', () => {
    expect(loadChoice(new BrokenStorage())).toEqual(defaultChoice());
    expect(saveChoice('wallpaper', 'ocean', new BrokenStorage())).toBe(false);
    expect(loadChoice(null)).toEqual(defaultChoice());
    expect(saveChoice('wallpaper', 'ocean', null)).toBe(false);
  });
});
