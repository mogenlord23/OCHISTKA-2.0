import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import VpnPage from './VpnPage.svelte';
import fs from 'node:fs';
import path from 'node:path';
import { i18n } from '../lib/i18n.svelte';
import { VPN_BACKGROUND, VPN_TUN_MODE_REASON } from '../lib/vpn';

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
  });

  it('accepts a valid https link with a short status and no not-connected text', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'https://example.com/sub' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    expect(screen.getByRole('status').textContent).toBe('Ссылка проверена');
    expect(document.body.textContent).not.toMatch(/не подключ/i);
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
  it('shows the connect toggle off and disabled, the timer at zero, no not-connected text', () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const toggle = screen.getByRole('button', { name: 'Подключить' }) as HTMLButtonElement;
    expect(toggle.getAttribute('aria-pressed')).toBe('false');
    expect(toggle.disabled).toBe(true);
    expect(screen.getByLabelText('Время подключения').textContent).toBe('00:00:00');
    expect(document.body.textContent).not.toMatch(/не подключ/i);
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
  it('shows Zapret and Xbox DNS cards with disabled switches that are off', () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    expect(screen.getAllByText('Дополнительный инструмент')).toHaveLength(2);
    for (const name of ['Zapret', 'Xbox DNS']) {
      const sw = screen.getByRole('switch', { name }) as HTMLButtonElement;
      expect(sw.getAttribute('aria-checked')).toBe('false');
      expect(sw.disabled).toBe(true);
      expect(sw.closest('.switch-row')?.textContent).toContain('Выключено');
    }
  });

  it('does not show GoodbyeDPI', () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    expect(screen.queryByText(/GoodbyeDPI/i)).toBeNull();
  });

  it('clicking a disabled switch opens nothing and shows no text', async () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    await fireEvent.click(screen.getByRole('switch', { name: 'Zapret' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.body.textContent).not.toMatch(/не подключ|заглушк/i);
  });
});

describe('VpnPage: общий шаблон', () => {
  it('draws no own title, lead, tab bar or poster (PageShell owns them)', () => {
    const { container } = render(VpnPage, { props: { subsection: 'vpn' } });
    expect(screen.queryByRole('tablist')).toBeNull();
    expect(screen.queryByTestId('vpn-title')).toBeNull();
    expect(container.querySelector('h1')).toBeNull();
    expect(container.querySelector('img')).toBeNull();
    expect(screen.queryByText('Сетевые функции и диагностика · только чтение')).toBeNull();
  });

  it('an unknown subsection id falls back to the subscription cards', () => {
    render(VpnPage, { props: { subsection: 'subscription' } });
    expect(screen.getByText('Ссылка подписки / импорта')).toBeTruthy();
  });

  it('the subscription tab uses the same card class as the routes tab', () => {
    const first = render(VpnPage, { props: { subsection: 'vpn' } });
    expect(first.container.querySelectorAll('section.card').length).toBeGreaterThanOrEqual(3);
    first.unmount();
    const second = render(VpnPage, { props: { subsection: 'advanced' } });
    expect(second.container.querySelectorAll('section.card')).toHaveLength(2);
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

  it('has no checkboxes or tick marks', async () => {
    invokeMock.mockResolvedValue(rows);
    const { container } = render(VpnPage, { props: { subsection: 'connections' } });
    await screen.findByText('10.0.0.2:50000');
    expect(container.querySelectorAll('input[type="checkbox"]')).toHaveLength(0);
    expect(container.textContent).not.toMatch(/[✓✔☑]/);
  });

  it('a clicked row gets exactly one marker; another click moves it', async () => {
    invokeMock.mockResolvedValue(rows);
    const { container } = render(VpnPage, { props: { subsection: 'connections' } });
    await screen.findByText('10.0.0.2:50000');
    const marked = () => container.querySelectorAll('tr[aria-current="true"]');
    expect(marked()).toHaveLength(0);
    const bodyRows = () => container.querySelectorAll('tbody tr.row');
    await fireEvent.click(bodyRows()[0]!);
    expect(marked()).toHaveLength(1);
    await fireEvent.click(bodyRows()[1]!);
    expect(marked()).toHaveLength(1);
    expect(bodyRows()[1]!.getAttribute('aria-current')).toBe('true');
  });

  it('keeps row height on the 8 px grid', () => {
    const source = fs.readFileSync(path.join(__dirname, 'VpnConnectionsTab.svelte'), 'utf8');
    expect(source).toMatch(/const ROW_H = 32;/);
  });
});

describe('VpnPage: язык', () => {
  it('shows English texts after switching to ENG', () => {
    i18n.lang = 'en';
    try {
      render(VpnPage, { props: { subsection: 'vpn' } });
      expect(screen.getByText('Subscription link / import')).toBeTruthy();
      expect(screen.getByRole('button', { name: 'Import' })).toBeTruthy();
    } finally {
      i18n.lang = 'ru';
    }
  });
});

describe('VPN files: no not-connected text, green background, no old poster', () => {
  const dir = path.join(__dirname, '..');
  const files = [
    'lib/vpn.ts',
    'lib/vpnNetwork.ts',
    'components/VpnPage.svelte',
    'components/VpnConnectionsTab.svelte',
    'components/VpnNetworkTab.svelte',
  ];

  it('never contains the not-connected wording', () => {
    for (const file of files) {
      const text = fs.readFileSync(path.join(dir, file), 'utf8');
      expect(text, file).not.toMatch(/пока не подключ|не подключен|не подключён|not connected/i);
    }
  });

  it('paints the page with the green section background and drops the poster', () => {
    const page = fs.readFileSync(path.join(dir, 'components/VpnPage.svelte'), 'utf8');
    expect(page).toContain(VPN_BACKGROUND);
    expect(page).not.toMatch(/pop-art|playHeehee/);
  });
});
