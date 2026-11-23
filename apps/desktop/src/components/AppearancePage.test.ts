import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import AppearancePage from './AppearancePage.svelte';
import appearanceSource from './AppearancePage.svelte?raw';
import { COLOR_SCHEMES, SOLID_WALLPAPERS } from '../lib/appearance';
import { i18n } from '../lib/i18n.svelte';
import { readSoundEnabled } from '../lib/sound';
import { soundState, syncSoundState } from '../lib/soundState.svelte';

const STATUS = {
  accent: '#FF009D',
  transparencyOn: false,
  fillOn: false,
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
    case 'setTransparency':
      windows = {
        ...windows,
        transparencyOn: request.enabled as boolean,
        fillOn: request.enabled ? false : windows.fillOn,
        hasBackup: true,
      };
      break;
    case 'setFillTaskbar':
      windows = {
        ...windows,
        fillOn: request.enabled as boolean,
        transparencyOn: request.enabled ? false : windows.transparencyOn,
        hasBackup: true,
      };
      break;
    case 'restoreAppearance':
      windows = { ...windows, transparencyOn: false, fillOn: false, accentTile: null };
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

/** Команда, которую тест завершает сам: так видно, что рамка не ждёт ответа. */
function hang() {
  let settle: (value: unknown) => void = () => {};
  let fail: (reason: unknown) => void = () => {};
  invokeMock.mockReset();
  invokeMock.mockImplementation(
    () =>
      new Promise((resolve, reject) => {
        settle = resolve;
        fail = reject;
      }),
  );
  return { settle: (v: unknown) => settle(v), fail: (e: unknown) => fail(e) };
}

const requests = () => invokeMock.mock.calls.map((call) => call[1].request);
const actions = () => requests().map((r) => r.action);

describe('AppearancePage wallpapers', () => {
  it('click sets the Windows wallpaper, outline only, second click restores', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const tile = screen.getByRole('button', { name: 'Обои 30' });
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
    await fireEvent.click(
      screen.getByRole('button', { name: `Обои ${SOLID_WALLPAPERS.length + 1}` }),
    );
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
    const a = screen.getByRole('button', { name: 'Обои 18' });
    const b = screen.getByRole('button', { name: 'Обои 19' });
    await fireEvent.click(a);
    await waitFor(() => expect(a.getAttribute('aria-pressed')).toBe('true'));
    await fireEvent.click(b);
    await waitFor(() => expect(b.getAttribute('aria-pressed')).toBe('true'));
    expect(a.getAttribute('aria-pressed')).toBe('false');
    expect(document.querySelectorAll('.grid .tile.selected')).toHaveLength(1);
    // «Перезапуск»: страница создаётся заново и читает выбор из Windows.
    first.unmount();
    render(AppearancePage, { subsection: 'wallpapers' });
    const again = screen.getByRole('button', { name: 'Обои 19' });
    await waitFor(() => expect(again.getAttribute('aria-pressed')).toBe('true'));
    expect(document.querySelectorAll('.grid .tile.selected')).toHaveLength(1);
  });

  it('shows exactly 7 solid tiles first, each with the app pattern overlay', () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const solids = [...document.querySelectorAll('.grid-wallpaper .tile')].slice(0, 8);
    expect(solids.slice(0, 7).every((t) => t.querySelector('.pattern'))).toBe(true);
    expect(solids[7].querySelector('.pattern')).toBeNull();
    expect(document.querySelectorAll('.grid-wallpaper .pattern')).toHaveLength(7);
  });

  it('lazy-loads wallpaper images and pages the grid', () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const imgs = document.querySelectorAll('.grid img');
    expect(imgs.length).toBeGreaterThan(0);
    expect(imgs.length).toBeLessThan(146);
    for (const img of imgs) expect(img.getAttribute('loading')).toBe('lazy');
  });

  it('keeps the two wallpaper actions together in a flexible wrapping row', () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const add = screen.getByRole('button', { name: 'Добавить свои обои…' });
    const restore = screen.getByRole('button', { name: 'Вернуть прежний фон' });
    expect(add.parentElement).toBe(restore.parentElement);
    expect(add.parentElement?.classList.contains('actions')).toBe(true);
    expect(appearanceSource).toMatch(/flex-wrap:\s*wrap/);
    expect(appearanceSource).toMatch(/flex:\s*1 1 220px/);
    expect(appearanceSource).toMatch(/min-width:\s*min\(100%, 220px\)/);
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
    // На плитках нет анимации-вертушки: только превью и обводка выбранного.
    expect(document.querySelectorAll('.grid-cursor .spin')).toHaveLength(0);
    await fireEvent.click(tile);
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('false'));
    expect(actions()).toContain('restoreCursorScheme');
  });
});

const checked = (el: HTMLElement) => el.getAttribute('aria-checked') === 'true';

describe('AppearancePage colours', () => {
  it('shows only the two toggles and the restore button, no theme controls', async () => {
    render(AppearancePage, { subsection: 'colors' });
    for (const name of ['Эффект прозрачности', 'Залить панель задач и Пуск']) {
      expect(screen.getByRole('switch', { name })).toBeTruthy();
    }
    // «Вернуть оформление Windows» — обычная кнопка, не флажок.
    expect(screen.getByRole('button', { name: 'Вернуть оформление Windows' })).toBeTruthy();
    expect(screen.queryByRole('switch', { name: 'Вернуть оформление Windows' })).toBeNull();
    for (const gone of ['Тёмный режим', 'Светлый режим', 'Режим для Пуска']) {
      expect(screen.queryByRole('button', { name: gone })).toBeNull();
    }
    expect(document.querySelectorAll('.grid-color .tile')).toHaveLength(COLOR_SCHEMES.length);
    // По умолчанию оба переключателя выключены.
    await waitFor(() => expect(actions()).toContain('status'));
    for (const name of ['Эффект прозрачности', 'Залить панель задач и Пуск']) {
      expect(
        (screen.getByRole('switch', { name }) as HTMLElement).getAttribute('aria-checked'),
      ).toBe('false');
    }
  });

  it('a colour click sets the Windows accent, a second click restores it', async () => {
    render(AppearancePage, { subsection: 'colors' });
    const index = COLOR_SCHEMES.findIndex((tile) => tile.preview === '#087FFF');
    const tile = screen.getByRole('button', { name: `Цвет ${index + 1}` });
    await fireEvent.click(tile);
    await waitFor(() =>
      expect(requests()).toContainEqual({ action: 'setAccent', color: '#087FFF', confirmed: true }),
    );
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('true'));
    expect(screen.queryByRole('dialog')).toBeNull();
    // Выбор цвета не трогает заливку панели задач.
    expect(actions()).not.toContain('setFillTaskbar');
    await fireEvent.click(tile);
    await waitFor(() => expect(actions()).toContain('restoreAccent'));
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('false'));
  });

  it('toggles apply on click without a dialog and revert on the second click', async () => {
    render(AppearancePage, { subsection: 'colors' });
    const fill = screen.getByRole('switch', {
      name: 'Залить панель задач и Пуск',
    }) as HTMLElement;
    await fireEvent.click(fill);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(fill.getAttribute('aria-checked')).toBe('true');
    await waitFor(() =>
      expect(requests()).toContainEqual({
        action: 'setFillTaskbar',
        enabled: true,
        confirmed: true,
      }),
    );
    await fireEvent.click(fill);
    expect(fill.getAttribute('aria-checked')).toBe('false');
    await waitFor(() =>
      expect(requests()).toContainEqual({
        action: 'setFillTaskbar',
        enabled: false,
        confirmed: true,
      }),
    );
    const transparency = screen.getByRole('switch', {
      name: 'Эффект прозрачности',
    }) as HTMLElement;
    await fireEvent.click(transparency);
    expect(transparency.getAttribute('aria-checked')).toBe('true');
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('transparency and taskbar fill switch each other off, and both can be off', async () => {
    render(AppearancePage, { subsection: 'colors' });
    const transparency = screen.getByRole('switch', {
      name: 'Эффект прозрачности',
    }) as HTMLElement;
    const fill = screen.getByRole('switch', {
      name: 'Залить панель задач и Пуск',
    }) as HTMLElement;
    await fireEvent.click(transparency);
    expect([checked(transparency), checked(fill)]).toEqual([true, false]);
    await fireEvent.click(fill);
    expect([checked(transparency), checked(fill)]).toEqual([false, true]);
    await fireEvent.click(transparency);
    expect([checked(transparency), checked(fill)]).toEqual([true, false]);
    await fireEvent.click(transparency);
    expect([checked(transparency), checked(fill)]).toEqual([false, false]);
    // Гашение второго флажка отдельной командой не идёт: это делает та же запись в Windows.
    await waitFor(() => expect(actions()).toContain('setTransparency'));
    expect(requests()).not.toContainEqual({
      action: 'setFillTaskbar',
      enabled: false,
      confirmed: true,
    });
  });

  it('the switches sit flush left with the page content', () => {
    render(AppearancePage, { subsection: 'colors' });
    const boxes = document.querySelectorAll('.options .switch');
    expect(boxes).toHaveLength(2);
    expect(document.querySelectorAll('.chip, .box.chip')).toHaveLength(0);
  });

  it('restore is available only with a backup and needs no confirmation', async () => {
    windows = { ...STATUS, hasBackup: true };
    render(AppearancePage, { subsection: 'colors' });
    const restore = screen.getByRole('button', { name: 'Вернуть оформление Windows' });
    await waitFor(() => expect(restore.getAttribute('aria-disabled')).toBe('false'));
    await fireEvent.click(restore);
    expect(screen.queryByRole('dialog')).toBeNull();
    await waitFor(() =>
      expect(requests()).toContainEqual({ action: 'restoreAppearance', confirmed: true }),
    );
  });

  it('a toggle reverts and shows the backend error when the command fails', async () => {
    render(AppearancePage, { subsection: 'colors' });
    await waitFor(() => expect(actions()).toContain('status'));
    const pending = hang();
    const toggle = screen.getByRole('switch', {
      name: 'Эффект прозрачности',
    }) as HTMLElement;
    await fireEvent.click(toggle);
    expect(toggle.getAttribute('aria-checked')).toBe('true');
    pending.fail({ kind: 'unsupported', message: 'Эта функция доступна только в Windows' });
    expect(await screen.findByText('Эта функция доступна только в Windows')).toBeTruthy();
    await waitFor(() => expect(toggle.getAttribute('aria-checked')).toBe('false'));
  });
});

describe('AppearancePage instant selection', () => {
  it('the outline appears before the command resolves and stays when it succeeds', async () => {
    render(AppearancePage, { subsection: 'cursors' });
    const pending = hang();
    const tile = screen.getByRole('button', { name: 'Курсор 4' });
    await fireEvent.click(tile);
    expect(tile.getAttribute('aria-pressed')).toBe('true');
    expect(tile.classList.contains('selected')).toBe(true);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    pending.settle(fakeWindows({ action: 'applyCursorScheme', id: 'cursor-red' }));
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('true'));
  });

  it('the outline is removed and a short error shown when the command fails', async () => {
    render(AppearancePage, { subsection: 'wallpapers' });
    const pending = hang();
    const tile = screen.getByRole('button', { name: 'Обои 2' });
    await fireEvent.click(tile);
    expect(tile.getAttribute('aria-pressed')).toBe('true');
    pending.fail({ kind: 'wallpaper', message: 'Не удалось сменить обои: нет доступа' });
    expect(await screen.findByText('Не удалось сменить обои: нет доступа')).toBeTruthy();
    await waitFor(() => expect(tile.getAttribute('aria-pressed')).toBe('false'));
  });

  it('a second click works at once and commands run in click order', async () => {
    render(AppearancePage, { subsection: 'colors' });
    const pending = hang();
    const a = screen.getByRole('button', { name: 'Цвет 1' });
    const b = screen.getByRole('button', { name: 'Цвет 2' });
    await fireEvent.click(a);
    await fireEvent.click(b);
    expect(a.getAttribute('aria-pressed')).toBe('false');
    expect(b.getAttribute('aria-pressed')).toBe('true');
    expect(invokeMock).toHaveBeenCalledTimes(1);
    pending.settle(fakeWindows({ action: 'setAccent', color: COLOR_SCHEMES[0].preview }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(2));
  });
});

describe('AppearancePage settings', () => {
  beforeEach(() => {
    try {
      localStorage.clear();
    } catch {
      /* нет хранилища */
    }
    syncSoundState();
    i18n.lang = 'ru';
  });

  it('has only sound and language, no reduced-motion switch, no version text', () => {
    render(AppearancePage, { subsection: 'settings' });
    expect(document.body.textContent).not.toMatch(/Версия|OCHISTKA 0/);
    expect(screen.queryByText('Меньше анимации')).toBeNull();
    expect(screen.getByText('Звук')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'RU' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'ENG' })).toBeTruthy();
  });

  it('sound switch is the shared sound state and is remembered', async () => {
    render(AppearancePage, { subsection: 'settings' });
    const before = soundState.on;
    await fireEvent.click(screen.getByRole('switch'));
    expect(soundState.on).toBe(!before);
    expect(readSoundEnabled()).toBe(!before);
    await fireEvent.click(screen.getByRole('switch'));
    expect(soundState.on).toBe(before);
  });

  it('language buttons switch the language and remember it', async () => {
    render(AppearancePage, { subsection: 'settings' });
    await fireEvent.click(screen.getByRole('button', { name: 'ENG' }));
    expect(i18n.lang).toBe('en');
    expect(localStorage.getItem('ochistka.lang')).toBe('en');
    await fireEvent.click(screen.getByRole('button', { name: 'RU' }));
    expect(localStorage.getItem('ochistka.lang')).toBe('ru');
  });
});
