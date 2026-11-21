import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import VpnPage from './VpnPage.svelte';
import * as sound from '../lib/sound';
import { NETWORK_CHANGE_REASON, NO_SERVERS_REASON, TUN_MODE_REASON } from '../lib/vpn';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
  Channel: class {
    onmessage: (event: unknown) => void = () => {};
  },
}));

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
    core: { available: true },
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

describe('VpnPage: постер', () => {
  it('one mouse press plays one animation cycle and one sound (no second one on click)', async () => {
    const heehee = vi.spyOn(sound, 'playHeehee').mockImplementation(() => {});
    render(VpnPage, { props: { subsection: 'vpn' } });
    const poster = screen.getByRole('button', { name: 'Постер' });
    await fireEvent.pointerDown(poster, { button: 0 });
    expect(poster.classList.contains('pressed')).toBe(true);
    // Следом приходит click с detail 1: ничего нового он не запускает.
    await fireEvent.click(poster, { detail: 1 });
    expect(heehee).toHaveBeenCalledTimes(1);
    await fireEvent.animationEnd(poster);
    expect(poster.classList.contains('pressed')).toBe(false);
    heehee.mockRestore();
  });

  it('the keyboard (click with detail 0) also plays exactly once', async () => {
    const heehee = vi.spyOn(sound, 'playHeehee').mockImplementation(() => {});
    render(VpnPage, { props: { subsection: 'vpn' } });
    const poster = screen.getByRole('button', { name: 'Постер' });
    await fireEvent.click(poster, { detail: 0 });
    expect(heehee).toHaveBeenCalledTimes(1);
    heehee.mockRestore();
  });

  it('the wheel over the poster does not scroll the page', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const poster = screen.getByRole('button', { name: 'Постер' });
    const wheel = new WheelEvent('wheel', { deltaY: 120, bubbles: true, cancelable: true });
    poster.dispatchEvent(wheel);
    expect(wheel.defaultPrevented).toBe(true);
  });
});

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
      await screen.findByText(
        'Импорт завершён: серверов 2. Часть записей пропущена (1): wireguard: не поддерживается.',
      ),
    ).toBeTruthy();
    expect(calls('vpn_import')[0]?.[1]).toEqual({ input: 'https://sub.example.com/x?t=1' });
    expect(await screen.findByText('Серверы / 2')).toBeTruthy();
    expect(screen.getByText('Латвия')).toBeTruthy();
    expect(document.querySelector('img.flag')).not.toBeNull();
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
    await screen.findByText('Импорт завершён: серверов 2.');
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
    await screen.findByText('Импорт завершён: серверов 1.');
  });

  it('imports a chosen file as text', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot(),
      vpn_import: () => ({ count: 1, skipped: 0, issues: [] }),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    const file = new File(['trojan://p@h:443#Файл'], 'sub.txt', { type: 'text/plain' });
    await fireEvent.change(screen.getByLabelText('Файл подписки'), { target: { files: [file] } });
    await screen.findByText('Импорт завершён: серверов 1.');
    expect(calls('vpn_import')[0]?.[1]).toEqual({ input: 'trojan://p@h:443#Файл' });
  });

  it('never prefills the subscription field: empty profile and locally saved link both open empty', async () => {
    // Чистая установка: ни подписки, ни серверов.
    render(VpnPage, { props: { subsection: 'vpn' } });
    await waitFor(() => expect(calls('vpn_snapshot').length).toBeGreaterThan(0));
    expect((screen.getByLabelText('Ссылка подписки') as HTMLInputElement).value).toBe('');
    cleanup();
    // Ссылка сохранена локально у пользователя: в поле она не появляется, серверы остаются.
    backend({
      vpn_snapshot: () =>
        makeSnapshot({
          subscriptionUrl: 'https://saved.example/sub',
          servers: SERVERS,
          selected: 's1',
        }),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText(`Серверы / ${SERVERS.length}`);
    expect((screen.getByLabelText('Ссылка подписки') as HTMLInputElement).value).toBe('');
  });

  it('a manually pasted link is imported as typed', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot({ subscriptionUrl: 'https://saved.example/sub' }),
      vpn_import: () => ({ count: 1, skipped: 0, issues: [] }),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    const field = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.input(field, { target: { value: 'https://new.example/sub' } });
    await fireEvent.keyDown(field, { key: 'Enter' });
    await waitFor(() => expect(calls('vpn_import').length).toBe(1));
    expect(calls('vpn_import')[0]?.[1]).toEqual({ input: 'https://new.example/sub' });
  });
});

describe('VpnPage: подключение и серверы', () => {
  it('without servers the connect button is disabled; the reason is a tooltip, not clutter', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const toggle = await screen.findByRole('button', { name: 'Подключить' });
    expect(toggle.getAttribute('title')).toBe(NO_SERVERS_REASON);
    expect(screen.queryByText(NO_SERVERS_REASON)).toBeNull();
    expect(toggle.getAttribute('aria-disabled')).toBe('true');
    await fireEvent.click(toggle);
    expect(calls('vpn_connect')).toHaveLength(0);
    expect(screen.getByLabelText('Время подключения').textContent).toBe('00:00:00');
  });

  it('first connect prepares VPN with progress on the button and shows no technical words', async () => {
    const connected = {
      state: 'connected',
      serverId: 's1',
      serverName: 'Латвия',
      since: Math.floor(Date.now() / 1000),
      message: 'Подключено',
      verified: true,
    };
    let finish: (v: unknown) => void = () => {};
    let progress: (event: unknown) => void = () => {};
    let available = false;
    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'vpn_snapshot')
        return Promise.resolve(
          makeSnapshot({ servers: SERVERS, selected: 's1', core: { available } }),
        );
      if (cmd === 'vpn_connect') {
        progress = (args?.onProgress as { onmessage: (e: unknown) => void }).onmessage;
        return new Promise((resolve) => {
          finish = (v) => {
            available = true;
            resolve(v);
          };
        });
      }
      return Promise.resolve(null);
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText('Серверы / 2');
    expect(document.body.textContent).not.toMatch(/ядр/i);
    const button = screen.getByRole('button', { name: 'Подключить' });
    expect(button.getAttribute('aria-disabled')).toBe('false');
    await fireEvent.click(button);
    expect(await screen.findByRole('button', { name: 'Подготовка VPN…' })).toBeTruthy();
    progress({ percent: 42 });
    expect(await screen.findByRole('button', { name: 'Подготовка VPN… 42%' })).toBeTruthy();
    expect(screen.getByRole('progressbar').getAttribute('aria-valuenow')).toBe('42');
    finish(connected);
    expect(await screen.findByRole('button', { name: 'Отключить' })).toBeTruthy();
    expect(screen.queryByRole('progressbar')).toBeNull();
  });

  it('a failed preparation says to check the internet and offers to retry', async () => {
    backend({
      vpn_snapshot: () =>
        makeSnapshot({ servers: SERVERS, selected: 's1', core: { available: false } }),
      vpn_connect: () =>
        Promise.reject({
          kind: 'prepare',
          message: 'Не удалось подготовить VPN: проверьте интернет',
        }),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText('Серверы / 2');
    await fireEvent.click(screen.getByRole('button', { name: 'Подключить' }));
    expect(await screen.findByText('Не удалось подготовить VPN: проверьте интернет')).toBeTruthy();
    const retry = await screen.findByRole('button', { name: 'Повторить' });
    expect(retry.getAttribute('aria-disabled')).toBe('false');
    expect(document.body.textContent).not.toMatch(/ядр/i);
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
        Promise.reject({ kind: 'probe', message: 'VPN не принял параметры сервера' }),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText('Серверы / 2');
    await fireEvent.click(screen.getByRole('button', { name: 'Подключить' }));
    expect(await screen.findByText('VPN не принял параметры сервера')).toBeTruthy();
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
    const mode = screen.getByRole('button', { name: 'Режим' });
    expect(mode.getAttribute('aria-expanded')).toBe('false');
    await fireEvent.click(mode);
    expect(mode.getAttribute('aria-expanded')).toBe('true');
    const proxy = screen.getByRole('menuitemradio', { name: /Системный прокси/ });
    expect(proxy.getAttribute('aria-checked')).toBe('true');
    const tun = screen.getByRole('menuitemradio', { name: /TUN/ }) as HTMLButtonElement;
    expect(tun.disabled).toBe(true);
    await fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' });
    expect(screen.queryByRole('menu')).toBeNull();
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

  it('has no manual latency button and measures on its own when the list appears', async () => {
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
    expect(screen.queryByRole('button', { name: /задержку/i })).toBeNull();
    expect(await screen.findByText('42 мс', {}, { timeout: 3000 })).toBeTruthy();
    expect(screen.getByText('нет ответа')).toBeTruthy();
  });

  it('repeats about once a minute and pauses when the VPN tab is left', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: false });
    try {
      backend({
        vpn_snapshot: () => makeSnapshot({ servers: SERVERS, selected: 's1' }),
        vpn_measure_latency: () => makeSnapshot({ servers: SERVERS, selected: 's1' }),
      });
      const view = render(VpnPage, { props: { subsection: 'vpn' } });
      await vi.advanceTimersByTimeAsync(1000);
      expect(calls('vpn_measure_latency')).toHaveLength(1);
      await vi.advanceTimersByTimeAsync(30_000);
      expect(calls('vpn_measure_latency')).toHaveLength(1);
      await vi.advanceTimersByTimeAsync(31_000);
      expect(calls('vpn_measure_latency')).toHaveLength(2);
      await view.rerender({ subsection: 'advanced' });
      await vi.advanceTimersByTimeAsync(180_000);
      expect(calls('vpn_measure_latency')).toHaveLength(2);
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('VpnPage: «Дополнительно», блоки «Маршруты» и «Исключения»', () => {
  it('shows the two blocks with plain explanations on the Advanced tab', async () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    expect(await screen.findByRole('heading', { name: 'Маршруты' })).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Исключения' })).toBeTruthy();
    expect(screen.getByText(/Что идёт через VPN/)).toBeTruthy();
    expect(screen.getByText(/Что идёт напрямую, мимо VPN/)).toBeTruthy();
  });

  it('has no separate Routes tab any more', () => {
    render(VpnPage, { props: { subsection: 'routes' } });
    expect(screen.queryByRole('tab', { name: 'Маршруты и исключения' })).toBeNull();
    // Старый id раздела не ломает страницу: открывается первая вкладка.
    expect(screen.getByTestId('vpn-title').textContent).toBe('VPN');
  });

  it('shows presets and saves split entries through the backend', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot(),
      vpn_set_rules: (a) => a?.rules,
    });
    render(VpnPage, { props: { subsection: 'advanced' } });
    expect(await screen.findByText('Российские сайты напрямую')).toBeTruthy();
    await fireEvent.input(screen.getByLabelText('Сайты напрямую'), {
      target: { value: 'example.ru, bank.ru\nvk.com' },
    });
    await fireEvent.input(screen.getByLabelText('Приложения через VPN'), {
      target: { value: 'chrome.exe' },
    });
    await fireEvent.click(screen.getByRole('switch', { name: /Российские сайты/ }));
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
    render(VpnPage, { props: { subsection: 'advanced' } });
    const direct = (await screen.findByLabelText('Сайты напрямую')) as HTMLTextAreaElement;
    await waitFor(() => expect(direct.value).toBe('a.ru'));
    expect((screen.getByLabelText('Приложения напрямую') as HTMLTextAreaElement).value).toBe(
      'steam.exe',
    );
    expect(
      screen.getByRole('switch', { name: /Российские сайты/ }).getAttribute('aria-checked'),
    ).toBe('true');
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
    render(VpnPage, { props: { subsection: 'advanced' } });
    await screen.findByLabelText('Сайты напрямую');
    await fireEvent.click(screen.getByRole('button', { name: 'Сохранить' }));
    expect(await screen.findByText(/отключи и подключи VPN/)).toBeTruthy();
  });
});

describe('VpnPage: маршруты и «Дополнительно»', () => {
  it('shows «Обход блокировок» and «DNS» as one switch each, both off with status text', async () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    expect(screen.getByTestId('vpn-title').textContent).toBe('Дополнительно');
    // Порядок: Обход блокировок, DNS, Маршруты и исключения; имя в кнопке не дублируется.
    await screen.findByText('Маршруты');
    const order = ['Обход блокировок', 'DNS', 'Маршруты'].map(
      (text) => screen.getAllByText(text, { exact: true })[0],
    );
    for (let i = 0; i < order.length - 1; i++) {
      expect(
        order[i].compareDocumentPosition(order[i + 1]) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
    }
    expect(screen.getAllByText('Обход блокировок', { exact: true })).toHaveLength(1);
    expect(screen.getAllByText('DNS', { exact: true })).toHaveLength(1);
    // Telegram не отдельная возможность: только упоминание внутри одной карточки.
    expect(screen.queryByRole('switch', { name: /Telegram/ })).toBeNull();
    expect(screen.queryByText(/Zapret|Xbox/)).toBeNull();
    // Отдельной панели пробной проверки, галочки согласия и оценок больше нет.
    expect(screen.queryByText(/Пробная проверка/)).toBeNull();
    expect(screen.queryByRole('checkbox')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Работает' })).toBeNull();
    for (const name of ['Обход блокировок', 'DNS']) {
      const sw = screen.getByRole('switch', { name });
      expect(sw.getAttribute('aria-checked')).toBe('false');
      expect(sw.closest('.switch-row')?.textContent).toContain('Выключено');
    }
  });

  it('does not show GoodbyeDPI', () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    expect(screen.queryByText(/GoodbyeDPI/i)).toBeNull();
  });

  it('has no confirmation dialog and the tools are disabled with a reason', () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    for (const name of ['Обход блокировок', 'DNS']) {
      expect((screen.getByRole('switch', { name }) as HTMLButtonElement).disabled).toBe(true);
    }
    expect(screen.queryByRole('dialog')).toBeNull();
    // Только DNS: у обхода своя причина по состоянию из Rust.
    expect(screen.getAllByText(/Пока недоступно/)).toHaveLength(1);
  });
});

const IDLE = { active: false, remainingSecs: 0, message: 'Выключено', unavailable: null };
const RUNNING = { active: true, remainingSecs: 65, message: 'Пробный запуск', unavailable: null };

describe('VpnPage: «Обход блокировок» — один переключатель', () => {
  it('does not start anything when the tab opens; start needs a real press with consent=true', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot(),
      desync_trial_status: () => IDLE,
      desync_trial_start: () => RUNNING,
    });
    render(VpnPage, { props: { subsection: 'advanced' } });
    const sw = screen.getByRole('switch', { name: 'Обход блокировок' }) as HTMLButtonElement;
    await waitFor(() => expect(sw.disabled).toBe(false));
    expect(calls('desync_trial_start')).toHaveLength(0);
    expect(sw.getAttribute('aria-checked')).toBe('false');
    expect(screen.getByText(/Временный тестовый режим/)).toBeTruthy();
    await fireEvent.click(sw);
    await waitFor(() => expect(calls('desync_trial_start')[0]?.[1]).toEqual({ consent: true }));
    await waitFor(() => expect(sw.getAttribute('aria-checked')).toBe('true'));
    expect(sw.closest('.switch-row')?.textContent).toContain('Включено · осталось 1:05');
  });

  it('a second press stops it through desync_trial_stop', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot(),
      desync_trial_status: () => RUNNING,
      desync_trial_stop: () => IDLE,
    });
    render(VpnPage, { props: { subsection: 'advanced' } });
    const sw = screen.getByRole('switch', { name: 'Обход блокировок' }) as HTMLButtonElement;
    await waitFor(() => expect(sw.getAttribute('aria-checked')).toBe('true'));
    await fireEvent.click(sw);
    await waitFor(() => expect(calls('desync_trial_stop')).toHaveLength(1));
    expect(calls('desync_trial_start')).toHaveLength(0);
  });

  it('shows «Подготовка…» while starting and never looks on after an error', async () => {
    let release: (v: unknown) => void = () => {};
    backend({
      vpn_snapshot: () => makeSnapshot(),
      desync_trial_status: () => IDLE,
      desync_trial_start: () =>
        new Promise((resolve, reject) => {
          release = () => reject({ kind: 'desync', message: 'Нужны права администратора' });
          void resolve;
        }),
    });
    render(VpnPage, { props: { subsection: 'advanced' } });
    const sw = screen.getByRole('switch', { name: 'Обход блокировок' }) as HTMLButtonElement;
    await waitFor(() => expect(sw.disabled).toBe(false));
    await fireEvent.click(sw);
    await waitFor(() => expect(sw.closest('.switch-row')?.textContent).toContain('Подготовка…'));
    release(null);
    await waitFor(() => expect(sw.closest('.switch-row')?.textContent).toContain('Ошибка'));
    expect(sw.getAttribute('aria-checked')).toBe('false');
    expect(sw.closest('.switch-row')?.textContent).toContain('Нужны права администратора');
  });

  it('has no preset or route chooser: one switch only, everything else is data in the preset', async () => {
    backend({ vpn_snapshot: () => makeSnapshot(), desync_trial_status: () => IDLE });
    render(VpnPage, { props: { subsection: 'advanced' } });
    await screen.findByRole('switch', { name: 'Обход блокировок' });
    expect(screen.getAllByRole('switch').map((el) => el.getAttribute('aria-label'))).toEqual([
      'Обход блокировок',
      'DNS',
    ]);
    expect(screen.queryByRole('combobox')).toBeNull();
    expect(screen.queryByRole('listbox')).toBeNull();
    expect(screen.queryByRole('radio')).toBeNull();
    expect(screen.queryByText(/пресет|маршрут (pass)|V13|Universal/i)).toBeNull();
  });

  it('shows the Russian refusal when VPN is running and keeps the switch off', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot(),
      desync_trial_status: () => IDLE,
      desync_trial_start: () =>
        Promise.reject({
          kind: 'desync',
          message: 'Сначала отключите VPN: пока обход и VPN вместе не работают',
        }),
    });
    render(VpnPage, { props: { subsection: 'advanced' } });
    const sw = screen.getByRole('switch', { name: 'Обход блокировок' }) as HTMLButtonElement;
    await waitFor(() => expect(sw.disabled).toBe(false));
    await fireEvent.click(sw);
    await waitFor(() =>
      expect(sw.closest('.switch-row')?.textContent).toContain('Сначала отключите VPN'),
    );
    expect(sw.getAttribute('aria-checked')).toBe('false');
    expect(calls('desync_trial_start')[0]?.[1]).toEqual({ consent: true });
  });

  it('never shows «Выключено» while the driver is still running; a press retries the stop', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot(),
      desync_trial_status: () => ({
        ...IDLE,
        active: true,
        remainingSecs: 30,
        error: 'Драйвер не остановлен, перезагрузите компьютер',
      }),
      desync_trial_stop: () => IDLE,
    });
    render(VpnPage, { props: { subsection: 'advanced' } });
    const sw = screen.getByRole('switch', { name: 'Обход блокировок' }) as HTMLButtonElement;
    const row = () => sw.closest('.switch-row')?.textContent ?? '';
    await waitFor(() => expect(row()).toContain('Драйвер не остановлен, перезагрузите компьютер'));
    expect(row()).toContain('Ошибка');
    expect(row()).not.toContain('Выключено');
    await fireEvent.click(sw);
    await waitFor(() => expect(calls('desync_trial_stop')).toHaveLength(1));
    expect(calls('desync_trial_start')).toHaveLength(0);
  });

  it('is disabled with a visible reason when Rust says the trial is unavailable', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot(),
      desync_trial_status: () => ({ ...IDLE, unavailable: 'Нужны права администратора' }),
    });
    render(VpnPage, { props: { subsection: 'advanced' } });
    const sw = screen.getByRole('switch', { name: 'Обход блокировок' }) as HTMLButtonElement;
    await screen.findByText('Нужны права администратора');
    expect(sw.disabled).toBe(true);
    expect(sw.getAttribute('aria-checked')).toBe('false');
  });
});

describe('VpnPage: DNS — выбор провайдера', () => {
  it('opens the brand menu with Automatic checked and providers shown as not yet available', async () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Выбрать DNS-провайдера' }));
    const auto = screen.getByRole('menuitemradio', { name: 'Автоматически' });
    expect(auto.getAttribute('aria-checked')).toBe('true');
    for (const name of ['Cloudflare', 'Google', 'OpenDNS', 'Quad9']) {
      const item = screen.getByRole('menuitemradio', { name }) as HTMLButtonElement;
      expect(item.disabled).toBe(true);
      expect(item.getAttribute('aria-checked')).toBe('false');
    }
    await fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' });
    expect(screen.queryByRole('menu')).toBeNull();
    expect(
      (screen.getByRole('switch', { name: 'DNS' }) as HTMLButtonElement).getAttribute(
        'aria-checked',
      ),
    ).toBe('false');
  });
});

describe('VpnPage: вкладки', () => {
  it('shows four tabs, the first selected, with the lead line and big title', () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const tabs = screen.getAllByRole('tab').map((tab) => tab.textContent?.trim());
    expect(tabs).toEqual(['VPN', 'Дополнительно', 'Сеть', 'Соединения']);
    expect(screen.getByRole('tab', { name: 'VPN' }).getAttribute('aria-selected')).toBe('true');
    expect(screen.getByText('Подписка, серверы, маршруты · сеть и соединения')).toBeTruthy();
    expect(screen.getByTestId('vpn-title').textContent).toBe('VPN');
  });

  it('clicking a tab switches the content and the title', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    await fireEvent.click(screen.getByRole('tab', { name: 'Дополнительно' }));
    expect(screen.getByRole('switch', { name: 'DNS' })).toBeTruthy();
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
