import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import ProgramsPage from './ProgramsPage.svelte';
import { clearListCache } from '../lib/listCache';
import {
  listInstalled,
  listStartup,
  measureFolder,
  revealInExplorer,
  setStartupEnabled,
  stopMeasure,
  uninstallProgram,
} from '../lib/programsApi';
import type { InstalledProgram, StartupEntry } from '../lib/programsApi';

vi.mock('../lib/programsApi', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/programsApi')>()),
  getIcon: vi.fn().mockResolvedValue(null),
  setStartupEnabled: vi.fn(),
  uninstallProgram: vi.fn(),
  listInstalled: vi.fn(),
  listStartup: vi.fn(),
  measureFolder: vi.fn(),
  revealInExplorer: vi.fn(),
  stopMeasure: vi.fn(),
}));

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
  clearListCache();
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
    expect(remove.title).toBe('');
    expect(remove.getAttribute('aria-describedby')).toBe('delete-reason');
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

  it('opens sorted by size, largest first, unknown size last; the user can flip it', async () => {
    const SMALL: InstalledProgram = {
      ...CHROME,
      name: 'Small Tool',
      id: 'small',
      sizeBytes: 10 * 1024,
    };
    vi.mocked(listInstalled).mockResolvedValue([NO_PATH, SMALL, CHROME]);
    render(ProgramsPage, { subsection: 'installed' });
    await screen.findByText('Google Chrome');
    const names = () =>
      screen
        .getAllByRole('row')
        .slice(1)
        .map((row) => within(row).getAllByRole('button')[0].textContent?.trim());
    const sizeHead = () =>
      within(screen.getByRole('columnheader', { name: /^Размер/ })).getByRole('button');
    expect(names()).toEqual(['Google Chrome', 'Small Tool', 'Old Tool']);
    expect(screen.getByRole('columnheader', { name: /^Размер/ }).getAttribute('aria-sort')).toBe(
      'descending',
    );
    await fireEvent.click(sizeHead());
    expect(names()).toEqual(['Small Tool', 'Google Chrome', 'Old Tool']);
    expect(screen.getByRole('columnheader', { name: /^Размер/ }).getAttribute('aria-sort')).toBe(
      'ascending',
    );
    await fireEvent.click(screen.getByRole('button', { name: /^Программа/ }));
    expect(names()).toEqual(['Google Chrome', 'Old Tool', 'Small Tool']);
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
    expect(items).toEqual(['Размер папки', 'Показать в Проводнике', 'Удалить…']);

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

  it('menu: show in Explorer opens the folder, delete asks for confirmation', async () => {
    vi.mocked(revealInExplorer).mockResolvedValue(undefined);
    mountInstalled();
    const row = (await screen.findByText('Google Chrome')).closest('[role="row"]') as HTMLElement;
    await fireEvent.contextMenu(row, { clientX: 10, clientY: 10 });
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Показать в Проводнике' }));
    expect(revealInExplorer).toHaveBeenCalledWith(CHROME.installLocation);
    await waitFor(() =>
      expect(screen.getByRole('status').textContent).toContain(CHROME.installLocation as string),
    );

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

describe('ProgramsPage cache', () => {
  it('shows the previous list at once on reopening and refreshes it in the background', async () => {
    const first = mountInstalled();
    expect(await screen.findByText('Google Chrome')).toBeTruthy();
    first.unmount();
    let finish!: (list: InstalledProgram[]) => void;
    vi.mocked(listInstalled).mockReturnValue(new Promise((resolve) => (finish = resolve)));
    render(ProgramsPage, { subsection: 'installed' });
    // Свежий список ещё читается, а прошлый уже на экране, без «Читаю…».
    expect(screen.getByText('Google Chrome')).toBeTruthy();
    expect(screen.queryByText('Читаю список программ…')).toBeNull();
    finish([NO_PATH]);
    expect(await screen.findByText('Old Tool')).toBeTruthy();
    await waitFor(() => expect(screen.queryByText('Google Chrome')).toBeNull());
  });
});

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
    expect(toggle.getAttribute('aria-label')).toBe('Отключить автозагрузку');
    expect(toggle.textContent?.trim()).toBe('Отключить');
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

describe('ProgramsPage show in Explorer', () => {
  it('opens Explorer only on click of the path, never on hover', async () => {
    vi.mocked(revealInExplorer).mockResolvedValue(undefined);
    mountInstalled();
    const row = (await screen.findByText('Google Chrome')).closest('[role="row"]') as HTMLElement;
    await fireEvent.click(within(row).getByRole('button', { name: 'Google Chrome' }));
    const path = screen.getByRole('button', { name: CHROME.installLocation as string });
    await fireEvent.mouseEnter(path);
    await fireEvent.mouseOver(path);
    expect(revealInExplorer).not.toHaveBeenCalled();
    await fireEvent.click(path);
    expect(revealInExplorer).toHaveBeenCalledTimes(1);
    expect(revealInExplorer).toHaveBeenCalledWith(CHROME.installLocation);
  });

  it('the button explains itself when there is no install folder, and shows backend reasons', async () => {
    mountInstalled();
    const row = (await screen.findByText('Old Tool')).closest('[role="row"]') as HTMLElement;
    await fireEvent.click(within(row).getByRole('button', { name: 'Old Tool' }));
    const button = screen.getByRole('button', {
      name: 'Показать в Проводнике',
    }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    expect(button.title).toBe('');
    expect(button.getAttribute('aria-describedby')).toBe('path-note');
    expect(document.getElementById('path-note')?.textContent).toContain(
      'Папка установки не указана',
    );

    vi.mocked(revealInExplorer).mockRejectedValue({
      kind: 'reveal',
      message: 'Файл или папка не найдены: их, возможно, уже удалили или перенесли.',
    });
    const chrome = screen.getByText('Google Chrome').closest('[role="row"]') as HTMLElement;
    await fireEvent.click(within(chrome).getByRole('button', { name: 'Google Chrome' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Показать в Проводнике' }));
    await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('не найдены'));
  });
});
