import { describe, expect, it } from 'vitest';
import { HISTORY_LIMIT, pushSample } from './history';

describe('pushSample', () => {
  it('keeps 90 samples as the history limit', () => {
    expect(HISTORY_LIMIT).toBe(90);
  });

  it('appends samples in order', () => {
    expect(pushSample([], 10)).toEqual([10]);
    expect(pushSample([10, 20], 30)).toEqual([10, 20, 30]);
  });

  it('caps the history at 90 samples and drops the oldest first', () => {
    let history: number[] = [];
    for (let i = 0; i < 100; i += 1) {
      history = pushSample(history, i);
    }
    expect(history).toHaveLength(90);
    expect(history[0]).toBe(10);
    expect(history[89]).toBe(99);
  });

  it('does not grow past the cap when it is already full', () => {
    const full = Array.from({ length: 90 }, (_, i) => i);
    const next = pushSample(full, 90);
    expect(next).toHaveLength(90);
    expect(next[0]).toBe(1);
    expect(next[89]).toBe(90);
  });

  it('does not modify the array it was given', () => {
    const before = [1, 2, 3];
    pushSample(before, 4);
    expect(before).toEqual([1, 2, 3]);
  });

  it('accepts a custom limit', () => {
    expect(pushSample([1, 2, 3], 4, 3)).toEqual([2, 3, 4]);
  });
});
