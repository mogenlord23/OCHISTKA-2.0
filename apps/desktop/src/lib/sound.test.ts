import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  attachUiSounds,
  playHeehee,
  playUiSound,
  preloadSounds,
  readSoundEnabled,
  resetSoundForTests,
  soundKindFor,
  writeSoundEnabled,
} from './sound';

afterEach(() => {
  localStorage.removeItem('soundEnabled');
  vi.unstubAllGlobals();
  resetSoundForTests();
});

/** Поддельный Web Audio: считает запуски буферов. */
function fakeWebAudio() {
  const started: string[] = [];
  const decoded: string[] = [];
  class FakeContext {
    state = 'running';
    destination = {};
    resume = vi.fn(() => Promise.resolve());
    decodeAudioData = vi.fn(async (data: ArrayBuffer) => {
      const id = `buf${decoded.length}:${data.byteLength}`;
      decoded.push(id);
      return { id } as unknown as AudioBuffer;
    });
    createGain() {
      return { gain: { value: 1 }, connect: (n: unknown) => n };
    }
    createBufferSource() {
      const node = {
        buffer: null as unknown as { id: string },
        connect: (n: unknown) => n,
        start: vi.fn(() => started.push(node.buffer.id)),
      };
      return node;
    }
  }
  vi.stubGlobal('AudioContext', FakeContext);
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => ({ arrayBuffer: async () => new ArrayBuffer(8) })),
  );
  return { started, decoded };
}

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

describe('web audio playback', () => {
  it('decodes every sound in advance and starts a buffer without loading anything on click', async () => {
    const { started, decoded } = fakeWebAudio();
    await preloadSounds();
    expect(decoded).toHaveLength(3);
    const fetchMock = vi.mocked(globalThis.fetch);
    const loads = fetchMock.mock.calls.length;
    playHeehee();
    playUiSound('click');
    expect(started).toHaveLength(2);
    expect(fetchMock.mock.calls.length).toBe(loads);
  });

  it('respects the sound toggle for clicks and heehee', async () => {
    const { started } = fakeWebAudio();
    await preloadSounds();
    writeSoundEnabled(false);
    playHeehee();
    playUiSound('switch');
    expect(started).toHaveLength(0);
  });

  it('falls back to a plain player for heehee when decoding is unavailable', () => {
    const play = vi.fn(() => Promise.resolve());
    vi.stubGlobal('Audio', function () {
      return { play };
    });
    playHeehee();
    expect(play).toHaveBeenCalledTimes(1);
    playUiSound('click');
    expect(play).toHaveBeenCalledTimes(1);
  });
});

describe('which element makes which sound', () => {
  function html(markup: string): Element {
    document.body.innerHTML = markup;
    return document.body.querySelector('[data-t]')!;
  }

  it('uses the switch sound for sections and tabs, click for other buttons', () => {
    expect(
      soundKindFor(html('<nav class="rail"><button data-t><span>x</span></button></nav>')),
    ).toBe('switch');
    expect(soundKindFor(html('<nav class="subtabs"><button data-t>x</button></nav>'))).toBe(
      'switch',
    );
    expect(soundKindFor(html('<button data-t>Применить</button>'))).toBe('click');
    expect(soundKindFor(html('<input data-t type="checkbox" />'))).toBe('click');
  });

  it('is silent for plain content, disabled buttons, the poster and the sound toggle', () => {
    expect(soundKindFor(html('<p data-t>text</p>'))).toBeNull();
    expect(soundKindFor(html('<button data-t disabled>x</button>'))).toBeNull();
    expect(soundKindFor(html('<button data-t class="poster">x</button>'))).toBeNull();
    expect(soundKindFor(html('<button data-t class="pill sound">x</button>'))).toBeNull();
    expect(soundKindFor(html('<button data-t data-no-ui-sound>x</button>'))).toBeNull();
    expect(soundKindFor(null)).toBeNull();
  });

  it('plays on pointer press and on keyboard activation, once per action', async () => {
    const { started } = fakeWebAudio();
    await preloadSounds();
    const detach = attachUiSounds(window);
    const button = html('<button data-t>x</button>') as HTMLButtonElement;
    button.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button: 0 }));
    button.dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 1 }));
    expect(started).toHaveLength(1);
    button.dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 0 }));
    expect(started).toHaveLength(2);
    detach();
    button.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button: 0 }));
    expect(started).toHaveLength(2);
  });
});
