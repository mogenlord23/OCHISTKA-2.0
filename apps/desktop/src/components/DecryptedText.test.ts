import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import DecryptedText from './DecryptedText.svelte';

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

const shownText = (container: HTMLElement) =>
  [...container.querySelectorAll('[aria-hidden="true"] > span')]
    .map((word) => word.textContent?.replace(/\s+/g, '') ?? '')
    .join('');

describe('DecryptedText (port of React Bits)', () => {
  it('shows the text at rest and exposes it once to assistive tech', () => {
    const { container } = render(DecryptedText, {
      text: 'https://h.test/••••',
      animateOn: 'manual',
    });
    expect(container.querySelector('.sr-only')?.textContent).toBe('https://h.test/••••');
    expect(shownText(container)).toBe('https://h.test/••••');
  });

  it('scrambles with pool characters only and restores the exact text', async () => {
    vi.useFakeTimers();
    const text = 'https://h.test/••••';
    const pool = 'XYZ';
    const ref: { play?: () => void } = {};
    const { container, component } = render(DecryptedText, {
      text,
      animateOn: 'manual',
      characters: pool,
      speed: 20,
      maxIterations: 5,
    });
    Object.assign(ref, component);
    (component as unknown as { play: () => void }).play();
    await vi.advanceTimersByTimeAsync(0);
    const original = new Set(text);
    // Первый кадр: всё, кроме настоящего, из пула.
    const frames: string[] = [];
    for (let i = 0; i < 8; i++) {
      frames.push(
        [...container.querySelectorAll('[aria-hidden="true"] span span')]
          .map((n) => n.textContent ?? '')
          .join(''),
      );
      await vi.advanceTimersByTimeAsync(20);
    }
    for (const frame of frames) {
      for (const ch of frame) expect(original.has(ch) || pool.includes(ch)).toBe(true);
    }
    expect(shownText(container)).toBe(text);
  });

  it('does not animate when the system asks for reduced motion', async () => {
    vi.stubGlobal('matchMedia', () => ({ matches: true }) as MediaQueryList);
    const { container, component } = render(DecryptedText, { text: 'abc', animateOn: 'manual' });
    (component as unknown as { play: () => void }).play();
    await Promise.resolve();
    expect(shownText(container)).toBe('abc');
    expect(screen.getByText('abc', { selector: '.sr-only' })).toBeTruthy();
  });
});
