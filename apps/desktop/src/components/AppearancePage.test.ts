import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import AppearancePage from './AppearancePage.svelte';
import { APPEARANCE_STORAGE_KEY } from '../lib/appearanceStore';

beforeEach(() => localStorage.clear());
afterEach(() => cleanup());

describe('AppearancePage', () => {
  it('marks a tile as selected and stores the choice', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const sunset = screen.getByRole('button', { name: /Фото 003/ });
    expect(sunset.getAttribute('aria-pressed')).toBe('false');

    await fireEvent.click(sunset);

    expect(sunset.getAttribute('aria-pressed')).toBe('true');
    expect(sunset.textContent).toContain('Выбрано');
    expect(screen.getByText('Фото 003', { selector: 'strong' })).toBeTruthy();
    const stored = JSON.parse(localStorage.getItem(APPEARANCE_STORAGE_KEY) ?? '{}');
    expect(stored.wallpaper).toBe('photo-c805a20e08574240');
  });

  it('shows the stored choice when mounted again', () => {
    localStorage.setItem(APPEARANCE_STORAGE_KEY, JSON.stringify({ cursor: 'cursor-red' }));
    render(AppearancePage, { subsection: 'cursors' });
    expect(screen.getByRole('button', { name: /Красный/ }).getAttribute('aria-pressed')).toBe(
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

  it('lazy-loads wallpaper images and pages the grid', () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const imgs = document.querySelectorAll('img');
    expect(imgs.length).toBeGreaterThan(0);
    expect(imgs.length).toBeLessThan(146);
    for (const img of imgs) expect(img.getAttribute('loading')).toBe('lazy');
  });

  it('draws 35 cursor tiles and keeps the choice', async () => {
    render(AppearancePage, { subsection: 'cursors' });
    expect(document.querySelectorAll('.grid-cursor .tile')).toHaveLength(35);
    expect(screen.getByRole('button', { name: /Белый/ }).getAttribute('aria-pressed')).toBe('true');
    await fireEvent.click(screen.getByRole('button', { name: /Бирюзовый/ }));
    expect(JSON.parse(localStorage.getItem(APPEARANCE_STORAGE_KEY) ?? '{}').cursor).toBe(
      'cursor-cyan',
    );
  });

  it('settings: reduced motion switch persists and sets the data attribute', async () => {
    render(AppearancePage, { subsection: 'settings' });
    const sw = screen.getByRole('switch', { name: 'Меньше анимации' });
    await fireEvent.click(sw);
    expect(sw.getAttribute('aria-checked')).toBe('true');
    expect(document.documentElement.dataset.reduceMotion).toBe('true');
    expect(JSON.parse(localStorage.getItem('ochistka.settings') ?? '{}').reduceMotion).toBe(true);
    await fireEvent.click(sw);
    expect(document.documentElement.dataset.reduceMotion).toBeUndefined();
  });
});
