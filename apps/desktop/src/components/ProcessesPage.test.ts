import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import ProcessesPage from './ProcessesPage.svelte';
import { endTask, exePath, listProcesses, listServices } from '../lib/processesApi';
import type { ProcessInfo } from '../lib/processesApi';

vi.mock('../lib/processesApi', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/processesApi')>()),
  endTask: vi.fn(),
  exePath: vi.fn(),
  listProcesses: vi.fn(),
  listServices: vi.fn(),
}));

const LIST: ProcessInfo[] = [
  { pid: 100, name: 'chrome.exe', cpuPercent: 3.25, memoryBytes: 2048, startTimeUnix: 111 },
  { pid: 2048, name: 'explorer.exe', cpuPercent: 0.5, memoryBytes: 1024, startTimeUnix: 222 },
  { pid: 4, name: 'System', cpuPercent: 0, memoryBytes: 0, startTimeUnix: 1 },
];

beforeEach(() => {
  vi.mocked(listProcesses).mockResolvedValue(LIST);
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.resetAllMocks();
});

async function mountProcesses() {
  const view = render(ProcessesPage, { subsection: 'processes' });
  await tick();
  await vi.waitFor(() => expect(screen.getByText('chrome.exe')).toBeTruthy());
  return view;
}

describe('ProcessesPage virtual list', () => {
  it('renders only the visible window of a long list but reports the full row count', async () => {
    const many: ProcessInfo[] = Array.from({ length: 3000 }, (_, i) => ({
      pid: 10_000 + i,
      name: `app${String(i).padStart(4, '0')}.exe`,
      cpuPercent: 0,
      memoryBytes: 1024,
      startTimeUnix: i,
    }));
    vi.mocked(listProcesses).mockResolvedValue(many);
    render(ProcessesPage, { subsection: 'processes' });
    await vi.waitFor(() => expect(screen.getByText('app0000.exe')).toBeTruthy());
    const table = screen.getByRole('table', { name: 'Процессы' });
    expect(table.getAttribute('aria-rowcount')).toBe('3001');
    const rows = table.querySelectorAll('[role="row"]').length;
    expect(rows).toBeGreaterThan(5);
    expect(rows).toBeLessThan(60);
    expect(screen.queryByText('app2999.exe')).toBeNull();
  });
});

describe('ProcessesPage list', () => {
  it('shows name, PID, CPU and formatted memory', async () => {
    await mountProcesses();
    const row = screen.getByText('chrome.exe').closest('[role="row"]') as HTMLElement;
    expect(within(row).getByText('100')).toBeTruthy();
    expect(within(row).getByText('3,25%')).toBeTruthy();
    expect(within(row).getByText('2,0 КБ')).toBeTruthy();
  });

  it('shows «Нет данных» when the system gives no memory figure', async () => {
    await mountProcesses();
    const row = screen.getByText('System').closest('[role="row"]') as HTMLElement;
    expect(within(row).getByText('Нет данных')).toBeTruthy();
  });

  it('sorts by a clicked header: largest first, then smallest, with aria-sort', async () => {
    await mountProcesses();
    const names = () =>
      screen
        .getAllByRole('row')
        .slice(1)
        .map((row) => within(row).getAllByRole('button')[0].textContent?.trim());
    expect(names()).toEqual(['chrome.exe', 'explorer.exe', 'System']);

    await fireEvent.click(screen.getByRole('button', { name: /^PID/ }));
    expect(names()).toEqual(['explorer.exe', 'chrome.exe', 'System']);
    expect(screen.getByRole('columnheader', { name: /^PID/ }).getAttribute('aria-sort')).toBe(
      'descending',
    );

    await fireEvent.click(screen.getByRole('button', { name: /^PID/ }));
    expect(names()).toEqual(['System', 'chrome.exe', 'explorer.exe']);
    expect(screen.getByRole('columnheader', { name: /^PID/ }).getAttribute('aria-sort')).toBe(
      'ascending',
    );

    await fireEvent.click(screen.getByRole('button', { name: /^CPU/ }));
    expect(names()).toEqual(['chrome.exe', 'explorer.exe', 'System']);

    await fireEvent.click(screen.getByRole('button', { name: /^RAM/ }));
    expect(names()).toEqual(['chrome.exe', 'explorer.exe', 'System']);
    await fireEvent.click(screen.getByRole('button', { name: /^RAM/ }));
    expect(names()).toEqual(['explorer.exe', 'chrome.exe', 'System']);
  });

  it('refreshes every 3 seconds while the processes subsection is open', async () => {
    vi.useFakeTimers();
    render(ProcessesPage, { subsection: 'processes' });
    await vi.advanceTimersByTimeAsync(0);
    expect(listProcesses).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(2999);
    expect(listProcesses).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1);
    expect(listProcesses).toHaveBeenCalledTimes(2);

    await vi.advanceTimersByTimeAsync(6000);
    expect(listProcesses).toHaveBeenCalledTimes(4);
  });

  it('stops refreshing when the component is removed', async () => {
    vi.useFakeTimers();
    const view = render(ProcessesPage, { subsection: 'processes' });
    await vi.advanceTimersByTimeAsync(0);
    view.unmount();
    await vi.advanceTimersByTimeAsync(9000);
    expect(listProcesses).toHaveBeenCalledTimes(1);
  });

  it('filters by name or PID', async () => {
    await mountProcesses();
    const search = screen.getByRole('searchbox', { name: 'Поиск по имени или PID' });

    await fireEvent.input(search, { target: { value: 'EXPLORER' } });
    expect(screen.queryByText('chrome.exe')).toBeNull();
    expect(screen.getByText('explorer.exe')).toBeTruthy();

    await fireEvent.input(search, { target: { value: '100' } });
    expect(screen.getByText('chrome.exe')).toBeTruthy();
    expect(screen.queryByText('explorer.exe')).toBeNull();
  });
});

describe('ProcessesPage end task', () => {
  async function selectAndAskEnd(name: string) {
    await fireEvent.click(screen.getByRole('button', { name }));
    await fireEvent.click(screen.getByRole('button', { name: 'Завершить задачу' }));
  }

  it('opens a confirmation dialog and does not kill before confirmation', async () => {
    vi.mocked(endTask).mockResolvedValue(undefined);
    await mountProcesses();
    await selectAndAskEnd('chrome.exe');

    const dialog = screen.getByRole('dialog', { name: 'Завершить задачу?' });
    expect(dialog.textContent).toContain('chrome.exe');
    expect(endTask).not.toHaveBeenCalled();
  });

  it('cancelling the dialog never calls the backend', async () => {
    await mountProcesses();
    await selectAndAskEnd('chrome.exe');
    await fireEvent.click(screen.getByRole('button', { name: 'Отмена' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(endTask).not.toHaveBeenCalled();
  });

  it('Escape closes the dialog without killing', async () => {
    await mountProcesses();
    await selectAndAskEnd('chrome.exe');
    await fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(endTask).not.toHaveBeenCalled();
  });

  it('confirming calls the backend once with confirmed=true and the start time', async () => {
    vi.mocked(endTask).mockResolvedValue(undefined);
    await mountProcesses();
    await selectAndAskEnd('chrome.exe');
    await fireEvent.click(screen.getByRole('button', { name: 'Завершить' }));

    expect(endTask).toHaveBeenCalledTimes(1);
    expect(endTask).toHaveBeenCalledWith(
      { pid: 100, name: 'chrome.exe', cpuPercent: 3.25, memoryBytes: 2048, startTimeUnix: 111 },
      true,
    );
    await vi.waitFor(() => expect(screen.getByRole('status').textContent).toContain('завершён'));
  });

  it('keeps the protected PID 4 button disabled', async () => {
    await mountProcesses();
    await fireEvent.click(screen.getByRole('button', { name: 'System' }));
    const end = screen.getByRole('button', { name: 'Завершить задачу' }) as HTMLButtonElement;
    expect(end.disabled).toBe(true);
    expect(endTask).not.toHaveBeenCalled();
  });

  it('shows the file path as text and does not open anything', async () => {
    vi.mocked(exePath).mockResolvedValue('C:\\Program Files\\Chrome\\chrome.exe');
    await mountProcesses();
    await fireEvent.click(screen.getByRole('button', { name: 'chrome.exe' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Расположение файла' }));
    await vi.waitFor(() =>
      expect(screen.getByText(/C:\\Program Files\\Chrome\\chrome.exe/)).toBeTruthy(),
    );
    expect(exePath).toHaveBeenCalledWith(100);
  });
});

describe('ProcessesPage services', () => {
  it('shows the placeholder reason from the backend', async () => {
    vi.mocked(listServices).mockRejectedValue({
      kind: 'notConnected',
      message: 'Список служб пока не подключён: нужен Service Control Manager',
    });
    render(ProcessesPage, { subsection: 'services' });
    await vi.waitFor(() =>
      expect(screen.getByRole('alert').textContent).toContain('Список служб пока не подключён'),
    );
    expect(listProcesses).not.toHaveBeenCalled();
  });
});
