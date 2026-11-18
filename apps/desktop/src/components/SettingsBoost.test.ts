import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import SettingsPage from './SettingsPage.svelte';
import BoostPage from './BoostPage.svelte';
import { boostSetProfile, tweakSet, tweaksOverview, tweaksRevert } from '../lib/tweaksApi';
import type { TweaksOverview } from '../lib/types';

vi.mock('../lib/tweaksApi', () => ({
  tweaksOverview: vi.fn(),
  tweakSet: vi.fn(),
  tweaksRevert: vi.fn(),
  boostSetProfile: vi.fn(),
}));

function overview(over: Partial<Record<string, boolean>> = {}, profile: string | null = null) {
  const ids = ['transparency', 'web_search', 'spotlight', 'suggestions', 'game_dvr', 'game_mode'];
  return {
    tweaks: ids.map((id) => ({ id, applied: over[id] ?? false, hasBackup: over[id] ?? false })),
    boost: { profile, hasBackup: profile !== null },
  } satisfies TweaksOverview;
}

beforeEach(() => {
  vi.mocked(tweaksOverview).mockReset().mockResolvedValue(overview());
  vi.mocked(tweakSet).mockReset();
  vi.mocked(tweaksRevert).mockReset();
  vi.mocked(boostSetProfile).mockReset();
});

afterEach(() => {
  cleanup();
});

describe('SettingsPage', () => {
  it('lists the safe settings with the saved-state note and reads the state', async () => {
    render(SettingsPage);
    await waitFor(() => expect(tweaksOverview).toHaveBeenCalled());
    expect(screen.getByText('Убрать прозрачность Windows')).toBeTruthy();
    expect(screen.getByText('Убрать веб-подсказки из поиска')).toBeTruthy();
    expect(screen.getAllByText('Исходное состояние сохраняется перед применением')).toHaveLength(4);
    expect(screen.getByRole('button', { name: 'Обновить состояние' })).toBeTruthy();
  });

  it('changes nothing until the confirmation dialog is accepted', async () => {
    vi.mocked(tweakSet).mockResolvedValue(overview({ transparency: true }));
    render(SettingsPage);
    const box = (await screen.findByRole('checkbox', {
      name: /Убрать прозрачность/,
    })) as HTMLInputElement;
    await waitFor(() => expect(box.disabled).toBe(false));
    box.click();
    expect(await screen.findByRole('dialog')).toBeTruthy();
    expect(tweakSet).not.toHaveBeenCalled();

    screen.getByRole('button', { name: 'Отмена' }).click();
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(tweakSet).not.toHaveBeenCalled();

    box.click();
    (await screen.findByRole('button', { name: 'Применить' })).click();
    await waitFor(() => expect(tweakSet).toHaveBeenCalledWith('transparency', true));
    await waitFor(() => expect(box.checked).toBe(true));
  });

  it('reverts the section only after confirmation', async () => {
    vi.mocked(tweaksOverview).mockResolvedValue(overview({ transparency: true }));
    vi.mocked(tweaksRevert).mockResolvedValue(overview());
    render(SettingsPage);
    const revert = screen.getByRole('button', { name: 'Вернуть изменения раздела' });
    await waitFor(() => expect((revert as HTMLButtonElement).disabled).toBe(false));
    revert.click();
    expect(tweaksRevert).not.toHaveBeenCalled();
    (await screen.findByRole('button', { name: 'Вернуть' })).click();
    await waitFor(() => expect(tweaksRevert).toHaveBeenCalledWith('settings'));
  });

  it('disables the list with a plain reason when the system is not supported', async () => {
    vi.mocked(tweaksOverview).mockRejectedValue({ kind: 'unsupported', message: 'x' });
    render(SettingsPage);
    expect(await screen.findByRole('alert')).toBeTruthy();
    const box = screen.getByRole('checkbox', { name: /Убрать прозрачность/ }) as HTMLInputElement;
    expect(box.disabled).toBe(true);
  });
});

describe('BoostPage', () => {
  it('shows the three profiles and the separate parameter', async () => {
    render(BoostPage);
    await waitFor(() => expect(tweaksOverview).toHaveBeenCalled());
    for (const name of ['Базовый', 'Игровой', 'Максимум', 'Убрать фоновую запись игр']) {
      expect(screen.getByText(name)).toBeTruthy();
    }
  });

  it('marks the active profile and applies a profile after confirmation', async () => {
    vi.mocked(tweaksOverview).mockResolvedValue(overview({}, 'basic'));
    vi.mocked(boostSetProfile).mockResolvedValue(overview({}, 'gaming'));
    render(BoostPage);
    const basic = (await screen.findByRole('checkbox', { name: /Базовый/ })) as HTMLInputElement;
    await waitFor(() => expect(basic.checked).toBe(true));
    const gaming = screen.getByRole('checkbox', { name: /^Игровой/ }) as HTMLInputElement;
    gaming.click();
    expect(boostSetProfile).not.toHaveBeenCalled();
    (await screen.findByRole('button', { name: 'Применить' })).click();
    await waitFor(() => expect(boostSetProfile).toHaveBeenCalledWith('gaming'));
  });

  it('switches the background recording through the confirmed tweak command', async () => {
    vi.mocked(tweakSet).mockResolvedValue(overview({ game_dvr: true }));
    render(BoostPage);
    const box = (await screen.findByRole('checkbox', {
      name: /Убрать фоновую запись игр/,
    })) as HTMLInputElement;
    await waitFor(() => expect(box.disabled).toBe(false));
    box.click();
    (await screen.findByRole('button', { name: 'Применить' })).click();
    await waitFor(() => expect(tweakSet).toHaveBeenCalledWith('game_dvr', true));
  });
});
