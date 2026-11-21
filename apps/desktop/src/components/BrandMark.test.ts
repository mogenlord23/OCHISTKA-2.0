import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import BrandMark from './BrandMark.svelte';

afterEach(cleanup);

describe('BrandMark', () => {
  it('is a plain static word with the dense rest weight and no effect markup', () => {
    const { container } = render(BrandMark);
    const word = screen.getByLabelText('OCHISTKA');
    expect(word.textContent?.trim()).toBe('OCHISTKA');
    expect(word.style.fontVariationSettings).toMatch(/wght["'] 900/);
    expect(word.style.fontVariationSettings).toMatch(/opsz["'] 28/);
    expect(container.querySelector('.variable-proximity')).toBeNull();
    expect(container.querySelector('.sizer, .live')).toBeNull();
  });

  it('stays a drag region so the window can be moved by the word', () => {
    render(BrandMark);
    expect(screen.getByLabelText('OCHISTKA').hasAttribute('data-tauri-drag-region')).toBe(true);
  });
});
