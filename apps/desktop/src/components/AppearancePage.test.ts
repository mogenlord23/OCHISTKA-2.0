import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import AppearancePage from './AppearancePage.svelte';
import { APPEARANCE_STORAGE_KEY } from '../lib/appearanceStore';

const STATUS = {
  accent: '#FF009D',
  transparency: true,
  darkApps: true,
  darkSystem: false,
  fillTaskbar: false,
  hasBackup: false,
  hasPreviousWallpaper: false,
};

beforeEach(() => {
  localStorage.clear();
  invokeMock.mockReset();
  invokeMock.mockResolvedValue({ type: 'status', status: STATUS });
});
afterEach(() => {
  cleanup();
  document.getElementById('ochistka-app-cursor')?.remove();
});

const requests = () => invokeMock.mock.calls.map((call) => call[1].request);

describe('AppearancePage wallpapers and cursors', () => {
  it('selects a tile with an outline state only: no hex, tooltip or caption', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const tile = screen.getByRole('button', { name: 'Обои 12' });
    expect(tile.getAttribute('aria-pressed')).toBe('false');
    await fireEvent.click(tile);
    expect(tile.getAttribute('aria-pressed')).toBe('true');
    expect(tile.classList.contains('selected')).toBe(true);
    expect(tile.textContent?.trim()).toBe('');
    expect(document.querySelectorAll('.grid [title]')).toHaveLength(0);
    expect(document.body.textContent).not.toMatch(
      /#[0-9A-Fa-f]{6}|Сплошной цвет|пока не подключено/,
    );
    const stored = JSON.parse(localStorage.getItem(APPEARANCE_STORAGE_KEY) ?? '{}');
    expect(stored.wallpaper).toBeTruthy();
    expect(requests().every((r) => r.action === 'status')).toBe(true);
  });

  it('lazy-loads wallpaper images and pages the grid', () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const imgs = document.querySelectorAll('.grid img');
    expect(imgs.length).toBeGreaterThan(0);
    expect(imgs.length).toBeLessThan(146);
    for (const img of imgs) expect(img.getAttribute('loading')).toBe('lazy');
  });

  it('asks before opening the wallpaper picker and sends confirmed only after Yes', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    await fireEvent.click(screen.getByRole('button', { name: 'Добавить свои обои…' }));
    expect(screen.getByRole('dialog')).toBeTruthy();
    expect(requests().some((r) => r.action === 'pickAndApplyWallpaper')).toBe(false);
    await fireEvent.click(screen.getByRole('button', { name: 'Отмена' }));
    expect(requests().some((r) => r.action === 'pickAndApplyWallpaper')).toBe(false);
    await fireEvent.click(screen.getByRole('button', { name: 'Добавить свои обои…' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Выбрать файл' }));
    await waitFor(() =>
      expect(requests()).toContainEqual({ action: 'pickAndApplyWallpaper', confirmed: true }),
    );
  });

  it('keeps «Вернуть прежний фон» inert until a previous wallpaper exists', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const back = screen.getByRole('button', { name: 'Вернуть прежний фон' });
    await fireEvent.click(back);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('draws 35 cursor tiles; choosing one becomes the app pointer', async () => {
    render(AppearancePage, { subsection: 'cursors' });
    expect(document.querySelectorAll('.grid-cursor .tile')).toHaveLength(35);
    expect(document.getElementById('ochistka-app-cursor')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Курсор 6' }));
    const style = document.getElementById('ochistka-app-cursor');
    expect(style?.textContent).toContain('cursor: url("data:image/svg+xml,');
    expect(style?.textContent).toContain(', auto');
    expect(JSON.parse(localStorage.getItem(APPEARANCE_STORAGE_KEY) ?? '{}').cursor).toBe(
      'cursor-lime',
    );
    expect(requests().every((r) => r.action === 'status')).toBe(true);
  });
});

describe('AppearancePage colours', () => {
  it('shows the option chips and 53 swatches', async () => {
    render(AppearancePage, { subsection: 'colors' });
    for (const name of [
      'Эффект прозрачности',
      'Залить Пуск и всю панель задач',
      'Режим для Пуска',
      'Тёмный режим',
      'Светлый режим',
      'Вернуть оформление Windows',
    ]) {
      expect(screen.getByRole('button', { name })).toBeTruthy();
    }
    expect(document.querySelectorAll('.grid-color .tile')).toHaveLength(53);
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Тёмный режим' }).getAttribute('aria-pressed'),
      ).toBe('true'),
    );
  });

  it('changes the accent only after confirmation', async () => {
    render(AppearancePage, { subsection: 'colors' });
    await fireEvent.click(screen.getByRole('button', { name: 'Цвет 3' }));
    expect(screen.getByRole('dialog')).toBeTruthy();
    expect(requests().some((r) => r.action === 'setAccent')).toBe(false);
    await fireEvent.click(screen.getByRole('button', { name: 'Сменить' }));
    await waitFor(() =>
      expect(requests()).toContainEqual({ action: 'setAccent', color: '#087FFF', confirmed: true }),
    );
  });

  it('light mode asks, then sets theme for apps and system', async () => {
    render(AppearancePage, { subsection: 'colors' });
    await fireEvent.click(screen.getByRole('button', { name: 'Светлый режим' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Изменить' }));
    await waitFor(() =>
      expect(requests()).toContainEqual({
        action: 'setTheme',
        dark: false,
        alsoSystem: true,
        confirmed: true,
      }),
    );
  });

  it('restore is available only with a backup', async () => {
    invokeMock.mockResolvedValue({ type: 'status', status: { ...STATUS, hasBackup: true } });
    render(AppearancePage, { subsection: 'colors' });
    const restore = screen.getByRole('button', { name: 'Вернуть оформление Windows' });
    await waitFor(() => expect(restore.getAttribute('aria-disabled')).toBe('false'));
    await fireEvent.click(restore);
    await fireEvent.click(screen.getByRole('button', { name: 'Вернуть' }));
    await waitFor(() =>
      expect(requests()).toContainEqual({ action: 'restoreAppearance', confirmed: true }),
    );
  });

  it('shows the backend error text', async () => {
    render(AppearancePage, { subsection: 'colors' });
    invokeMock.mockRejectedValue({
      kind: 'unsupported',
      message: 'Эта функция доступна только в Windows',
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Эффект прозрачности' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Изменить' }));
    expect(await screen.findByText('Эта функция доступна только в Windows')).toBeTruthy();
  });
});

describe('AppearancePage settings', () => {
  it('has language and reduced motion, no version text', async () => {
    render(AppearancePage, { subsection: 'settings' });
    expect(document.body.textContent).not.toMatch(/Версия|OCHISTKA 0/);
    const sw = screen.getByRole('switch', { name: 'Меньше анимации' });
    await fireEvent.click(sw);
    expect(sw.getAttribute('aria-checked')).toBe('true');
    expect(document.documentElement.dataset.reduceMotion).toBe('true');
    expect(JSON.parse(localStorage.getItem('ochistka.settings') ?? '{}').reduceMotion).toBe(true);
    await fireEvent.click(sw);
    expect(document.documentElement.dataset.reduceMotion).toBeUndefined();
  });
});
