import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import AppearancePage from './AppearancePage.svelte';
import { APPEARANCE_STORAGE_KEY } from '../lib/appearanceStore';

beforeEach(() => localStorage.clear());
afterEach(() => cleanup());

describe('AppearancePage', () => {
  it('marks a tile as selected and stores the choice', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const sunset = screen.getByRole('button', { name: /Закат/ });
    expect(sunset.getAttribute('aria-pressed')).toBe('false');

    await fireEvent.click(sunset);

    expect(sunset.getAttribute('aria-pressed')).toBe('true');
    expect(sunset.textContent).toContain('Выбрано');
    expect(screen.getByText('Закат', { selector: 'strong' })).toBeTruthy();
    const stored = JSON.parse(localStorage.getItem(APPEARANCE_STORAGE_KEY) ?? '{}');
    expect(stored.wallpaper).toBe('sunset');
  });

  it('shows the stored choice when mounted again', () => {
    localStorage.setItem(APPEARANCE_STORAGE_KEY, JSON.stringify({ cursor: 'inverted' }));
    render(AppearancePage, { subsection: 'cursors' });
    expect(screen.getByRole('button', { name: /Инверсный/ }).getAttribute('aria-pressed')).toBe(
      'true',
    );
  });

  it('renders a disabled apply button with its reason', () => {
    render(AppearancePage, { subsection: 'colors' });
    const apply = screen.getByRole('button', { name: 'Применить к Windows' }) as HTMLButtonElement;
    expect(apply.disabled).toBe(true);
    const reason = document.getElementById('apply-reason');
    expect(reason?.textContent).toBe('Применение к Windows пока не подключено');
    expect(apply.getAttribute('aria-describedby')).toBe('apply-reason');
  });

  it('shows the grid for each subsection', () => {
    const { unmount } = render(AppearancePage, { subsection: 'colors' });
    expect(screen.getByRole('button', { name: /Лаймовый/ })).toBeTruthy();
    unmount();
  });
});
