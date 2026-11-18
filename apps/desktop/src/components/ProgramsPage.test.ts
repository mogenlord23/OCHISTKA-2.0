import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import ProgramsPage from './ProgramsPage.svelte';
import {
  listInstalled,
  listStartup,
  measureFolder,
  setStartupEnabled,
  stopMeasure,
  uninstallProgram,
} from '../lib/programsApi';
import type { InstalledProgram, StartupEntry } from '../lib/programsApi';

vi.mock('../lib/programsApi', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/programsApi')>()),
  getIcon: vi.fn().mockResolvedValue(null),
  getProgramIcon: vi.fn().mockResolvedValue(null),
  setStartupEnabled: vi.fn(),
  uninstallProgram: vi.fn(),
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
  id: 'google chrome|120.0|google llc',
  iconPath: null,
  canUninstall: true,
  uninstallReason: null,
};

const NO_PATH: InstalledProgram = {
  name: 'Old Tool',
  publisher: null,
  version: null,
  sizeBytes: null,
  installLocation: null,
  diskLetter: null,
  kind: 'other',
  id: 'old tool||',
  iconPath: null,
  canUninstall: false,
  uninstallReason: 'В реестре нет команды удаления',
};

function mountInstalled() {
  vi.mocked(listInstalled).mockResolvedValue([CHROME, NO_PATH]);
  return render(ProgramsPage, { subsection: 'installed' });
}

describe('ProgramsPage virtual list and icons', () => {
  it('renders only the visible window of a long list', async () => {
    const many: InstalledProgram[] = Array.from({ length: 2500 }, (_, i) => ({
      ...CHROME,
      name: `Program ${String(i).padStart(4, '0')}`,
      id: `p${i}`,
    }));
    vi.mocked(listInstalled).mockResolvedValue(many);
    render(ProgramsPage, { subsection: 'installed' });
    expect(await screen.findByText('Program 0000')).toBeTruthy();
    const table = screen.getByRole('table', { name: 'Программы' });
    expect(table.getAttribute('aria-rowcount')).toBe('2501');
    expect(table.querySelectorAll('[role="row"]').length).toBeLessThan(60);
  });

  it('asks the backend for the icon by program id, also for Store packages', async () => {
    const { getProgramIcon } = await import('../lib/programsApi');
    vi.mocked(getProgramIcon).mockResolvedValue('data:image/png;base64,QUJD');
    const store: InstalledProgram = { ...CHROME, name: 'Calc', id: 'calc||', kind: 'store' };
    vi.mocked(listInstalled).mockResolvedValue([store]);
    const { container } = render(ProgramsPage, { subsection: 'installed' });
    await vi.waitFor(() =>
      expect(container.querySelector('img')?.getAttribute('src')).toBe(
        'data:image/png;base64,QUJD',
      ),
    );
    expect(getProgramIcon).toHaveBeenCalledWith('calc||');
  });
});

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

  it('disables delete with the reason when the program has no uninstall command', async () => {
    mountInstalled();
    await fireEvent.click(await screen.findByRole('button', { name: /Old Tool/ }));
    const remove = screen.getByRole('button', { name: 'Удалить' }) as HTMLButtonElement;
    expect(remove.disabled).toBe(true);
    expect(remove.title).toBe('В реестре нет команды удаления');
    expect(screen.getByText(/Удалить недоступно: В реестре нет команды удаления/)).toBeTruthy();
  });

  it('asks for confirmation before uninstalling and never runs it on cancel', async () => {
    vi.mocked(uninstallProgram).mockResolvedValue(undefined);
    mountInstalled();
    await fireEvent.click(await screen.findByRole('button', { name: /Google Chrome/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Удалить' }));
    const dialog = screen.getByRole('dialog', { name: 'Удалить программу?' });
    expect(dialog.textContent).toContain('Google Chrome');
    expect(uninstallProgram).not.toHaveBeenCalled();

    await fireEvent.keyDown(dialog, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(uninstallProgram).not.toHaveBeenCalled();

    await fireEvent.click(screen.getByRole('button', { name: 'Удалить' }));
    await fireEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'Удалить' }),
    );
    expect(uninstallProgram).toHaveBeenCalledTimes(1);
    expect(uninstallProgram).toHaveBeenCalledWith(CHROME.id, true);
    expect(await screen.findByText(/Запущено удаление «Google Chrome»/)).toBeTruthy();
  });

  it('sorts by size: largest first, then smallest, unknown size always last', async () => {
    mountInstalled();
    await screen.findByText('Google Chrome');
    const names = () =>
      screen
        .getAllByRole('row')
        .slice(1)
        .map((row) => within(row).getAllByRole('button')[0].textContent?.trim());
    const sizeHead = () =>
      within(screen.getByRole('columnheader', { name: /^Размер/ })).getByRole('button');
    await fireEvent.click(sizeHead());
    expect(names()).toEqual(['Google Chrome', 'Old Tool']);
    await fireEvent.click(sizeHead());
    expect(names()).toEqual(['Google Chrome', 'Old Tool']);
    expect(screen.getByRole('columnheader', { name: /^Размер/ }).getAttribute('aria-sort')).toBe(
      'ascending',
    );
    await fireEvent.click(screen.getByRole('button', { name: /^Программа/ }));
    expect(names()).toEqual(['Google Chrome', 'Old Tool']);
  });

  it('shows sizes in human units and «Нет данных» for unknown values', async () => {
    mountInstalled();
    const row = (await screen.findByText('Google Chrome')).closest('[role="row"]') as HTMLElement;
    expect(within(row).getByText('2,0 МБ')).toBeTruthy();
    const empty = screen.getByText('Old Tool').closest('[role="row"]') as HTMLElement;
    expect(within(empty).getAllByText('Нет данных').length).toBe(2);
  });

  it('opens a context menu on right click and closes it on Escape', async () => {
    mountInstalled();
    const name = await screen.findByText('Google Chrome');
    await fireEvent.contextMenu(name.closest('[role="row"]') as HTMLElement, {
      clientX: 120,
      clientY: 90,
    });
    const menu = screen.getByRole('menu');
    const items = within(menu)
      .getAllByRole('menuitem')
      .map((item) => item.textContent?.trim());
    expect(items).toEqual(['Размер папки', 'Показать путь', 'Удалить…']);

    await fireEvent.keyDown(menu, { key: 'Escape' });
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('closes the context menu on an outside click', async () => {
    mountInstalled();
    const name = await screen.findByText('Google Chrome');
    await fireEvent.contextMenu(name.closest('[role="row"]') as HTMLElement, {
      clientX: 10,
      clientY: 10,
    });
    expect(screen.getByRole('menu')).toBeTruthy();
    await fireEvent.pointerDown(document.body);
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('menu: show path prints the folder, delete asks for confirmation', async () => {
    mountInstalled();
    const row = (await screen.findByText('Google Chrome')).closest('[role="row"]') as HTMLElement;
    await fireEvent.contextMenu(row, { clientX: 10, clientY: 10 });
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Показать путь' }));
    expect(screen.getByRole('status').textContent).toContain(CHROME.installLocation);

    await fireEvent.contextMenu(row, { clientX: 10, clientY: 10 });
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Удалить…' }));
    expect(screen.getByRole('dialog', { name: 'Удалить программу?' })).toBeTruthy();
    expect(uninstallProgram).not.toHaveBeenCalled();
  });

  it('menu disables items with a reason when they are not available', async () => {
    mountInstalled();
    const row = (await screen.findByText('Old Tool')).closest('[role="row"]') as HTMLElement;
    await fireEvent.contextMenu(row, { clientX: 10, clientY: 10 });
    const measure = screen.getByRole('menuitem', { name: 'Размер папки' }) as HTMLButtonElement;
    const remove = screen.getByRole('menuitem', { name: 'Удалить…' }) as HTMLButtonElement;
    expect(measure.disabled).toBe(true);
    expect(measure.title).toBe('В реестре не указана папка установки');
    expect(remove.disabled).toBe(true);
    expect(remove.title).toBe('В реестре нет команды удаления');
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

const HELPER: StartupEntry = {
  name: 'Helper',
  command: 'C:\\Apps\\helper.exe',
  location: 'Реестр: HKCU, Run',
  source: 'hkcuRun',
  key: 'Helper',
  enabled: true,
  previousEnabled: null,
};
const MESSENGER: StartupEntry = {
  name: 'Messenger',
  command: null,
  location: 'Папка «Автозагрузка» пользователя',
  source: 'userFolder',
  key: 'Messenger.lnk',
  enabled: false,
  previousEnabled: true,
};

describe('ProgramsPage startup', () => {
  it('renders the startup list with the state of every entry', async () => {
    vi.mocked(listStartup).mockResolvedValue([HELPER, MESSENGER]);
    render(ProgramsPage, { subsection: 'startup' });
    expect(await screen.findByText('Helper')).toBeTruthy();
    expect(screen.getByText('Messenger')).toBeTruthy();
    expect(screen.getByText('C:\\Apps\\helper.exe')).toBeTruthy();
    expect(screen.getByText('Включена')).toBeTruthy();
    expect(screen.getByText('Отключена')).toBeTruthy();
    expect(screen.getByText('раньше: включена')).toBeTruthy();
  });

  it('says so when startup is empty', async () => {
    vi.mocked(listStartup).mockResolvedValue([]);
    render(ProgramsPage, { subsection: 'startup' });
    expect(await screen.findByText('Автозагрузка пуста.')).toBeTruthy();
  });

  it('toggles only after confirmation, shows before and after, and reloads', async () => {
    vi.mocked(listStartup).mockResolvedValue([HELPER, MESSENGER]);
    vi.mocked(setStartupEnabled).mockResolvedValue({ previousEnabled: true, enabled: false });
    render(ProgramsPage, { subsection: 'startup' });
    await screen.findByText('Helper');
    const toggle = screen.getByRole('button', { name: /автозагрузку$/ }) as HTMLButtonElement;
    expect(toggle.disabled).toBe(true);

    await fireEvent.click(screen.getByRole('button', { name: 'Helper' }));
    expect(toggle.disabled).toBe(false);
    expect(toggle.textContent?.trim()).toBe('Отключить автозагрузку');
    await fireEvent.click(toggle);

    const dialog = screen.getByRole('dialog');
    expect(dialog.textContent).toContain('Helper');
    expect(setStartupEnabled).not.toHaveBeenCalled();
    await fireEvent.keyDown(dialog, { key: 'Escape' });
    expect(setStartupEnabled).not.toHaveBeenCalled();

    await fireEvent.click(toggle);
    await fireEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'Отключить' }),
    );
    expect(setStartupEnabled).toHaveBeenCalledWith(HELPER, false, true);
    expect(await screen.findByText(/было включено, стало отключено/)).toBeTruthy();
    await waitFor(() => expect(listStartup).toHaveBeenCalledTimes(2));
  });

  it('offers to enable a disabled entry', async () => {
    vi.mocked(listStartup).mockResolvedValue([MESSENGER]);
    render(ProgramsPage, { subsection: 'startup' });
    await fireEvent.click(await screen.findByRole('button', { name: 'Messenger' }));
    expect(screen.getByRole('button', { name: 'Включить автозагрузку' })).toBeTruthy();
  });
});
