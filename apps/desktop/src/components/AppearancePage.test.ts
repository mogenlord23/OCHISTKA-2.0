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
  originalsMissing: true,
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

const count = (action: string) => actions().filter((a) => a === action).length;

describe('AppearancePage wallpapers', () => {
  it('first click sets the Windows wallpaper and shows an outline only', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const tile = screen.getByRole('button', { name: 'Обои 12' });
    expect(tile.getAttribute('aria-pressed')).toBe('false');
    await fireEvent.click(tile);
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('true'));
    expect(count('applyWallpaperTile')).toBe(1);
    expect(tile.classList.contains('selected')).toBe(true);
    expect(tile.textContent?.trim()).toBe('');
    expect(tile.querySelector('svg')).toBeNull();
    expect(document.querySelectorAll('.grid [title]')).toHaveLength(0);
    expect(document.body.textContent).not.toMatch(
      /#[0-9A-Fa-f]{6}|Сплошной цвет|подключено|применено|Готово|✓|✔/i,
    );
  });

  it('a second click on the selected wallpaper does nothing', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const tile = screen.getByRole('button', { name: 'Обои 12' });
    await fireEvent.click(tile);
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('true'));
    const before = invokeMock.mock.calls.length;
    await fireEvent.click(tile);
    await fireEvent.click(tile);
    expect(invokeMock.mock.calls.length).toBe(before);
    expect(count('restoreWallpaper')).toBe(0);
    expect(tile.getAttribute('aria-pressed')).toBe('true');
  });

  it('only «Вернуть как было» restores, and it clears the outline', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const restore = screen.getByRole('button', { name: 'Вернуть как было' });
    expect(restore.getAttribute('aria-disabled')).toBe('true');
    await fireEvent.click(restore);
    expect(count('restoreWallpaper')).toBe(0);
    const tile = screen.getByRole('button', { name: 'Обои 12' });
    await fireEvent.click(tile);
    await waitFor(() => expect(restore.getAttribute('aria-disabled')).toBe('false'));
    await fireEvent.click(restore);
    await waitFor(() => expect(count('restoreWallpaper')).toBe(1));
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('false'));
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('choosing another tile applies it without restoring in between', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    await fireEvent.click(screen.getByRole('button', { name: 'Обои 12' }));
    await waitFor(() => expect(count('applyWallpaperTile')).toBe(1));
    await fireEvent.click(screen.getByRole('button', { name: 'Обои 13' }));
    await waitFor(() => expect(count('applyWallpaperTile')).toBe(2));
    expect(count('restoreWallpaper')).toBe(0);
  });

  it('solid tiles send their colour, photo tiles send no colour', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    await fireEvent.click(screen.getByRole('button', { name: 'Обои 1' }));
    await waitFor(() => expect(actions()).toContain('applyWallpaperTile'));
    const solid = requests().find((r) => r.action === 'applyWallpaperTile');
    expect(solid?.solidColor).toMatch(/^#[0-9A-Fa-f]{6}$/);
    expect(solid?.confirmed).toBe(true);
  });

  it('draws square photo tiles of one size, centred and covering', () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const imgs = document.querySelectorAll<HTMLImageElement>('.grid-wallpaper img');
    expect(imgs.length).toBeGreaterThan(0);
    expect(imgs.length).toBeLessThan(146);
    for (const img of imgs) {
      expect(img.getAttribute('loading')).toBe('lazy');
      expect(img.getAttribute('width')).toBe(img.getAttribute('height'));
    }
    expect(document.querySelector('.grid-wallpaper')?.classList.contains('squares')).toBe(true);
  });

  it('opens the file picker straight from the button, without a dialog', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    await fireEvent.click(screen.getByRole('button', { name: 'Добавить свои обои…' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    await waitFor(() =>
      expect(requests()).toContainEqual({ action: 'pickAndApplyWallpaper', confirmed: true }),
    );
  });

  it('offers «Скачать оригиналы» only while originals are missing', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const button = await screen.findByRole('button', { name: 'Скачать оригиналы' });
    await fireEvent.click(button);
    await waitFor(() => expect(actions()).toContain('downloadOriginals'));
    cleanup();
    windows = { ...STATUS, originalsMissing: false };
    render(AppearancePage, { subsection: 'wallpapers' });
    await waitFor(() => expect(actions().filter((a) => a === 'status')).toHaveLength(2));
    expect(screen.queryByRole('button', { name: 'Скачать оригиналы' })).toBeNull();
  });

  it('has no own page title: the shell draws it', () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    expect(document.querySelector('h1, h2')).toBeNull();
  });
});

describe('AppearancePage cursors', () => {
  it('draws 36 tiles; click sets the Windows scheme once', async () => {
    render(AppearancePage, { subsection: 'cursors' });
    expect(document.querySelectorAll('.grid-cursor .tile')).toHaveLength(36);
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
    const before = invokeMock.mock.calls.length;
    await fireEvent.click(tile);
    expect(invokeMock.mock.calls.length).toBe(before);
    expect(count('restoreCursorScheme')).toBe(0);
  });

  it('restores only through «Вернуть как было»', async () => {
    render(AppearancePage, { subsection: 'cursors' });
    const restore = screen.getByRole('button', { name: 'Вернуть как было' });
    await fireEvent.click(restore);
    expect(count('restoreCursorScheme')).toBe(0);
    const tile = screen.getByRole('button', { name: 'Курсор 6' });
    await fireEvent.click(tile);
    await waitFor(() => expect(restore.getAttribute('aria-disabled')).toBe('false'));
    await fireEvent.click(restore);
    await waitFor(() => expect(count('restoreCursorScheme')).toBe(1));
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('false'));
  });

  it('previews at the real 32 px, and the animated scheme animates', () => {
    render(AppearancePage, { subsection: 'cursors' });
    const svgs = document.querySelectorAll('.grid-cursor svg');
    expect(svgs).toHaveLength(36);
    for (const svg of svgs) {
      expect(svg.getAttribute('width')).toBe('32');
      expect(svg.getAttribute('height')).toBe('32');
    }
    expect(document.querySelectorAll('.grid-cursor svg animate')).toHaveLength(1);
  });

  it('sends the animated scheme as pattern pulse', async () => {
    render(AppearancePage, { subsection: 'cursors' });
    await fireEvent.click(screen.getByRole('button', { name: 'Курсор 36' }));
    await waitFor(() =>
      expect(requests()).toContainEqual(
        expect.objectContaining({ action: 'applyCursorScheme', pattern: 'pulse' }),
      ),
    );
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
      'Вернуть как было',
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

  it('a colour click sets the Windows accent; a second click does nothing', async () => {
    render(AppearancePage, { subsection: 'colors' });
    const tile = screen.getByRole('button', { name: 'Цвет 3' });
    await fireEvent.click(tile);
    await waitFor(() =>
      expect(requests()).toContainEqual({ action: 'setAccent', color: '#087FFF', confirmed: true }),
    );
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('true'));
    expect(screen.queryByRole('dialog')).toBeNull();
    const before = invokeMock.mock.calls.length;
    await fireEvent.click(tile);
    expect(invokeMock.mock.calls.length).toBe(before);
    expect(count('restoreAccent')).toBe(0);
    expect(tile.getAttribute('aria-pressed')).toBe('true');
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

  it('«Вернуть как было» is available only with a stored value and restores it', async () => {
    windows = { ...STATUS, hasBackup: true };
    render(AppearancePage, { subsection: 'colors' });
    const restore = screen.getByRole('button', { name: 'Вернуть как было' });
    await waitFor(() => expect(restore.getAttribute('aria-disabled')).toBe('false'));
    await fireEvent.click(restore);
    await fireEvent.click(screen.getByRole('button', { name: 'Вернуть' }));
    await waitFor(() =>
      expect(requests()).toContainEqual({ action: 'restoreAppearance', confirmed: true }),
    );
  });

  it('«Вернуть как было» does nothing while nothing was changed', async () => {
    render(AppearancePage, { subsection: 'colors' });
    await waitFor(() => expect(actions()).toContain('status'));
    await fireEvent.click(screen.getByRole('button', { name: 'Вернуть как было' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(count('restoreAppearance')).toBe(0);
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

describe('AppearancePage header art', () => {
  it('shows the brush illustration, decorative only', () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const art = document.querySelector('.brushes');
    expect(art).toBeTruthy();
    expect(art?.getAttribute('aria-hidden')).toBe('true');
    expect(document.querySelector('.brushes circle')).toBeNull();
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
