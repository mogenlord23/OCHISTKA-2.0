import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import ServicesPage from './ServicesPage.svelte';
import {
  listServices,
  restoreServiceStart,
  setServiceStart,
  startService,
  stopService,
  type ServiceInfo,
} from '../lib/servicesApi';

vi.mock('../lib/servicesApi', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/servicesApi')>()),
  listServices: vi.fn(),
  startService: vi.fn(),
  stopService: vi.fn(),
  setServiceStart: vi.fn(),
  restoreServiceStart: vi.fn(),
}));

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

function svc(patch: Partial<ServiceInfo> = {}): ServiceInfo {
  return {
    name: 'Fax',
    displayName: 'Факс',
    state: 'stopped',
    startType: 'manual',
    previousStartType: null,
    guard: 'free',
    note: null,
    ...patch,
  };
}

const SPOOLER = svc({
  name: 'Spooler',
  displayName: 'Диспетчер печати',
  state: 'running',
  startType: 'auto',
  guard: 'careful',
  note: 'Перестанет работать печать на всех принтерах.',
});
const RPC = svc({
  name: 'RpcSs',
  displayName: 'Удалённый вызов процедур',
  state: 'running',
  startType: 'auto',
  guard: 'locked',
  note: 'Критичная служба Windows: менять её нельзя.',
});

function rowOf(text: string): HTMLElement {
  return screen.getByText(text).closest('[role="row"]') as HTMLElement;
}

async function mount(elevated = true, list: ServiceInfo[] = [svc(), SPOOLER, RPC]) {
  vi.mocked(listServices).mockResolvedValue({ elevated, services: list });
  render(ServicesPage);
  await screen.findByText('Факс');
}

describe('ServicesPage', () => {
  it('has no read-only note and shows state and start type from the system', async () => {
    await mount();
    expect(screen.queryByText(/только чтение/i)).toBeNull();
    const row = rowOf('Диспетчер печати');
    expect(within(row).getByText('Работает')).toBeTruthy();
    expect((within(row).getByRole('combobox') as HTMLSelectElement).value).toBe('auto');
  });

  it('starts a service at once and shows the actual state from the answer', async () => {
    vi.mocked(startService).mockResolvedValue(svc({ state: 'running' }));
    await mount();
    const toggle = within(rowOf('Факс')).getByRole('switch') as HTMLButtonElement;
    expect(toggle.getAttribute('aria-checked')).toBe('false');
    await fireEvent.click(toggle);
    expect(startService).toHaveBeenCalledWith('Fax');
    // Действие не вызывает лишнего перечитывания списка.
    expect(listServices).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(screen.getByRole('status').textContent).toContain('работает'));
    expect(within(rowOf('Факс')).getByRole('switch').getAttribute('aria-checked')).toBe('true');
  });

  it('shows the error and returns the switch when the start fails', async () => {
    vi.mocked(startService).mockRejectedValue({
      kind: 'failed',
      message: 'Служба не запустилась. Сейчас: остановлена.',
    });
    await mount();
    await fireEvent.click(within(rowOf('Факс')).getByRole('switch'));
    await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('не запустилась'));
    expect(within(rowOf('Факс')).getByRole('switch').getAttribute('aria-checked')).toBe('false');
  });

  it('asks confirmation with consequences before stopping a careful service', async () => {
    vi.mocked(stopService).mockResolvedValue({ ...SPOOLER, state: 'stopped' });
    await mount();
    await fireEvent.click(within(rowOf('Диспетчер печати')).getByRole('switch'));
    const dialog = screen.getByRole('dialog');
    expect(dialog.textContent).toContain('печать');
    expect(stopService).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Остановить' }));
    expect(stopService).toHaveBeenCalledWith('Spooler', true);
  });

  it('cancelling the confirmation never calls the backend', async () => {
    await mount();
    await fireEvent.click(within(rowOf('Диспетчер печати')).getByRole('switch'));
    await fireEvent.click(screen.getByRole('button', { name: 'Отмена' }));
    expect(stopService).not.toHaveBeenCalled();
  });

  it('stops an ordinary service without a dialog', async () => {
    vi.mocked(stopService).mockResolvedValue(svc({ state: 'stopped' }));
    await mount(true, [svc({ state: 'running' })]);
    await fireEvent.click(within(rowOf('Факс')).getByRole('switch'));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(stopService).toHaveBeenCalledWith('Fax', false);
  });

  it('changes the start type through the list and offers to restore the previous one', async () => {
    vi.mocked(setServiceStart).mockResolvedValue(
      svc({ startType: 'disabled', previousStartType: 'manual' }),
    );
    vi.mocked(restoreServiceStart).mockResolvedValue(svc());
    await mount();
    const select = within(rowOf('Факс')).getByRole('combobox') as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: 'disabled' } });
    expect(setServiceStart).toHaveBeenCalledWith('Fax', 'disabled', false);
    const restore = await screen.findByRole('button', { name: /Вернуть: вручную/ });
    await fireEvent.click(restore);
    expect(restoreServiceStart).toHaveBeenCalledWith('Fax');
  });

  it('disabling a careful service asks first', async () => {
    await mount();
    const select = within(rowOf('Диспетчер печати')).getByRole('combobox') as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: 'disabled' } });
    expect(screen.getByRole('dialog').textContent).toContain('Отключить');
    expect(setServiceStart).not.toHaveBeenCalled();
  });

  it('locks critical services with a reason', async () => {
    await mount();
    const row = rowOf('Удалённый вызов процедур');
    expect((within(row).getByRole('switch') as HTMLButtonElement).disabled).toBe(true);
    expect((within(row).getByRole('combobox') as HTMLSelectElement).disabled).toBe(true);
    expect(within(row).getByText(/защищена/)).toBeTruthy();
  });

  it('without admin rights everything is disabled and the reason is on screen', async () => {
    await mount(false);
    expect(screen.getByRole('status').textContent).toContain('от имени администратора');
    expect((within(rowOf('Факс')).getByRole('switch') as HTMLButtonElement).disabled).toBe(true);
    expect((within(rowOf('Факс')).getByRole('combobox') as HTMLSelectElement).disabled).toBe(true);
  });

  it('filters by search and by «only running»', async () => {
    await mount();
    await fireEvent.input(screen.getByRole('searchbox'), { target: { value: 'печат' } });
    expect(screen.queryByText('Факс')).toBeNull();
    expect(screen.getByText('Диспетчер печати')).toBeTruthy();
    await fireEvent.input(screen.getByRole('searchbox'), { target: { value: '' } });
    await fireEvent.click(screen.getByRole('switch', { name: 'Только работающие' }));
    expect(screen.queryByText('Факс')).toBeNull();
  });

  it('shows the read error from the backend', async () => {
    vi.mocked(listServices).mockRejectedValue({
      kind: 'unsupported',
      message: 'Службы недоступны на linux.',
    });
    render(ServicesPage);
    await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('недоступны'));
  });
});
