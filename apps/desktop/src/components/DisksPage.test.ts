import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import DisksPage from './DisksPage.svelte';
import {
  analyzeCleanup,
  cleanSelected,
  listDrives,
  listQuarantine,
  purgeItem,
  stopAnalysis,
} from '../lib/disksApi';
import type { QuarantineEntry } from '../lib/disksApi';

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

afterEach(() => {
  cleanup();
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
});
