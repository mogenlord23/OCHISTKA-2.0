import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import VpnConnectionsTab from './VpnConnectionsTab.svelte';
import { navigation } from '../lib/navigation.svelte';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

const CONNECTIONS = [
  {
    localAddr: '10.0.0.5',
    localPort: 50000,
    remoteAddr: '93.184.216.34',
    remotePort: 443,
    state: 'ESTABLISHED',
    pid: 4242,
    processName: 'browser.exe',
  },
  {
    localAddr: '10.0.0.5',
    localPort: 50001,
    remoteAddr: '1.1.1.1',
    remotePort: 53,
    state: 'SYN_SENT',
    pid: null,
    processName: null,
  },
];

const writeText = vi.fn(async () => {});

beforeEach(() => {
  Object.assign(navigator, { clipboard: { writeText } });
  navigation.processPid = null;
  invokeMock.mockImplementation(async (cmd: string, args?: { request?: { action: string } }) => {
    if (cmd === 'get_tcp_connections') return CONNECTIONS;
    if (cmd === 'processes_request') {
      const action = args?.request?.action;
      if (action === 'exePath') return { type: 'exePath', path: 'C:\\Apps\\browser.exe' };
      if (action === 'revealExe') return { type: 'revealed', path: 'C:\\Apps\\browser.exe' };
    }
    return undefined;
  });
});

afterEach(() => {
  cleanup();
  invokeMock.mockReset();
  writeText.mockClear();
});

async function rowOf(text: string) {
  return (await screen.findByRole('button', { name: text })).closest('tr') as HTMLElement;
}

describe('VpnConnectionsTab: выбор и меню', () => {
  it('shows the last connection list immediately while refreshing after reopening', async () => {
    const cache = {};
    const first = render(VpnConnectionsTab, {props: {cache}});
    await rowOf('10.0.0.5:50000');
    first.unmount();
    invokeMock.mockImplementation((cmd: string) => cmd === 'get_tcp_connections'
      ? new Promise(() => {}) : undefined);
    render(VpnConnectionsTab, {props: {cache}});
    expect(screen.getByRole('button', { name: '10.0.0.5:50000' })).toBeTruthy();
    expect(screen.queryByText('Читаем соединения…')).toBeNull();
  });

  it('selects a row, shows details with the file path, and a second click clears it', async () => {
    render(VpnConnectionsTab);
    const row = await rowOf('10.0.0.5:50000');
    await fireEvent.click(row);
    expect(row.classList.contains('is-selected')).toBe(true);
    await waitFor(() => expect(screen.getByText('C:\\Apps\\browser.exe')).toBeTruthy());
    expect(screen.getByText(/browser\.exe · PID 4242/)).toBeTruthy();
    await fireEvent.click(row);
    expect(row.classList.contains('is-selected')).toBe(false);
  });

  it('opens the menu at the click point with real actions and copies the remote address', async () => {
    render(VpnConnectionsTab);
    const row = await rowOf('10.0.0.5:50000');
    await fireEvent.contextMenu(row, { clientX: 120, clientY: 90 });
    const menu = await screen.findByRole('menu');
    expect(menu).toBeTruthy();
    const names = screen.getAllByRole('menuitem').map((i) => i.textContent?.trim());
    expect(names).toEqual([
      'Копировать удалённый адрес',
      'Копировать локальный адрес',
      'Показать процесс',
      'Открыть расположение файла',
    ]);
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Копировать удалённый адрес' }));
    expect(writeText).toHaveBeenCalledWith('93.184.216.34:443');
  });

  it('process actions are disabled when the process is unknown', async () => {
    render(VpnConnectionsTab);
    const row = await rowOf('10.0.0.5:50001');
    await fireEvent.contextMenu(row, { clientX: 10, clientY: 10 });
    await screen.findByRole('menu');
    expect(
      (screen.getByRole('menuitem', { name: 'Показать процесс' }) as HTMLButtonElement).disabled,
    ).toBe(true);
    expect(
      (screen.getByRole('menuitem', { name: 'Открыть расположение файла' }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
  });

  it('"Показать процесс" asks to open the processes section with that pid', async () => {
    render(VpnConnectionsTab);
    const row = await rowOf('10.0.0.5:50000');
    await fireEvent.contextMenu(row, { clientX: 5, clientY: 5 });
    await fireEvent.click(await screen.findByRole('menuitem', { name: 'Показать процесс' }));
    expect(navigation.processPid).toBe(4242);
  });

  it('"Открыть расположение файла" calls the backend for that pid', async () => {
    render(VpnConnectionsTab);
    const row = await rowOf('10.0.0.5:50000');
    await fireEvent.contextMenu(row, { clientX: 5, clientY: 5 });
    await fireEvent.click(
      await screen.findByRole('menuitem', { name: 'Открыть расположение файла' }),
    );
    await waitFor(() =>
      expect(
        invokeMock.mock.calls.some(
          (c) =>
            c[0] === 'processes_request' &&
            c[1]?.request?.action === 'revealExe' &&
            c[1].request.pid === 4242,
        ),
      ).toBe(true),
    );
  });
});
