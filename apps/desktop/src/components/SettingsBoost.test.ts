import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import SettingsPage from './SettingsPage.svelte';
import BoostPage from './BoostPage.svelte';
import {
  boostSetProfile,
  openActivationSettings,
  tweakSet,
  tweaksOverview,
  tweaksRevert,
  windowsActivation,
} from '../lib/tweaksApi';
import { BOOST_EXTRAS, SETTINGS_TWEAKS } from '../lib/tweaksText';
import type { TweakState, TweaksOverview } from '../lib/types';

vi.mock('../lib/tweaksApi', () => ({
  tweaksOverview: vi.fn(),
  tweakSet: vi.fn(),
  tweaksRevert: vi.fn(),
  boostSetProfile: vi.fn(),
  windowsActivation: vi.fn(),
  openActivationSettings: vi.fn(),
}));

const ALL_IDS = [...SETTINGS_TWEAKS, ...BOOST_EXTRAS].map((tweak) => tweak.id);
const ADMIN_IDS = new Set(['login_blur', 'power_throttling', 'hags', 'vbs_off']);

function stateOf(id: string, applied: boolean, elevated: boolean): TweakState {
  const needsAdmin = ADMIN_IDS.has(id) && !elevated;
  return {
    id,
    applied,
    hasBackup: applied,
    availability: needsAdmin ? 'needsAdmin' : 'available',
    reason: needsAdmin
      ? 'Нужны права администратора: запусти OCHISTKA от имени администратора.'
      : null,
    restart: id === 'hags' || id === 'vbs_off' ? 'reboot' : id === 'widgets' ? 'explorer' : 'none',
    dangerous: id === 'vbs_off',
  };
}

function overview(
  over: Partial<Record<string, boolean>> = {},
  profile: string | null = null,
  elevated = true,
  patch: Partial<Record<string, Partial<TweakState>>> = {},
) {
  return {
    tweaks: ALL_IDS.map((id) => ({
      ...stateOf(id, over[id] ?? false, elevated),
      ...(patch[id] ?? {}),
    })),
    boost: { profile, hasBackup: profile !== null, steps: [] },
    build: 22631,
    elevated,
  } satisfies TweaksOverview;
}

const switchNamed = async (name: RegExp) =>
  (await screen.findByRole('switch', { name })) as HTMLButtonElement;
const isOn = (el: HTMLElement) => el.getAttribute('aria-checked') === 'true';

beforeEach(() => {
  vi.mocked(tweaksOverview).mockReset().mockResolvedValue(overview());
  vi.mocked(tweakSet).mockReset();
  vi.mocked(tweaksRevert).mockReset();
  vi.mocked(boostSetProfile).mockReset();
  vi.mocked(windowsActivation)
    .mockReset()
    .mockResolvedValue({ status: 'licensed', graceDays: null });
  vi.mocked(openActivationSettings).mockReset().mockResolvedValue(undefined);
});

afterEach(() => {
  cleanup();
});

describe('SettingsPage', () => {
  it('shows the saved-state note once and collapsible groups; no search field and no category chips', async () => {
    render(SettingsPage);
    await waitFor(() => expect(tweaksOverview).toHaveBeenCalled());
    expect(screen.queryByText(/Исходное состояние сохраняется перед применением/)).toBeNull();
    // Первая группа открыта, остальные свёрнуты.
    expect(screen.getByText('Убрать прозрачность Windows')).toBeTruthy();
    expect(screen.queryByText('Убрать веб-подсказки из поиска')).toBeNull();
    const heads = screen.getAllByRole('button', { expanded: false });
    expect(heads.length).toBe(5);
    await fireEvent.click(screen.getByRole('button', { name: /Поиск Windows/, expanded: false }));
    expect(screen.getByText('Убрать веб-подсказки из поиска')).toBeTruthy();
    expect(screen.queryByRole('searchbox')).toBeNull();
    expect(screen.queryByRole('navigation', { name: 'Категории' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Вернуть изменения' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Обновить состояние' })).toBeTruthy();
  });

  it('shows the real activation status and opens the Windows page by the button', async () => {
    render(SettingsPage);
    expect(await screen.findByText('Windows активирована')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Открыть параметры активации' }));
    expect(openActivationSettings).toHaveBeenCalled();
    // Это информационный блок, не переключатель.
    expect(within(screen.getByLabelText('Активация Windows')).queryByRole('switch')).toBeNull();
  });

  it('says plainly when the activation cannot be read', async () => {
    vi.mocked(windowsActivation).mockRejectedValue({ kind: 'probe', message: 'x' });
    render(SettingsPage);
    expect(await screen.findByText('Состояние активации не удалось определить')).toBeTruthy();
  });

  it('applies a setting at once, without any dialog, and shows it on in the same tick', async () => {
    let finish!: (value: TweaksOverview) => void;
    vi.mocked(tweakSet).mockReturnValue(new Promise((resolve) => (finish = resolve)));
    render(SettingsPage);
    const sw = await switchNamed(/Убрать прозрачность/);
    await waitFor(() => expect(sw.disabled).toBe(false));
    sw.click();
    expect(screen.queryByRole('dialog')).toBeNull();
    await waitFor(() => expect(isOn(sw)).toBe(true));
    expect(sw.getAttribute('aria-busy')).toBe('true');
    await waitFor(() => expect(tweakSet).toHaveBeenCalledWith('transparency', true));
    finish(overview({ transparency: true }));
    await waitFor(() => expect(sw.getAttribute('aria-busy')).toBe('false'));
    expect(isOn(sw)).toBe(true);
  });

  it('rolls the switch back and says it briefly in the row when applying fails', async () => {
    vi.mocked(tweakSet).mockRejectedValue({ kind: 'probe', message: 'HRESULT 0x80070005' });
    render(SettingsPage);
    const sw = await switchNamed(/Убрать прозрачность/);
    await waitFor(() => expect(sw.disabled).toBe(false));
    sw.click();
    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toBe('Не получилось изменить настройку. Попробуй ещё раз.');
    await waitFor(() => expect(isOn(sw)).toBe(false));
  });

  it('shows the real reason from the system instead of the general phrase', async () => {
    vi.mocked(tweakSet).mockRejectedValue({
      kind: 'notApplicable',
      message: 'Windows сразу вернула прежнее значение.',
    });
    render(SettingsPage);
    const sw = await switchNamed(/Убрать прозрачность/);
    await waitFor(() => expect(sw.disabled).toBe(false));
    sw.click();
    expect((await screen.findByRole('alert')).textContent).toBe(
      'Windows сразу вернула прежнее значение.',
    );
  });

  it('shows a setting forced by a computer policy as on and locked, with the reason', async () => {
    vi.mocked(tweaksOverview).mockResolvedValue(
      overview({ ad_id: true }, null, true, {
        ad_id: { availability: 'managedByPolicy', reason: 'Включено политикой компьютера.' },
      }),
    );
    render(SettingsPage);
    await fireEvent.click(
      screen.getByRole('button', { name: /Конфиденциальность/, expanded: false }),
    );
    const sw = await switchNamed(/рекламный/i);
    expect(sw.disabled).toBe(true);
    expect(isOn(sw)).toBe(true);
    expect(await screen.findByText('Включено политикой компьютера.')).toBeTruthy();
  });

  it('keeps the same row height with and without a message: a failure replaces the note line', async () => {
    vi.mocked(tweakSet).mockRejectedValue({ kind: 'probe', message: 'x' });
    render(SettingsPage);
    const sw = await switchNamed(/Убрать прозрачность/);
    const row = sw.closest('.row') as HTMLElement;
    const lines = () => row.querySelectorAll('.meta').length;
    expect(lines()).toBe(1);
    await waitFor(() => expect(sw.disabled).toBe(false));
    sw.click();
    await screen.findByRole('alert');
    expect(lines()).toBe(1);
  });

  it('turns a setting off again with a second press', async () => {
    vi.mocked(tweaksOverview).mockResolvedValue(overview({ transparency: true }));
    vi.mocked(tweakSet).mockResolvedValue(overview());
    render(SettingsPage);
    const sw = await switchNamed(/Убрать прозрачность/);
    await waitFor(() => expect(isOn(sw)).toBe(true));
    sw.click();
    await waitFor(() => expect(tweakSet).toHaveBeenCalledWith('transparency', false));
    await waitFor(() => expect(isOn(sw)).toBe(false));
  });

  it('greys out a setting the Windows version does not support, with the reason', async () => {
    vi.mocked(tweaksOverview).mockResolvedValue(
      overview({}, null, true, {
        login_blur: {
          availability: 'unsupportedVersion',
          reason: 'Эта версия Windows не поддерживает настройку.',
        },
      }),
    );
    render(SettingsPage);
    const sw = await switchNamed(/Размытие при входе/);
    expect(await screen.findByText('Эта версия Windows не поддерживает настройку.')).toBeTruthy();
    expect(sw.disabled).toBe(true);
    sw.click();
    expect(tweakSet).not.toHaveBeenCalled();
  });

  it('disables machine-wide settings without administrator rights and says why', async () => {
    vi.mocked(tweaksOverview).mockResolvedValue(overview({}, null, false));
    render(SettingsPage);
    const sw = await switchNamed(/Размытие при входе/);
    await waitFor(() => expect(sw.disabled).toBe(true));
    expect((await screen.findAllByText(/Нужны права администратора/)).length).toBeGreaterThan(0);
    expect(
      await screen.findByText(/Часть настроек меняет параметры всего компьютера/),
    ).toBeTruthy();
  });

  it('reverts the section at once, without a dialog', async () => {
    vi.mocked(tweaksOverview).mockResolvedValue(overview({ transparency: true }));
    vi.mocked(tweaksRevert).mockResolvedValue(overview());
    render(SettingsPage);
    const revert = screen.getByRole('button', { name: 'Вернуть изменения' });
    await waitFor(() => expect((revert as HTMLButtonElement).disabled).toBe(false));
    revert.click();
    expect(screen.queryByRole('dialog')).toBeNull();
    await waitFor(() => expect(tweaksRevert).toHaveBeenCalledWith('settings'));
  });

  it('disables the list with a plain reason when the system is not supported', async () => {
    vi.mocked(tweaksOverview).mockRejectedValue({ kind: 'unsupported', message: 'x' });
    render(SettingsPage);
    expect(await screen.findByRole('alert')).toBeTruthy();
    const sw = screen.getByRole('switch', { name: /Убрать прозрачность/ }) as HTMLButtonElement;
    expect(sw.disabled).toBe(true);
  });
});

describe('BoostPage', () => {
  it('shows the profile segments and the separate optimisations', async () => {
    render(BoostPage);
    await waitFor(() => expect(tweaksOverview).toHaveBeenCalled());
    const group = screen.getByRole('radiogroup', { name: 'Профиль для игр' });
    for (const name of ['Базовый', 'Игровой', 'Максимум']) {
      expect(within(group).getByRole('radio', { name })).toBeTruthy();
    }
    for (const tweak of BOOST_EXTRAS) expect(screen.getByText(tweak.title)).toBeTruthy();
    expect(screen.queryByText(/Исходное состояние сохраняется перед применением/)).toBeNull();
  });

  it('marks the active profile and switches at once, the old one clears in the same tick', async () => {
    vi.mocked(tweaksOverview).mockResolvedValue(overview({}, 'basic'));
    let finish!: (value: TweaksOverview) => void;
    vi.mocked(boostSetProfile).mockReturnValue(new Promise((resolve) => (finish = resolve)));
    render(BoostPage);
    const basic = await screen.findByRole('radio', { name: 'Базовый' });
    await waitFor(() => expect(isOn(basic)).toBe(true));
    const gaming = screen.getByRole('radio', { name: 'Игровой' });
    gaming.click();
    expect(screen.queryByRole('dialog')).toBeNull();
    await waitFor(() => expect(isOn(gaming)).toBe(true));
    expect(isOn(basic)).toBe(false);
    await waitFor(() => expect(boostSetProfile).toHaveBeenCalledWith('gaming'));
    finish(overview({}, 'gaming'));
    await waitFor(() => expect(isOn(gaming)).toBe(true));
  });

  it('releases the profile with a second press', async () => {
    vi.mocked(tweaksOverview).mockResolvedValue(overview({}, 'basic'));
    vi.mocked(boostSetProfile).mockResolvedValue(overview());
    render(BoostPage);
    const basic = await screen.findByRole('radio', { name: 'Базовый' });
    await waitFor(() => expect(isOn(basic)).toBe(true));
    basic.click();
    await waitFor(() => expect(boostSetProfile).toHaveBeenCalledWith(null));
  });

  it('puts the profile back and says why when it cannot be applied', async () => {
    vi.mocked(boostSetProfile).mockRejectedValue({ kind: 'probe', message: 'powercfg' });
    render(BoostPage);
    const max = await screen.findByRole('radio', { name: 'Максимум' });
    await waitFor(() => expect((max as HTMLButtonElement).disabled).toBe(false));
    max.click();
    expect((await screen.findByRole('alert')).textContent).toContain('Не получилось');
    await waitFor(() => expect(isOn(max)).toBe(false));
  });

  it('switches the background recording through the tweak command without a dialog', async () => {
    vi.mocked(tweakSet).mockResolvedValue(overview({ game_dvr: true }));
    render(BoostPage);
    const sw = await switchNamed(/Убрать фоновую запись игр/);
    await waitFor(() => expect(sw.disabled).toBe(false));
    sw.click();
    expect(screen.queryByRole('dialog')).toBeNull();
    await waitFor(() => expect(tweakSet).toHaveBeenCalledWith('game_dvr', true));
    await waitFor(() => expect(isOn(sw)).toBe(true));
  });

  it('asks for confirmation only for VBS/HVCI and does nothing until it is given', async () => {
    vi.mocked(tweakSet).mockResolvedValue(overview({ vbs_off: true }));
    render(BoostPage);
    const sw = await switchNamed(/Игровой режим без VBS\/HVCI/);
    await waitFor(() => expect(sw.disabled).toBe(false));
    sw.click();
    const dialog = await screen.findByRole('dialog');
    expect(dialog.textContent).toContain('защита станет слабее');
    expect(tweakSet).not.toHaveBeenCalled();
    expect(isOn(sw)).toBe(false);
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Отмена' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(tweakSet).not.toHaveBeenCalled();
    sw.click();
    await fireEvent.click(
      within(await screen.findByRole('dialog')).getByRole('button', { name: 'Отключить' }),
    );
    await waitFor(() => expect(tweakSet).toHaveBeenCalledWith('vbs_off', true, true));
  });

  it('turning VBS back on (switch off) needs no dialog', async () => {
    vi.mocked(tweaksOverview).mockResolvedValue(overview({ vbs_off: true }));
    vi.mocked(tweakSet).mockResolvedValue(overview());
    render(BoostPage);
    const sw = await switchNamed(/Игровой режим без VBS\/HVCI/);
    await waitFor(() => expect(isOn(sw)).toBe(true));
    sw.click();
    expect(screen.queryByRole('dialog')).toBeNull();
    await waitFor(() => expect(tweakSet).toHaveBeenCalledWith('vbs_off', false));
  });

  it('greys out optimisations that need administrator rights, with the reason', async () => {
    vi.mocked(tweaksOverview).mockResolvedValue(overview({}, null, false));
    render(BoostPage);
    const sw = await switchNamed(/Планирование GPU/);
    await waitFor(() => expect(sw.disabled).toBe(true));
    expect((await screen.findAllByText(/Нужны права администратора/)).length).toBeGreaterThan(0);
    const dvr = await switchNamed(/Убрать фоновую запись игр/);
    expect(dvr.disabled).toBe(false);
  });

  it('says when a change takes effect later', async () => {
    render(BoostPage);
    await waitFor(() =>
      expect(screen.getAllByText(/Вступит в силу после перезагрузки/).length).toBe(2),
    );
  });
});
