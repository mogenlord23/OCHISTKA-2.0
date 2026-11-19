import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import AppearancePage from './AppearancePage.svelte';

const STATUS = {
  accent: '#FF009D',
  transparency: true,
  darkApps: true,
  darkSystem: false,
  fillTaskbar: false,
  hasBackup: false,
  hasPreviousWallpaper: false,
  wallpaperTile: null as string | null,
  accentTile: null as string | null,
  cursorScheme: null as string | null,
};

/** Состояние «Windows» в тесте: команды меняют его так же, как ядро. */
let windows = { ...STATUS };

function fakeWindows(request: Record<string, unknown>) {
  switch (request.action) {
    case 'applyWallpaperTile':
      windows = { ...windows, wallpaperTile: request.id as string, hasPreviousWallpaper: true };
      break;
    case 'restoreWallpaper':
      windows = { ...windows, wallpaperTile: null, hasPreviousWallpaper: false };
      break;
    case 'applyCursorScheme':
      windows = { ...windows, cursorScheme: request.id as string };
      break;
    case 'restoreCursorScheme':
      windows = { ...windows, cursorScheme: null };
      break;
    case 'setAccent':
      windows = { ...windows, accentTile: request.color as string };
      break;
    case 'restoreAccent':
      windows = { ...windows, accentTile: null };
      break;
  }
  return { type: 'status', status: windows };
}

beforeEach(() => {
  localStorage.clear();
  windows = { ...STATUS };
  invokeMock.mockReset();
  invokeMock.mockImplementation(async (_cmd: string, args: { request: Record<string, unknown> }) =>
    fakeWindows(args.request),
  );
});
afterEach(() => {
  cleanup();
});

const requests = () => invokeMock.mock.calls.map((call) => call[1].request);
const actions = () => requests().map((r) => r.action);

describe('AppearancePage wallpapers', () => {
  it('click sets the Windows wallpaper, outline only, second click restores', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const tile = screen.getByRole('button', { name: 'Обои 12' });
    expect(tile.getAttribute('aria-pressed')).toBe('false');
    await fireEvent.click(tile);
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('true'));
    expect(actions()).toContain('applyWallpaperTile');
    expect(tile.classList.contains('selected')).toBe(true);
    expect(tile.textContent?.trim()).toBe('');
    expect(document.querySelectorAll('.grid [title]')).toHaveLength(0);
    expect(document.body.textContent).not.toMatch(
      /#[0-9A-Fa-f]{6}|Сплошной цвет|подключено|применено|Готово/i,
    );
    await fireEvent.click(tile);
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('false'));
    expect(actions()).toContain('restoreWallpaper');
    expect(tile.classList.contains('selected')).toBe(false);
  });

  it('a photo tile sends only its id: the backend copies the bundled original', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    await fireEvent.click(screen.getByRole('button', { name: 'Обои 1' }));
    await waitFor(() => expect(actions()).toContain('applyWallpaperTile'));
    const request = requests().find((r) => r.action === 'applyWallpaperTile');
    expect(request).toEqual({
      action: 'applyWallpaperTile',
      id: 'photo-6e9353eef7d201d5',
      confirmed: true,
    });
  });

  it('keeps one selected tile per group and restores the selection after a remount', async () => {
    const first = render(AppearancePage, { subsection: 'wallpapers' });
    const a = screen.getByRole('button', { name: 'Обои 2' });
    const b = screen.getByRole('button', { name: 'Обои 3' });
    await fireEvent.click(a);
    await waitFor(() => expect(a.getAttribute('aria-pressed')).toBe('true'));
    await fireEvent.click(b);
    await waitFor(() => expect(b.getAttribute('aria-pressed')).toBe('true'));
    expect(a.getAttribute('aria-pressed')).toBe('false');
    expect(document.querySelectorAll('.grid .tile.selected')).toHaveLength(1);
    // «Перезапуск»: страница создаётся заново и читает выбор из Windows.
    first.unmount();
    render(AppearancePage, { subsection: 'wallpapers' });
    const again = screen.getByRole('button', { name: 'Обои 3' });
    await waitFor(() => expect(again.getAttribute('aria-pressed')).toBe('true'));
    expect(document.querySelectorAll('.grid .tile.selected')).toHaveLength(1);
  });

  it('lazy-loads wallpaper images and pages the grid', () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const imgs = document.querySelectorAll('.grid img');
    expect(imgs.length).toBeGreaterThan(0);
    expect(imgs.length).toBeLessThan(146);
    for (const img of imgs) expect(img.getAttribute('loading')).toBe('lazy');
  });

  it('opens the file picker straight from the button, without a dialog', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    await fireEvent.click(screen.getByRole('button', { name: 'Добавить свои обои…' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    await waitFor(() =>
      expect(requests()).toContainEqual({ action: 'pickAndApplyWallpaper', confirmed: true }),
    );
  });

  it('keeps «Вернуть прежний фон» inert until a previous wallpaper exists', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    await waitFor(() => expect(actions()).toContain('status'));
    await fireEvent.click(screen.getByRole('button', { name: 'Вернуть прежний фон' }));
    expect(actions()).not.toContain('restoreWallpaper');
  });
});

describe('AppearancePage cursors', () => {
  it('draws 53 tiles; click sets the Windows scheme, second click restores', async () => {
    render(AppearancePage, { subsection: 'cursors' });
    expect(document.querySelectorAll('.grid-cursor .tile')).toHaveLength(53);
    const tile = screen.getByRole('button', { name: 'Курсор 6' });
    await fireEvent.click(tile);
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('true'));
    expect(requests()).toContainEqual({
      action: 'applyCursorScheme',
      id: 'cursor-lime',
      color: '#8AFF00',
      secondary: '#8AFF00',
      pattern: 'plain',
      confirmed: true,
    });
    expect(document.getElementById('ochistka-app-cursor')).toBeNull();
    await fireEvent.click(tile);
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('false'));
    expect(actions()).toContain('restoreCursorScheme');
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

  it('a colour click sets the Windows accent, a second click restores it', async () => {
    render(AppearancePage, { subsection: 'colors' });
    const tile = screen.getByRole('button', { name: 'Цвет 3' });
    await fireEvent.click(tile);
    await waitFor(() =>
      expect(requests()).toContainEqual({ action: 'setAccent', color: '#087FFF', confirmed: true }),
    );
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('true'));
    expect(screen.queryByRole('dialog')).toBeNull();
    await fireEvent.click(tile);
    await waitFor(() => expect(actions()).toContain('restoreAccent'));
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('false'));
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
    windows = { ...STATUS, hasBackup: true };
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
    invokeMock.mockReset();
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
