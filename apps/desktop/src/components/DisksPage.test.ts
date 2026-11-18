import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import DisksPage from './DisksPage.svelte';
import {
  analyzeCleanup,
  cleanSelected,
  listDrives,
  listQuarantine,
  purgeItem,
  restoreItem,
  stopAnalysis,
} from '../lib/disksApi';
import type { QuarantineEntry } from '../lib/disksApi';
import { getSystemOverview } from '../lib/api';
import { playHeehee } from '../lib/sound';
import { i18n } from '../lib/i18n.svelte';
import pageSource from './DisksPage.svelte?raw';

vi.mock('../lib/api', () => ({ getSystemOverview: vi.fn() }));
vi.mock('../lib/sound', () => ({ playHeehee: vi.fn() }));

vi.mock('../lib/disksApi', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/disksApi')>()),
  analyzeCleanup: vi.fn(),
  cleanSelected: vi.fn(),
  listDrives: vi.fn(),
  listQuarantine: vi.fn(),
  purgeItem: vi.fn(),
  restoreItem: vi.fn(),
  stopAnalysis: vi.fn(),
}));

beforeEach(() => {
  vi.mocked(getSystemOverview).mockRejectedValue(new Error('нет данных'));
});

afterEach(() => {
  cleanup();
  i18n.lang = 'ru';
  vi.resetAllMocks();
});

const CATEGORIES = [
  {
    id: 'temp',
    title: 'Временные файлы (TEMP)',
    found: true,
    fileCount: 3,
    bytes: 2048,
    complete: true,
  },
  { id: 'chrome-cache', title: 'Кэш Chrome', found: false, fileCount: 0, bytes: 0, complete: true },
];

const ENTRY: QuarantineEntry = {
  id: '0123456789abcdef',
  originalPath: 'C:\\Users\\Slava\\AppData\\Local\\Temp\\a.tmp',
  size: 10,
  sha256: 'ab',
  movedAtUnix: 0,
  reason: 'глубокая очистка',
  purgeToken: 'purge:0123456789abcdef',
};

function mountCleanup() {
  vi.mocked(listQuarantine).mockResolvedValue([]);
  vi.mocked(listDrives).mockResolvedValue(['C:\\']);
  return render(DisksPage, { subsection: 'cleanup' });
}

describe('DisksPage deep cleanup', () => {
  it('shows analysed categories with sizes and enables cleaning only after a choice', async () => {
    mountCleanup();
    vi.mocked(analyzeCleanup).mockResolvedValue(CATEGORIES);
    vi.mocked(cleanSelected).mockResolvedValue({ moved: 3, bytes: 2048, failed: 0, failures: [] });

    await fireEvent.click(screen.getByRole('button', { name: 'Анализировать' }));
    expect(await screen.findByText('Временные файлы (TEMP)')).toBeTruthy();
    expect(screen.getByText(/2\.0 КБ/)).toBeTruthy();
    expect(screen.getByText('не найдено')).toBeTruthy();

    const clean = screen.getByRole('button', { name: 'Очистить выбранное' }) as HTMLButtonElement;
    expect(clean.disabled).toBe(true);

    await fireEvent.click(screen.getByRole('checkbox', { name: /TEMP/ }));
    expect(clean.disabled).toBe(false);
    await fireEvent.click(clean);
    await waitFor(() => expect(cleanSelected).toHaveBeenCalledWith(['temp']));
  });

  it('calls stop while an analysis runs', async () => {
    mountCleanup();
    let finish: (value: typeof CATEGORIES) => void = () => {};
    vi.mocked(analyzeCleanup).mockReturnValue(new Promise((resolve) => (finish = resolve)));
    vi.mocked(stopAnalysis).mockResolvedValue(undefined);

    await fireEvent.click(screen.getByRole('button', { name: 'Анализировать' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Стоп' }));
    expect(stopAnalysis).toHaveBeenCalledTimes(1);
    finish(CATEGORIES);
  });

  it('does not purge until the separate confirmation dialog is confirmed', async () => {
    vi.mocked(listQuarantine).mockResolvedValue([ENTRY]);
    vi.mocked(listDrives).mockResolvedValue([]);
    vi.mocked(purgeItem).mockResolvedValue(ENTRY);
    render(DisksPage, { subsection: 'cleanup' });

    await fireEvent.click(await screen.findByRole('button', { name: 'Удалить навсегда' }));
    expect(screen.getByRole('alertdialog')).toBeTruthy();
    expect(purgeItem).not.toHaveBeenCalled();

    await fireEvent.click(screen.getByRole('button', { name: 'Отмена' }));
    expect(screen.queryByRole('alertdialog')).toBeNull();
    expect(purgeItem).not.toHaveBeenCalled();

    await fireEvent.click(screen.getByRole('button', { name: 'Удалить навсегда' }));
    const dialog = within(screen.getByRole('alertdialog'));
    await fireEvent.click(dialog.getByRole('button', { name: 'Удалить навсегда' }));
    await waitFor(() => expect(purgeItem).toHaveBeenCalledWith(ENTRY.id, ENTRY.purgeToken));
  });

  it('offers an alternative path when the original is occupied and restores there', async () => {
    vi.mocked(listQuarantine).mockResolvedValue([ENTRY]);
    vi.mocked(listDrives).mockResolvedValue([]);
    vi.mocked(restoreItem)
      .mockRejectedValueOnce({ kind: 'occupied', message: 'место занято' })
      .mockResolvedValueOnce(ENTRY);
    render(DisksPage, { subsection: 'cleanup' });

    await fireEvent.click(await screen.findByRole('button', { name: 'Восстановить' }));
    await waitFor(() => expect(screen.getByLabelText(/Другой путь/)).toBeTruthy());
    expect(restoreItem).toHaveBeenLastCalledWith(ENTRY.id, null);

    const alt = 'C:\\Users\\Slava\\AppData\\Local\\Temp\\b.tmp';
    await fireEvent.input(screen.getByLabelText(/Другой путь/), { target: { value: alt } });
    await fireEvent.click(screen.getByRole('button', { name: 'Восстановить' }));
    await waitFor(() => expect(restoreItem).toHaveBeenLastCalledWith(ENTRY.id, alt));
    await waitFor(() => expect(screen.queryByLabelText(/Другой путь/)).toBeNull());
  });
});

const GIB = 1024 ** 3;
function overview(disks: object[]) {
  return { disks, cpu: {}, memory: {}, osName: '', osVersion: '' } as never;
}

describe('DisksPage drive tiles', () => {
  it('shows letter, type, short model and aligned capacity and free rows', async () => {
    vi.mocked(listQuarantine).mockResolvedValue([]);
    vi.mocked(listDrives).mockResolvedValue([]);
    vi.mocked(getSystemOverview).mockResolvedValue(
      overview([
        {
          displayName: 'Windows',
          mountPoint: 'C:\\',
          totalBytes: 500 * GIB,
          usedBytes: 200 * GIB,
          usagePercent: 40,
          kind: 'ssd',
          model: 'Kingston NV2',
        },
        {
          displayName: 'Data',
          mountPoint: 'D:\\',
          totalBytes: 1000 * GIB,
          usedBytes: 100 * GIB,
          usagePercent: 10,
          kind: 'hdd',
          model: null,
        },
      ]),
    );
    const { container } = render(DisksPage, { subsection: 'cleanup' });
    expect(await screen.findByText('C:')).toBeTruthy();
    expect(screen.getByText('SSD')).toBeTruthy();
    expect(screen.getByText('Kingston NV2')).toBeTruthy();
    expect(screen.getByText('HDD')).toBeTruthy();
    const unknown = screen.getByText('Модель неизвестна');
    expect(unknown.getAttribute('title')).toBeNull();
    expect(screen.getAllByText('Объём').length).toBe(2);
    expect(screen.getAllByText('Свободно').length).toBe(2);
    expect(screen.getByText('500.0 ГБ')).toBeTruthy();
    expect(screen.getByText('300.0 ГБ')).toBeTruthy();
    expect(container.querySelectorAll('svg[data-kind="ssd"]').length).toBe(1);
    expect(container.querySelectorAll('svg[data-kind="hdd"]').length).toBe(1);
  });

  it('keeps tile columns on the 8 px grid', () => {
    const css = pageSource.slice(pageSource.indexOf('<style>'));
    expect(css).toMatch(/\.disk-grid\s*\{[^}]*grid-template-columns:\s*96px/);
    expect(css).toMatch(/\.disk-grid\s*\{[^}]*gap:\s*8px/);
  });
});

describe('DisksPage headings and language', () => {
  it.each([
    ['cleanup', 'Диски / Глубокая очистка'],
    ['files', 'Диски / Очистка файлов'],
    ['space', 'Диски / Анализ пространства'],
    ['privacy', 'Диски / Приватность'],
  ])('uses one heading shape for %s', async (sub, text) => {
    vi.mocked(listQuarantine).mockResolvedValue([]);
    vi.mocked(listDrives).mockResolvedValue([]);
    render(DisksPage, { subsection: sub });
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe(text);
  });

  it('switches the texts to English', async () => {
    i18n.lang = 'en';
    vi.mocked(listQuarantine).mockResolvedValue([]);
    vi.mocked(listDrives).mockResolvedValue([]);
    render(DisksPage, { subsection: 'cleanup' });
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Disks / Deep cleanup');
    expect(screen.getByRole('button', { name: 'Analyze' })).toBeTruthy();
    expect(screen.getByText('Quarantine is empty.')).toBeTruthy();
  });
});

describe('DisksPage easter egg', () => {
  it('plays the sound and runs the press animation on click', async () => {
    vi.mocked(listQuarantine).mockResolvedValue([]);
    vi.mocked(listDrives).mockResolvedValue([]);
    render(DisksPage, { subsection: 'cleanup' });
    const egg = screen.getByRole('button', { name: /Портрет/ });
    await fireEvent.click(egg);
    expect(playHeehee).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(egg.classList.contains('pressed')).toBe(true));
  });

  it('is only on the deep cleanup subsection and respects reduced motion', () => {
    vi.mocked(listQuarantine).mockResolvedValue([]);
    vi.mocked(listDrives).mockResolvedValue([]);
    render(DisksPage, { subsection: 'files' });
    expect(screen.queryByRole('button', { name: /Портрет/ })).toBeNull();
    const css = pageSource.slice(pageSource.indexOf('<style>'));
    expect(css).toMatch(/object-fit:\s*cover/);
    const media = css.match(/@media \(prefers-reduced-motion: reduce\)\s*\{([\s\S]*?)\n\s{2}\}/);
    expect(media?.[1] ?? '').toMatch(/animation:\s*none/);
  });
});
