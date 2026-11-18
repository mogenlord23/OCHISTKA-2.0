import { describe, expect, it } from 'vitest';
import { loadSettings, saveSettings } from './appearanceStore';

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

describe('app settings storage', () => {
  it('defaults to full animation and round-trips', () => {
    const storage = new MemoryStorage();
    expect(loadSettings(storage)).toEqual({ reduceMotion: false });
    expect(saveSettings({ reduceMotion: true }, storage)).toBe(true);
    expect(loadSettings(storage)).toEqual({ reduceMotion: true });
  });

  it('does not throw when storage is unavailable or broken', () => {
    expect(loadSettings(new BrokenStorage())).toEqual({ reduceMotion: false });
    expect(saveSettings({ reduceMotion: true }, new BrokenStorage())).toBe(false);
    expect(loadSettings(null)).toEqual({ reduceMotion: false });
  });
});
