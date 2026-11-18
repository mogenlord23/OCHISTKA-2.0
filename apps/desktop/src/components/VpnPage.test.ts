import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import VpnPage from './VpnPage.svelte';
import {
  IMPORT_REASON,
  NETWORK_CHANGE_REASON,
  VPN_CONNECT_REASON,
  VPN_TUN_MODE_REASON,
} from '../lib/vpn';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

afterEach(() => {
  cleanup();
  invokeMock.mockReset();
});

const SAMPLE = [
  'vless://uuid@1.2.3.4:443#Germany',
  'trojan://pass@5.6.7.8:443#Finland',
  'banner line',
].join('\n');

describe('VpnPage: подписка и импорт', () => {
  it('shows the subscription card with an enabled import button', () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    expect(screen.getByText('Ссылка подписки / импорта')).toBeTruthy();
    const importButton = screen.getByRole('button', { name: 'Импорт' }) as HTMLButtonElement;
    expect(importButton.disabled).toBe(false);
  });

  it('rejects a non-https link with the https reason', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'http://example.com/sub' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    expect(screen.getByText('Разрешены только ссылки https://')).toBeTruthy();
    expect(screen.queryByText(IMPORT_REASON)).toBeNull();
  });

  it('shows the not-connected import reason for a valid https link', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'https://example.com/sub' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    expect(screen.getByText(IMPORT_REASON)).toBeTruthy();
  });

  it('parses pasted subscription text and lists servers with count and selection', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    // Однострочное поле: строки подписки приходят через пробел.
    await fireEvent.input(input, { target: { value: SAMPLE.split('\n').slice(0, 2).join(' ') } });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    expect(screen.getByText('Серверов: 2. Не распознано строк: 0.')).toBeTruthy();
    expect(screen.getByText('Серверы / 2')).toBeTruthy();
    const second = screen.getByRole('button', { name: /Finland/ });
    await fireEvent.click(second);
    expect(second.getAttribute('aria-pressed')).toBe('true');
  });

  it('shows the country code from a flag emoji', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'vless://a@1:1#🇱🇻 Латвия' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    expect(screen.getByText('LV')).toBeTruthy();
    expect(screen.getByText('Латвия')).toBeTruthy();
  });
});

describe('VpnPage: подключение и серверы', () => {
  it('shows the connect toggle off, the timer at zero, and the TUN reason on click', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const toggle = screen.getByRole('button', { name: 'Подключить' });
    expect(toggle.getAttribute('aria-pressed')).toBe('false');
    expect(screen.getByLabelText('Время подключения').textContent).toBe('00:00:00');

    await fireEvent.click(toggle);
    expect(screen.getByText(VPN_CONNECT_REASON)).toBeTruthy();
    expect(toggle.getAttribute('aria-pressed')).toBe('false');
    expect(screen.getByLabelText('Время подключения').textContent).toBe('00:00:00');
  });

  it('shows the TUN mode as a disabled dropdown with its reason', () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const tun = screen.getByRole('combobox', { name: 'Режим' }) as HTMLSelectElement;
    expect(tun.disabled).toBe(true);
    expect(screen.getByText(VPN_TUN_MODE_REASON)).toBeTruthy();
  });

  it('shows an honest empty state when there are no servers', () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    expect(screen.getByText('Серверы / 0')).toBeTruthy();
    expect(screen.getByText(/Серверов пока нет/)).toBeTruthy();
  });
});

describe('VpnPage: маршруты и «Дополнительно»', () => {
  it('shows Zapret and Xbox DNS switches, both off with status text', () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    expect(screen.getByTestId('vpn-title').textContent).toBe('Дополнительно');
    expect(screen.getAllByText('Дополнительный инструмент')).toHaveLength(2);
    for (const name of ['Zapret', 'Xbox DNS']) {
      const sw = screen.getByRole('switch', { name }) as HTMLButtonElement;
      expect(sw.getAttribute('aria-checked')).toBe('false');
      expect(sw.closest('.switch-row')?.textContent).toContain('выключено');
    }
  });

  it('does not show GoodbyeDPI', () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    expect(screen.queryByText(/GoodbyeDPI/i)).toBeNull();
  });

  it('clicking a switch opens a confirmation dialog and changes nothing yet', async () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    const zapret = screen.getByRole('switch', { name: 'Zapret' });
    await fireEvent.click(zapret);
    const dialog = screen.getByRole('dialog', { name: 'Включить Zapret?' });
    expect(dialog).toBeTruthy();
    expect(screen.queryByText(NETWORK_CHANGE_REASON)).toBeNull();
    expect(zapret.getAttribute('aria-checked')).toBe('false');
  });

  it('confirming runs the stub: shows the network reason and leaves the switch off', async () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    const xbox = screen.getByRole('switch', { name: 'Xbox DNS' });
    await fireEvent.click(xbox);
    await fireEvent.click(screen.getByRole('button', { name: 'Включить' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByText(NETWORK_CHANGE_REASON)).toBeTruthy();
    expect(xbox.getAttribute('aria-checked')).toBe('false');
  });

  it('cancelling closes the dialog and shows no network reason', async () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    await fireEvent.click(screen.getByRole('switch', { name: 'Zapret' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Отмена' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.queryByText(NETWORK_CHANGE_REASON)).toBeNull();
  });

  it('Escape closes the dialog without any change', async () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    await fireEvent.click(screen.getByRole('switch', { name: 'Zapret' }));
    await fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.queryByText(NETWORK_CHANGE_REASON)).toBeNull();
  });
});

describe('VpnPage: вкладки', () => {
  it('shows four tabs, the first selected, with the lead line and big title', () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const tabs = screen.getAllByRole('tab').map((tab) => tab.textContent?.trim());
    expect(tabs).toEqual(['VPN', 'Дополнительно', 'Сеть', 'Соединения']);
    expect(screen.getByRole('tab', { name: 'VPN' }).getAttribute('aria-selected')).toBe('true');
    expect(screen.getByText('Сетевые функции и диагностика · только чтение')).toBeTruthy();
    expect(screen.getByTestId('vpn-title').textContent).toBe('VPN');
  });

  it('clicking a tab switches the content and the title', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    await fireEvent.click(screen.getByRole('tab', { name: 'Дополнительно' }));
    expect(screen.getByRole('switch', { name: 'Xbox DNS' })).toBeTruthy();
    expect(screen.getByTestId('vpn-title').textContent).toBe('Дополнительно');
    expect(screen.queryByText('Ссылка подписки / импорта')).toBeNull();
  });

  it('an unknown subsection id falls back to the VPN tab', () => {
    render(VpnPage, { props: { subsection: 'subscription' } });
    expect(screen.getByText('Ссылка подписки / импорта')).toBeTruthy();
  });
});

describe('VpnPage: вкладка «Сеть»', () => {
  it('lists adapters with MAC, traffic, rates in human units and addresses', async () => {
    invokeMock.mockResolvedValue([
      {
        name: 'Ethernet',
        mac: 'AA:BB:CC:DD:EE:FF',
        receivedBytes: 3 * 1024 * 1024,
        transmittedBytes: 2048,
        receiveRate: 1536,
        transmitRate: 0,
        addresses: ['192.168.0.5/24'],
      },
    ]);
    render(VpnPage, { props: { subsection: 'network' } });
    expect(await screen.findByText('Ethernet')).toBeTruthy();
    expect(invokeMock).toHaveBeenCalledWith('get_network_adapters');
    expect(screen.getByText('AA:BB:CC:DD:EE:FF')).toBeTruthy();
    expect(screen.getByText('3.0 МБ')).toBeTruthy();
    expect(screen.getByText('1.5 КБ/с')).toBeTruthy();
    expect(screen.getByText('0 Б/с')).toBeTruthy();
    expect(screen.getByText('192.168.0.5/24')).toBeTruthy();
  });

  it('shows the backend reason when reading is unsupported', async () => {
    invokeMock.mockRejectedValue({
      kind: 'unsupported',
      message: 'Чтение сети недоступно на Linux',
    });
    render(VpnPage, { props: { subsection: 'network' } });
    expect((await screen.findByRole('alert')).textContent).toContain('недоступно на Linux');
  });
});

describe('VpnPage: вкладка «Соединения»', () => {
  const rows = [
    {
      localAddr: '127.0.0.1',
      localPort: 8080,
      remoteAddr: '0.0.0.0',
      remotePort: 0,
      state: 'LISTEN',
      pid: 900,
      processName: 'zeta.exe',
    },
    {
      localAddr: '10.0.0.2',
      localPort: 50000,
      remoteAddr: '93.184.216.34',
      remotePort: 443,
      state: 'ESTABLISHED',
      pid: 12,
      processName: 'alpha.exe',
    },
  ];

  function firstColumn(): string[] {
    return screen
      .getAllByRole('row')
      .slice(1)
      .map((row) => row.querySelectorAll('td')[3]?.textContent ?? '');
  }

  it('shows the table and sorts by a header click, toggling direction', async () => {
    invokeMock.mockResolvedValue(rows);
    render(VpnPage, { props: { subsection: 'connections' } });
    expect(await screen.findByText('10.0.0.2:50000')).toBeTruthy();
    expect(screen.getByText('93.184.216.34:443')).toBeTruthy();
    expect(firstColumn()).toEqual(['alpha.exe', 'zeta.exe']);

    await fireEvent.click(screen.getByRole('button', { name: /Процесс/ }));
    expect(firstColumn()).toEqual(['zeta.exe', 'alpha.exe']);

    await fireEvent.click(screen.getByRole('button', { name: /PID/ }));
    expect(firstColumn()).toEqual(['alpha.exe', 'zeta.exe']);
    expect(screen.getByRole('columnheader', { name: /PID/ }).getAttribute('aria-sort')).toBe(
      'ascending',
    );
  });

  it('shows the backend reason when the list is unavailable', async () => {
    invokeMock.mockRejectedValue({
      kind: 'unsupported',
      message: 'Чтение сети недоступно на macOS',
    });
    render(VpnPage, { props: { subsection: 'connections' } });
    expect((await screen.findByRole('alert')).textContent).toContain('macOS');
  });
});
