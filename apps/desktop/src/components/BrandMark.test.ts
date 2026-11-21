import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import BrandMark from './BrandMark.svelte';

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function layout() {
  vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
    cb(0);
    return 1;
  });
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (
    this: HTMLElement,
  ) {
    const i = Array.from(this.parentElement?.children ?? []).indexOf(this);
    return {
      left: i * 20,
      right: i * 20 + 20,
      top: 0,
      bottom: 20,
      width: 20,
      height: 20,
      x: i * 20,
      y: 0,
    } as DOMRect;
  });
}

describe('BrandMark', () => {
  it('keeps the word readable for assistive tech and draws eight letters', () => {
    const { container } = render(BrandMark);
    expect(screen.getByLabelText('OCHISTKA')).toBeTruthy();
    expect(container.querySelectorAll('.ch')).toHaveLength(8);
  });

  it('thickens letters near the pointer and resets on leave', async () => {
    layout();
    const { container } = render(BrandMark);
    const word = screen.getByLabelText('OCHISTKA');
    const letters = [...container.querySelectorAll<HTMLElement>('.ch')];
    await fireEvent(word, new MouseEvent('pointerenter'));
    word.dispatchEvent(new MouseEvent('pointermove', { clientX: 10 }));
    expect(Number(letters[0].style.getPropertyValue('--p'))).toBeGreaterThan(0.9);
    expect(Number(letters[7].style.getPropertyValue('--p'))).toBe(0);
    await fireEvent(word, new MouseEvent('pointerleave'));
    expect(letters.every((l) => l.style.getPropertyValue('--p') === '0')).toBe(true);
  });

  it('does nothing without hover and ignores moves after leaving', async () => {
    layout();
    const { container } = render(BrandMark);
    const word = screen.getByLabelText('OCHISTKA');
    word.dispatchEvent(new MouseEvent('pointermove', { clientX: 10 }));
    expect(container.querySelector<HTMLElement>('.ch')!.style.getPropertyValue('--p')).toBe('');
  });
});
