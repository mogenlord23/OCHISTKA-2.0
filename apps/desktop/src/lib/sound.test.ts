import { afterEach, describe, expect, it, vi } from 'vitest';
import { playHeehee, readSoundEnabled, writeSoundEnabled } from './sound';

afterEach(() => {
  localStorage.removeItem('soundEnabled');
  vi.unstubAllGlobals();
});

describe('sound store', () => {
  it('defaults to on and round-trips', () => {
    expect(readSoundEnabled()).toBe(true);
    writeSoundEnabled(false);
    expect(readSoundEnabled()).toBe(false);
    writeSoundEnabled(true);
    expect(readSoundEnabled()).toBe(true);
  });

  it('plays only when enabled', () => {
    const play = vi.fn(() => Promise.resolve());
    vi.stubGlobal('Audio', function () {
      return { play };
    });
    playHeehee();
    expect(play).toHaveBeenCalledTimes(1);
    writeSoundEnabled(false);
    playHeehee();
    expect(play).toHaveBeenCalledTimes(1);
  });

  it('survives broken storage', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    expect(readSoundEnabled()).toBe(true);
    vi.restoreAllMocks();
  });
});
