import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import BrandMark from './BrandMark.svelte';

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('BrandMark', () => {
  it('keeps the word readable for assistive tech and draws eight live letters', () => {
    const { container } = render(BrandMark);
    expect(screen.getByLabelText('OCHISTKA')).toBeTruthy();
    expect(container.querySelectorAll('.variable-proximity [aria-hidden="true"]')).toHaveLength(8);
  });

  it('reserves the width of the heaviest state and lets clicks reach the drag region', () => {
    const { container } = render(BrandMark);
    const sizer = container.querySelector<HTMLElement>('.sizer')!;
    expect(sizer.style.fontVariationSettings).toMatch(/wght["'] 1000/);
    const word = screen.getByLabelText('OCHISTKA');
    expect(word.hasAttribute('data-tauri-drag-region')).toBe(true);
  });

  it('shows a plain static word when the system asks for reduced motion', () => {
    vi.stubGlobal('matchMedia', () => ({ matches: true }) as MediaQueryList);
    const { container } = render(BrandMark);
    expect(container.querySelector('.variable-proximity')).toBeNull();
    expect(container.querySelector('.live')?.textContent?.trim()).toBe('OCHISTKA');
  });
});
