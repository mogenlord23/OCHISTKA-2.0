import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import ProgramsPage from './ProgramsPage.svelte';
import { listInstalled, listStartup, measureFolder, stopMeasure } from '../lib/programsApi';
import type { InstalledProgram } from '../lib/programsApi';

vi.mock('../lib/programsApi', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/programsApi')>()),
  listInstalled: vi.fn(),
  listStartup: vi.fn(),
  measureFolder: vi.fn(),
  stopMeasure: vi.fn(),
}));

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

const CHROME: InstalledProgram = {
  name: 'Google Chrome',
  publisher: 'Google LLC',
  version: '120.0',
  sizeBytes: 2048 * 1024,
  installLocation: 'C:\\Program Files\\Google\\Chrome',
  diskLetter: 'C:',
  kind: 'win32',
};

const NO_PATH: InstalledProgram = {
  name: 'Old Tool',
  publisher: null,
  version: null,
  sizeBytes: null,
  installLocation: null,
  diskLetter: null,
  kind: 'other',
};

function mountInstalled() {
  vi.mocked(listInstalled).mockResolvedValue([CHROME, NO_PATH]);
  return render(ProgramsPage, { subsection: 'installed' });
}

describe('ProgramsPage installed programs', () => {
  it('lists programs and filters them by name or publisher', async () => {
    mountInstalled();
    expect(await screen.findByText('Google Chrome')).toBeTruthy();
    expect(screen.getByText('Old Tool')).toBeTruthy();

    const search = screen.getByRole('searchbox');
    await fireEvent.input(search, { target: { value: 'google' } });
    expect(screen.queryByText('Old Tool')).toBeNull();
    expect(screen.getByText('Google Chrome')).toBeTruthy();

    await fireEvent.input(search, { target: { value: 'нет такой' } });
    expect(screen.getByText('Ничего не найдено.')).toBeTruthy();
  });

  it('keeps the delete button disabled and shows the reason', async () => {
    mountInstalled();
    await fireEvent.click(await screen.findByRole('button', { name: /Google Chrome/ }));
    const remove = screen.getByRole('button', { name: 'Удалить' }) as HTMLButtonElement;
    expect(remove.disabled).toBe(true);
    expect(screen.getByText('Удаление программ пока не подключено')).toBeTruthy();
  });

  it('measures the install folder and shows an honest partial result', async () => {
    mountInstalled();
    vi.mocked(measureFolder).mockResolvedValue({
      path: CHROME.installLocation ?? '',
      bytes: 4096,
      files: 7,
      complete: false,
    });
    await fireEvent.click(await screen.findByRole('button', { name: /Google Chrome/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Размер папки' }));

    await waitFor(() => expect(measureFolder).toHaveBeenCalledWith(CHROME.installLocation));
    expect(await screen.findByText(/файлов: 7/)).toBeTruthy();
    expect(screen.getByText(/нижняя оценка/)).toBeTruthy();
  });

  it('disables folder measuring when the install folder is unknown', async () => {
    mountInstalled();
    await fireEvent.click(await screen.findByRole('button', { name: /Old Tool/ }));
    const measure = screen.getByRole('button', { name: 'Размер папки' }) as HTMLButtonElement;
    expect(measure.disabled).toBe(true);
    expect(screen.getByText('В реестре не указана папка установки')).toBeTruthy();
  });

  it('reports a stopped count as a notice, not as an error', async () => {
    mountInstalled();
    let reject: (err: unknown) => void = () => {};
    vi.mocked(measureFolder).mockReturnValue(
      new Promise((_resolve, rej) => {
        reject = rej;
      }),
    );
    vi.mocked(stopMeasure).mockResolvedValue(undefined);
    await fireEvent.click(await screen.findByRole('button', { name: /Google Chrome/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Размер папки' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Стоп' }));
    expect(stopMeasure).toHaveBeenCalled();
    reject({ kind: 'cancelled', message: 'операция остановлена пользователем' });
    expect(await screen.findByText('Подсчёт остановлен.')).toBeTruthy();
  });
});

describe('ProgramsPage startup', () => {
  it('renders the read-only startup list', async () => {
    vi.mocked(listStartup).mockResolvedValue([
      {
        name: 'Helper',
        command: 'C:\\Apps\\helper.exe',
        location: 'Реестр: HKCU, Run',
      },
      { name: 'Messenger', command: null, location: 'Папка «Автозагрузка» пользователя' },
    ]);
    render(ProgramsPage, { subsection: 'startup' });
    expect(await screen.findByText('Helper')).toBeTruthy();
    expect(screen.getByText('Messenger')).toBeTruthy();
    expect(screen.getByText('C:\\Apps\\helper.exe')).toBeTruthy();
    expect(screen.getByText(/Только чтение/)).toBeTruthy();
  });

  it('says so when startup is empty', async () => {
    vi.mocked(listStartup).mockResolvedValue([]);
    render(ProgramsPage, { subsection: 'startup' });
    expect(await screen.findByText('Автозагрузка пуста.')).toBeTruthy();
  });
});
