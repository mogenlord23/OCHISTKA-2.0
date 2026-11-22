import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createLabelTypewriter, ERASE_MS, HOLD_MS, LABEL, SAVED, TYPE_MS } from './labelTypewriter';

describe('createLabelTypewriter', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  function setup() {
    const seen: string[] = [];
    const tw = createLabelTypewriter((t) => seen.push(t));
    return { seen, tw };
  }

  it('erases the label, types «Подписка сохранена», holds 3 s, then returns the label', () => {
    const { seen, tw } = setup();
    tw.play();
    vi.advanceTimersByTime(ERASE_MS * (LABEL.length + 1));
    expect(seen[0]).toBe(LABEL.slice(0, -1));
    expect(seen).toContain('');
    vi.advanceTimersByTime(TYPE_MS * (SAVED.length + 1));
    expect(seen.at(-1)).toBe(SAVED);
    // Пока держится 3 секунды, текст не меняется.
    const count = seen.length;
    vi.advanceTimersByTime(HOLD_MS - 300);
    expect(seen.length).toBe(count);
    vi.advanceTimersByTime(300 + ERASE_MS * (SAVED.length + 2) + TYPE_MS * (LABEL.length + 2) + 20);
    expect(seen.at(-1)).toBe(LABEL);
    // Текст всегда префикс одной из двух фраз: стирание и печать по одному знаку.
    for (const t of seen) expect(LABEL.startsWith(t) || SAVED.startsWith(t)).toBe(true);
  });

  it('a new play restarts the order from the label and leaves no stale timers', () => {
    const { seen, tw } = setup();
    tw.play();
    vi.advanceTimersByTime(ERASE_MS * 5);
    tw.play();
    vi.advanceTimersByTime(60_000);
    expect(seen.at(-1)).toBe(LABEL);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('showSaved shows the saved text at once and the label after 3 s, with no typing', () => {
    const { seen, tw } = setup();
    tw.showSaved();
    expect(seen).toEqual([SAVED]);
    vi.advanceTimersByTime(HOLD_MS - 1);
    expect(seen).toEqual([SAVED]);
    vi.advanceTimersByTime(2);
    expect(seen).toEqual([SAVED, LABEL]);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('reset stops everything and shows the plain label', () => {
    const { seen, tw } = setup();
    tw.play();
    vi.advanceTimersByTime(300);
    tw.reset();
    expect(seen.at(-1)).toBe(LABEL);
    expect(vi.getTimerCount()).toBe(0);
  });
});
