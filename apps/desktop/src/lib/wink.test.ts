import { afterEach, describe, expect, it, vi } from 'vitest';
import { portraitGeometry, EYE, startWinking, WINK_MAX_MS, WINK_MIN_MS } from './wink';

afterEach(() => vi.useRealTimers());

describe('startWinking', () => {
  it('winks at a random 20-40 s interval and keeps going', () => {
    vi.useFakeTimers();
    const wink = vi.fn();
    const stop = startWinking({ canWink: () => true, wink, random: () => 0 });
    vi.advanceTimersByTime(WINK_MIN_MS - 1);
    expect(wink).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(wink).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(WINK_MIN_MS);
    expect(wink).toHaveBeenCalledTimes(2);
    stop();
  });

  it('uses the upper bound for random 1', () => {
    vi.useFakeTimers();
    const wink = vi.fn();
    const stop = startWinking({ canWink: () => true, wink, random: () => 1 });
    vi.advanceTimersByTime(WINK_MAX_MS - 1);
    expect(wink).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(wink).toHaveBeenCalledTimes(1);
    stop();
  });

  it('skips while not allowed (hidden, unfocused, pressed) and tries again later', () => {
    vi.useFakeTimers();
    let ok = false;
    const wink = vi.fn();
    const stop = startWinking({ canWink: () => ok, wink, random: () => 0 });
    vi.advanceTimersByTime(WINK_MIN_MS);
    expect(wink).not.toHaveBeenCalled();
    ok = true;
    vi.advanceTimersByTime(WINK_MIN_MS);
    expect(wink).toHaveBeenCalledTimes(1);
    stop();
  });

  it('leaves no timer behind after stop', () => {
    vi.useFakeTimers();
    const wink = vi.fn();
    const stop = startWinking({ canWink: () => true, wink });
    stop();
    expect(vi.getTimerCount()).toBe(0);
    vi.advanceTimersByTime(WINK_MAX_MS * 3);
    expect(wink).not.toHaveBeenCalled();
  });
});

describe('portraitGeometry', () => {
  it('matches the image exactly for the image aspect ratio', () => {
    const g = portraitGeometry(EYE.imgW / 2, EYE.imgH / 2);
    expect(g.scale).toBeCloseTo(0.5);
    expect(g.offsetX).toBeCloseTo(0);
    expect(g.offsetY).toBeCloseTo(0);
    expect(EYE.x + EYE.w).toBeLessThan(EYE.imgW);
  });

  it('fits the whole image in a tall panel and centres the face', () => {
    const g = portraitGeometry(204, 732);
    expect(g.scale * EYE.imgW).toBeCloseTo(204);
    expect(g.offsetX).toBeCloseTo(0);
    expect(g.offsetY).toBeCloseTo((732 - EYE.imgH * g.scale) / 2);
    expect(g.offsetY + EYE.imgH * g.scale).toBeLessThan(732);
  });
});
