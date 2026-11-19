import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import VpnPage from './VpnPage.svelte';
import { NETWORK_CHANGE_REASON, NO_SERVERS_REASON, TUN_MODE_REASON } from '../lib/vpn';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

type Handler = (args: Record<string, unknown> | undefined) => unknown;

const SERVERS = [
  {
    id: 's1',
    name: '🇱🇻 Латвия',
    protocol: 'vless',
    host: 'a',
    port: 443,
    country: 'LV',
    latencyMs: null,
    measured: false,
  },
  {
    id: 's2',
    name: 'Finland',
    protocol: 'trojan',
    host: 'b',
    port: 443,
    country: 'FI',
    latencyMs: null,
    measured: false,
  },
];

function makeSnapshot(over: Record<string, unknown> = {}) {
  return {
    servers: [],
    selected: null,
    subscriptionUrl: '',
    rules: { directDomains: [], proxyDomains: [], directApps: [], proxyApps: [], presets: [] },
    presets: [
      { id: 'ru-direct', title: 'Российские сайты напрямую', description: 'Зоны .ru, Яндекс, VK' },
    ],
    core: {
      available: true,
      reason: null,
      fetchCommand: 'powershell -File tools\\vpn-core\\fetch.ps1',
    },
    connection: {
      state: 'disconnected',
      serverId: null,
      serverName: null,
      since: null,
      message: 'Не подключено',
      verified: false,
    },
    ...over,
  };
}

/** Подставной бэкенд: команды -> ответы; остальное (сеть, события окна) отвечает пустотой. */
function backend(handlers: Record<string, Handler>) {
  invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    const handler = handlers[cmd];
    if (handler) return handler(args);
    if (cmd === 'get_network_adapters' || cmd === 'get_tcp_connections') return [];
    return undefined;
  });
}

const calls = (cmd: string) => invokeMock.mock.calls.filter((c) => c[0] === cmd);

beforeEach(() => {
  backend({ vpn_snapshot: () => makeSnapshot() });
});

afterEach(() => {
  cleanup();
  invokeMock.mockReset();
});

async function openLoaded(snapshot = makeSnapshot({ servers: SERVERS, selected: 's1' })) {
  backend({ vpn_snapshot: () => snapshot });
  render(VpnPage, { props: { subsection: 'vpn' } });
  await screen.findByText(`Серверы / ${snapshot.servers.length}`);
}

describe('VpnPage: подписка и импорт', () => {
  it('shows the subscription card with an enabled import button', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    expect(screen.getByText('Ссылка подписки / импорта')).toBeTruthy();
    const importButton = screen.getByRole('button', { name: 'Импорт' }) as HTMLButtonElement;
    expect(importButton.disabled).toBe(false);
    await waitFor(() => expect(calls('vpn_snapshot').length).toBeGreaterThan(0));
  });

  it('rejects an http link without calling the backend', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'http://example.com/sub' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    expect(screen.getByText('Разрешены только ссылки https://')).toBeTruthy();
    expect(calls('vpn_import')).toHaveLength(0);
  });

  it('sends an https link to the backend, then shows the count and the server list', async () => {
    let imported = false;
    backend({
      vpn_snapshot: () =>
        imported ? makeSnapshot({ servers: SERVERS, selected: 's1' }) : makeSnapshot(),
      vpn_import: () => {
        imported = true;
        return { count: 2, skipped: 1, issues: ['wireguard: не поддерживается'] };
      },
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await fireEvent.input(screen.getByLabelText('Ссылка подписки'), {
      target: { value: 'https://sub.example.com/x?t=1' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    expect(
      await screen.findByText('Серверов: 2. Пропущено записей: 1 (wireguard: не поддерживается).'),
    ).toBeTruthy();
    expect(calls('vpn_import')[0]?.[1]).toEqual({ input: 'https://sub.example.com/x?t=1' });
    expect(await screen.findByText('Серверы / 2')).toBeTruthy();
    expect(screen.getByText('LV')).toBeTruthy();
    expect(screen.getByText('Латвия')).toBeTruthy();
  });

  it('turns space-separated pasted links into lines', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot(),
      vpn_import: () => ({ count: 2, skipped: 0, issues: [] }),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await fireEvent.input(screen.getByLabelText('Ссылка подписки'), {
      target: { value: 'vless://a@h:1#A trojan://b@h:2#B' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    await screen.findByText('Серверов: 2.');
    expect(calls('vpn_import')[0]?.[1]).toEqual({ input: 'vless://a@h:1#A\ntrojan://b@h:2#B' });
  });

  it('shows the backend error text and keeps the page usable', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot(),
      vpn_import: () =>
        Promise.reject({ kind: 'probe', message: 'Сервис подписки ответил HTTP 403' }),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await fireEvent.input(screen.getByLabelText('Ссылка подписки'), {
      target: { value: 'https://sub.example.com/x' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    expect(await screen.findByText('Сервис подписки ответил HTTP 403')).toBeTruthy();
    expect((screen.getByRole('button', { name: 'Импорт' }) as HTMLButtonElement).disabled).toBe(
      false,
    );
  });

  it('shows a loading state on the button while the import runs', async () => {
    let finish: (v: unknown) => void = () => {};
    backend({
      vpn_snapshot: () => makeSnapshot(),
      vpn_import: () => new Promise((resolve) => (finish = resolve)),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await fireEvent.input(screen.getByLabelText('Ссылка подписки'), {
      target: { value: 'https://sub.example.com/x' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Импорт' }));
    const busy = (await screen.findByRole('button', { name: 'Загрузка…' })) as HTMLButtonElement;
    expect(busy.disabled).toBe(true);
    finish({ count: 1, skipped: 0, issues: [] });
    await screen.findByText('Серверов: 1.');
  });

  it('imports a chosen file as text', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot(),
      vpn_import: () => ({ count: 1, skipped: 0, issues: [] }),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    const file = new File(['trojan://p@h:443#Файл'], 'sub.txt', { type: 'text/plain' });
    await fireEvent.change(screen.getByLabelText('Файл подписки'), { target: { files: [file] } });
    await screen.findByText('Серверов: 1.');
    expect(calls('vpn_import')[0]?.[1]).toEqual({ input: 'trojan://p@h:443#Файл' });
  });

  it('refills the saved subscription link after a restart', async () => {
    backend({ vpn_snapshot: () => makeSnapshot({ subscriptionUrl: 'https://saved.example/sub' }) });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await waitFor(() =>
      expect((screen.getByLabelText('Ссылка подписки') as HTMLInputElement).value).toBe(
        'https://saved.example/sub',
      ),
    );
  });
});

describe('VpnPage: подключение и серверы', () => {
  it('without servers the connect button is disabled and says why', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText(NO_SERVERS_REASON);
    const toggle = screen.getByRole('button', { name: 'Подключить' });
    expect(toggle.getAttribute('aria-disabled')).toBe('true');
    await fireEvent.click(toggle);
    expect(calls('vpn_connect')).toHaveLength(0);
    expect(screen.getByLabelText('Время подключения').textContent).toBe('00:00:00');
  });

  it('shows the missing core with the exact fetch command and disables connecting', async () => {
    await openLoaded(
      makeSnapshot({
        servers: SERVERS,
        selected: 's1',
        core: {
          available: false,
          reason: 'файл sing-box.exe не найден',
          fetchCommand: 'powershell -File fetch.ps1',
        },
      }),
    );
    expect(screen.getByText('VPN-ядро не установлено')).toBeTruthy();
    expect(screen.getByText('powershell -File fetch.ps1')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Подключить' }).getAttribute('aria-disabled')).toBe(
      'true',
    );
    expect(screen.getByText(/VPN-ядро не найдено: файл sing-box.exe не найден/)).toBeTruthy();
  });

  it('connect calls the backend, then shows the disconnect state and the status', async () => {
    const connected = {
      state: 'connected',
      serverId: 's1',
      serverName: 'Латвия',
      since: Math.floor(Date.now() / 1000) - 65,
      message: 'Подключено · Латвия · передача данных проверена',
      verified: true,
    };
    backend({
      vpn_snapshot: () => makeSnapshot({ servers: SERVERS, selected: 's1' }),
      vpn_connect: () => connected,
      vpn_disconnect: () => makeSnapshot().connection,
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText('Серверы / 2');
    await fireEvent.click(screen.getByRole('button', { name: 'Подключить' }));
    const off = await screen.findByRole('button', { name: 'Отключить' });
    expect(off.getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByText('Подключено · Латвия · передача данных проверена')).toBeTruthy();
    expect(screen.getByLabelText('Время подключения').textContent).toMatch(/^00:01:0\d$/);
    await fireEvent.click(off);
    const on = await screen.findByRole('button', { name: 'Подключить' });
    expect(on.getAttribute('aria-pressed')).toBe('false');
    expect(calls('vpn_disconnect')).toHaveLength(1);
  });

  it('a failed connect shows the reason and leaves the button ready', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot({ servers: SERVERS, selected: 's1' }),
      vpn_connect: () =>
        Promise.reject({ kind: 'probe', message: 'VPN-ядро не приняло параметры сервера' }),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText('Серверы / 2');
    await fireEvent.click(screen.getByRole('button', { name: 'Подключить' }));
    expect(await screen.findByText('VPN-ядро не приняло параметры сервера')).toBeTruthy();
    const again = await screen.findByRole('button', { name: 'Подключить' });
    expect(again.getAttribute('aria-disabled')).toBe('false');
  });

  it('shows the connecting state while the backend works', async () => {
    let finish: (v: unknown) => void = () => {};
    backend({
      vpn_snapshot: () => makeSnapshot({ servers: SERVERS, selected: 's1' }),
      vpn_connect: () => new Promise((resolve) => (finish = resolve)),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText('Серверы / 2');
    await fireEvent.click(screen.getByRole('button', { name: 'Подключить' }));
    const busy = await screen.findByRole('button', { name: 'Подключаем…' });
    expect(busy.getAttribute('aria-busy')).toBe('true');
    finish(makeSnapshot().connection);
    await screen.findByRole('button', { name: 'Подключить' });
  });

  it('keeps TUN unavailable with its reason, system proxy is the working mode', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const mode = screen.getByRole('combobox', { name: 'Режим' }) as HTMLSelectElement;
    expect(mode.value).toBe('proxy');
    expect((mode.querySelector('option[value="tun"]') as HTMLOptionElement).disabled).toBe(true);
    expect(screen.getByText(TUN_MODE_REASON, { exact: false })).toBeTruthy();
  });

  it('shows an honest empty state when there are no servers', () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    expect(screen.getByText('Серверы / 0')).toBeTruthy();
    expect(screen.getByText(/Серверов пока нет/)).toBeTruthy();
  });

  it('selecting a server tells the backend', async () => {
    await openLoaded();
    const second = screen.getByRole('button', { name: /Finland/ });
    await fireEvent.click(second);
    expect(second.getAttribute('aria-pressed')).toBe('true');
    expect(calls('vpn_select')[0]?.[1]).toEqual({ id: 's2' });
  });

  it('measures latency and shows milliseconds or «нет ответа»', async () => {
    const measured = makeSnapshot({
      servers: [
        { ...SERVERS[0], latencyMs: 42, measured: true },
        { ...SERVERS[1], latencyMs: null, measured: true },
      ],
      selected: 's1',
    });
    backend({
      vpn_snapshot: () => makeSnapshot({ servers: SERVERS, selected: 's1' }),
      vpn_measure_latency: () => measured,
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText('Серверы / 2');
    await fireEvent.click(screen.getByRole('button', { name: 'Проверить задержку' }));
    expect(await screen.findByText('42 мс')).toBeTruthy();
    expect(screen.getByText('нет ответа')).toBeTruthy();
  });
});

describe('VpnPage: маршруты и исключения', () => {
  it('shows presets and saves split entries through the backend', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot(),
      vpn_set_rules: (a) => a?.rules,
    });
    render(VpnPage, { props: { subsection: 'routes' } });
    expect(await screen.findByText('Российские сайты напрямую')).toBeTruthy();
    await fireEvent.input(screen.getByLabelText('Домены напрямую'), {
      target: { value: 'example.ru, bank.ru\nvk.com' },
    });
    await fireEvent.input(screen.getByLabelText('Приложения через VPN'), {
      target: { value: 'chrome.exe' },
    });
    await fireEvent.click(screen.getByRole('checkbox', { name: /Российские сайты/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Сохранить' }));
    expect(await screen.findByText(/Сохранено/)).toBeTruthy();
    expect(calls('vpn_set_rules')[0]?.[1]).toEqual({
      rules: {
        directDomains: ['example.ru', 'bank.ru', 'vk.com'],
        proxyDomains: [],
        directApps: [],
        proxyApps: ['chrome.exe'],
        presets: ['ru-direct'],
      },
    });
  });

  it('fills the fields from the saved rules and shows backend validation errors', async () => {
    backend({
      vpn_snapshot: () =>
        makeSnapshot({
          rules: {
            directDomains: ['a.ru'],
            proxyDomains: [],
            directApps: ['steam.exe'],
            proxyApps: [],
            presets: ['ru-direct'],
          },
        }),
      vpn_set_rules: () =>
        Promise.reject({
          kind: 'invalidTarget',
          message: '«http://x»: нужен домен без https:// и пути',
        }),
    });
    render(VpnPage, { props: { subsection: 'routes' } });
    const direct = (await screen.findByLabelText('Домены напрямую')) as HTMLTextAreaElement;
    await waitFor(() => expect(direct.value).toBe('a.ru'));
    expect((screen.getByLabelText('Приложения напрямую') as HTMLTextAreaElement).value).toBe(
      'steam.exe',
    );
    expect(
      (screen.getByRole('checkbox', { name: /Российские сайты/ }) as HTMLInputElement).checked,
    ).toBe(true);
    await fireEvent.click(screen.getByRole('button', { name: 'Сохранить' }));
    expect(await screen.findByText('«http://x»: нужен домен без https:// и пути')).toBeTruthy();
  });

  it('warns that the rules apply on reconnect when connected', async () => {
    backend({
      vpn_snapshot: () =>
        makeSnapshot({
          servers: SERVERS,
          selected: 's1',
          connection: {
            state: 'connected',
            serverId: 's1',
            serverName: 'x',
            since: 1,
            message: 'ok',
            verified: true,
          },
        }),
      vpn_set_rules: (a) => a?.rules,
    });
    render(VpnPage, { props: { subsection: 'routes' } });
    await screen.findByLabelText('Домены напрямую');
    await fireEvent.click(screen.getByRole('button', { name: 'Сохранить' }));
    expect(await screen.findByText(/отключи и подключи VPN/)).toBeTruthy();
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
  it('shows five tabs, the first selected, with the lead line and big title', () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const tabs = screen.getAllByRole('tab').map((tab) => tab.textContent?.trim());
    expect(tabs).toEqual(['VPN', 'Маршруты и исключения', 'Дополнительно', 'Сеть', 'Соединения']);
    expect(screen.getByRole('tab', { name: 'VPN' }).getAttribute('aria-selected')).toBe('true');
    expect(screen.getByText('Подписка, серверы, маршруты · сеть и соединения')).toBeTruthy();
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
