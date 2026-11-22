import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import VpnPage from './VpnPage.svelte';
import * as sound from '../lib/sound';
import { NETWORK_CHANGE_REASON, NO_SERVERS_REASON } from '../lib/vpn';

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
    rules: {
      directDomains: [],
      proxyDomains: [],
      directApps: [],
      proxyApps: [],
      presets: [],
      mode: 'all',
    },
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
  it('shows the subscription field with «Из файла…» in place of the old import button', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    expect(screen.getByText('Ссылка подписки / импорта')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Импорт' })).toBeNull();
    const fileButton = screen.getByRole('button', { name: 'Из файла…' }) as HTMLButtonElement;
    expect(fileButton.disabled).toBe(false);
    await waitFor(() => expect(calls('vpn_snapshot').length).toBeGreaterThan(0));
  });

  it('rejects an http link without calling the backend', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const input = screen.getByLabelText('Ссылка подписки') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'http://example.com/sub' } });
    expect(await screen.findByText('Разрешены только ссылки https://')).toBeTruthy();
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
    // Список ссылок серверов (не https-подписка) импортируется по Enter.
    expect(calls('vpn_import')).toHaveLength(0);
    await fireEvent.keyDown(screen.getByLabelText('Ссылка подписки'), { key: 'Enter' });
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
    expect(await screen.findByText('Сервис подписки ответил HTTP 403')).toBeTruthy();
    expect((screen.getByRole('button', { name: 'Из файла…' }) as HTMLButtonElement).disabled).toBe(
      false,
    );
  });

  it('shows a loading state while the auto-import runs', async () => {
    let finish: (v: unknown) => void = () => {};
    backend({
      vpn_snapshot: () => makeSnapshot(),
      vpn_import: () => new Promise((resolve) => (finish = resolve)),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await fireEvent.input(screen.getByLabelText('Ссылка подписки'), {
      target: { value: 'https://sub.example.com/x' },
    });
    expect((await screen.findAllByText('Загружаем подписку…')).length).toBeGreaterThan(0);
    expect((screen.getByRole('button', { name: 'Из файла…' }) as HTMLButtonElement).disabled).toBe(
      true,
    );
    finish({ count: 1, skipped: 0, issues: [] });
    await screen.findByText('Импорт завершён: серверов 1.');
  });

  it('imports a valid https link by itself once, and typing it letter by letter imports nothing', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot(),
      vpn_import: () => ({ count: 1, skipped: 0, issues: [] }),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    const input = screen.getByLabelText('Ссылка подписки');
    const typed = 'https://sub.example.com';
    for (let i = 1; i <= typed.length; i++) {
      await fireEvent.input(input, { target: { value: typed.slice(0, i) } });
    }
    expect(calls('vpn_import')).toHaveLength(0);
    // Вставка целиком: импорт один раз.
    await fireEvent.input(input, { target: { value: '' } });
    await fireEvent.input(input, { target: { value: 'https://sub.example.com/token' } });
    await screen.findByText('Импорт завершён: серверов 1.');
    expect(calls('vpn_import')).toHaveLength(1);
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
    // Enter после автоимпорта той же ссылки второй загрузки не делает.
    expect(calls('vpn_import')).toHaveLength(1);
    expect(calls('vpn_import')[0]?.[1]).toEqual({ input: 'https://new.example/sub' });
  });
});

describe('VpnPage: подключение и серверы', () => {
  it('without servers the connect button is disabled; the reason is a tooltip, not clutter', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const toggle = await screen.findByRole('button', { name: 'Включить VPN' });
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
    const button = screen.getByRole('button', { name: 'Включить VPN' });
    expect(button.getAttribute('aria-disabled')).toBe('false');
    await fireEvent.click(button);
    expect(await screen.findByRole('button', { name: 'Подготовка VPN…' })).toBeTruthy();
    progress({ percent: 42 });
    expect(await screen.findByRole('button', { name: 'Подготовка VPN… 42%' })).toBeTruthy();
    expect(screen.getByRole('progressbar').getAttribute('aria-valuenow')).toBe('42');
    finish(connected);
    expect(await screen.findByRole('button', { name: 'Выключить VPN' })).toBeTruthy();
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
    await fireEvent.click(screen.getByRole('button', { name: 'Включить VPN' }));
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
    await fireEvent.click(screen.getByRole('button', { name: 'Включить VPN' }));
    const off = await screen.findByRole('button', { name: 'Выключить VPN' });
    expect(off.getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByText('Подключено · Латвия · передача данных проверена')).toBeTruthy();
    expect(screen.getByLabelText('Время подключения').textContent).toMatch(/^00:01:0\d$/);
    await fireEvent.click(off);
    const on = await screen.findByRole('button', { name: 'Включить VPN' });
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
    await fireEvent.click(screen.getByRole('button', { name: 'Включить VPN' }));
    expect(await screen.findByText('VPN не принял параметры сервера')).toBeTruthy();
    const again = await screen.findByRole('button', { name: 'Включить VPN' });
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
    await fireEvent.click(screen.getByRole('button', { name: 'Включить VPN' }));
    // Переключатель меняется в тот же кадр: сразу «включён», пока ядро запускается.
    const busy = await screen.findByRole('button', { name: 'Включаем…' });
    expect(busy.getAttribute('aria-busy')).toBe('true');
    expect(busy.getAttribute('aria-pressed')).toBe('true');
    finish(makeSnapshot().connection);
    await screen.findByRole('button', { name: 'Включить VPN' });
  });

  it('has no choice of connection method and no technical words on the VPN screen', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByRole('radiogroup', { name: 'Область VPN' });
    expect(screen.queryByRole('button', { name: 'Режим' })).toBeNull();
    expect(screen.queryByRole('menu')).toBeNull();
    const text = document.body.textContent ?? '';
    for (const word of [
      /TUN/,
      /системн\w* прокси/i,
      /драйвер/i,
      /движок/i,
      /\bSHA\b/,
      /Напрямую/,
    ]) {
      expect(text).not.toMatch(word);
    }
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

  it('has no manual latency button and measures once right after an import', async () => {
    const measured = makeSnapshot({
      servers: [
        { ...SERVERS[0], latencyMs: 42, measured: true },
        { ...SERVERS[1], latencyMs: null, measured: true },
      ],
      selected: 's1',
    });
    backend({
      vpn_snapshot: () => makeSnapshot({ servers: SERVERS, selected: 's1' }),
      vpn_import: () => ({ count: 2, skipped: 0, issues: [] }),
      vpn_measure_latency: () => measured,
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText('Серверы / 2');
    expect(screen.queryByRole('button', { name: /задержку/i })).toBeNull();
    // Сам по себе, при открытии, замера нет.
    expect(calls('vpn_measure_latency')).toHaveLength(0);
    await fireEvent.input(screen.getByLabelText('Ссылка подписки'), {
      target: { value: 'https://sub.example.com/x?t=1' },
    });
    expect(await screen.findByText('42 мс', {}, { timeout: 3000 })).toBeTruthy();
    expect(screen.getByText('нет ответа')).toBeTruthy();
    expect(calls('vpn_measure_latency')).toHaveLength(1);
  });
});

describe('VpnPage: без фоновой работы, пока VPN выключен', () => {
  it('makes no requests and keeps no timers while the VPN is off, even after minutes', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: false });
    try {
      backend({ vpn_snapshot: () => makeSnapshot({ servers: SERVERS, selected: 's1' }) });
      render(VpnPage, { props: { subsection: 'vpn' } });
      await vi.advanceTimersByTimeAsync(500);
      const calm = calls('vpn_status').length + calls('vpn_measure_latency').length;
      await vi.advanceTimersByTimeAsync(300_000);
      expect(calls('vpn_status')).toHaveLength(0);
      expect(calls('vpn_measure_latency')).toHaveLength(0);
      expect(calm).toBe(0);
    } finally {
      vi.useRealTimers();
    }
  });

  it('checks the core only while connected and stops completely after turning off', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: false });
    try {
      const connected = {
        state: 'connected',
        serverId: 's1',
        serverName: 'A',
        since: 1,
        message: 'Подключено',
        verified: true,
      };
      const off = { state: 'disconnected', message: 'Не подключено', since: null };
      let current: Record<string, unknown> = connected;
      backend({
        vpn_snapshot: () => makeSnapshot({ servers: SERVERS, selected: 's1', connection: current }),
        vpn_status: () => current,
        vpn_disconnect: () => {
          current = off;
          return off;
        },
      });
      render(VpnPage, { props: { subsection: 'vpn' } });
      await vi.advanceTimersByTimeAsync(7000);
      expect(calls('vpn_status').length).toBeGreaterThanOrEqual(2);
      await fireEvent.click(screen.getByRole('button', { name: 'Выключить VPN' }));
      await vi.advanceTimersByTimeAsync(100);
      const after = calls('vpn_status').length;
      await vi.advanceTimersByTimeAsync(120_000);
      expect(calls('vpn_status')).toHaveLength(after);
      expect(calls('vpn_measure_latency')).toHaveLength(0);
    } finally {
      vi.useRealTimers();
    }
  });

  it('a press does not wait for the core: the switch flips while the request is still pending', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot({ servers: SERVERS, selected: 's1' }),
      vpn_connect: () => new Promise(() => {}),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText('Серверы / 2');
    await fireEvent.click(screen.getByRole('button', { name: 'Включить VPN' }));
    const flipped = await screen.findByRole('button', { name: 'Включаем…' });
    expect(flipped.getAttribute('aria-pressed')).toBe('true');
    // Страница при этом отзывчива: область выбирается, пока ядро ещё запускается.
    await fireEvent.click(screen.getByRole('radio', { name: 'Весь компьютер' }));
    expect(screen.getByRole('radio', { name: 'Весь компьютер' }).getAttribute('aria-checked')).toBe(
      'true',
    );
  });
});

describe('VpnPage: «Дополнительно», блоки «Через VPN» и «Исключения»', () => {
  it('shows the two blocks with plain explanations on the Advanced tab', async () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    expect(await screen.findByRole('heading', { name: 'Через VPN' })).toBeTruthy();
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
        mode: 'all',
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

describe('VpnPage: «Дополнительно»', () => {
  it('has has no technical words', async () => {
    render(VpnPage, { props: { subsection: 'advanced' } });
    expect(screen.getByTestId('vpn-title').textContent).toBe('Дополнительно');
    await screen.findByText('Через VPN');
    expect(screen.queryByText(/DNS|Маршрут|маршрут/)).toBeNull();
    expect(screen.queryByText(/Zapret|Xbox|GoodbyeDPI/i)).toBeNull();
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.queryByRole('checkbox')).toBeNull();
  });
});

describe('VpnPage: список сайтов', () => {
  it('has a side button that opens and closes the sites panel, same button again closes', async () => {
    backend({
      vpn_snapshot: () => makeSnapshot(),
      bypass_sites_list: () => ['example.com'],
    });
    const { container } = render(VpnPage, { props: { subsection: 'vpn' } });
    const side = screen.getByRole('button', { name: 'Список сайтов' });
    const panel = () => container.querySelector('.sites') as HTMLElement;
    expect(panel().getAttribute('aria-hidden')).toBe('true');
    expect(side.getAttribute('aria-expanded')).toBe('false');
    await fireEvent.click(side);
    expect(panel().getAttribute('aria-hidden')).toBe('false');
    expect(side.getAttribute('aria-expanded')).toBe('true');
    expect(await within(panel()).findByText('example.com')).toBeTruthy();
    await fireEvent.click(side);
    expect(panel().getAttribute('aria-hidden')).toBe('true');
    await fireEvent.click(side);
    await fireEvent.click(screen.getByRole('button', { name: 'Закрыть' }));
    expect(panel().getAttribute('aria-hidden')).toBe('true');
  });
});

describe('VpnPage: вкладки', () => {
  it('shows four tabs, the first selected, with the lead line and big title', () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    const tabs = screen.getAllByRole('tab').map((tab) => tab.textContent?.trim());
    expect(tabs).toEqual(['VPN', 'Дополнительно', 'Сеть', 'Соединения']);
    expect(screen.getByRole('tab', { name: 'VPN' }).getAttribute('aria-selected')).toBe('true');
    expect(screen.getByText('Подписка, серверы и сайты · сеть и соединения')).toBeTruthy();
    expect(screen.getByTestId('vpn-title').textContent).toBe('VPN');
  });

  it('clicking a tab switches the content and the title', async () => {
    render(VpnPage, { props: { subsection: 'vpn' } });
    await fireEvent.click(screen.getByRole('tab', { name: 'Дополнительно' }));
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

describe('VpnPage: области VPN', () => {
  const withMode = (mode: string) =>
    makeSnapshot({
      servers: SERVERS,
      selected: 's1',
      rules: {
        directDomains: [],
        proxyDomains: [],
        directApps: [],
        proxyApps: [],
        presets: [],
        mode,
      },
    } as never);

  it('shows two areas, highlights the chosen one and lets VPN be turned on in either', async () => {
    backend({ vpn_snapshot: () => withMode('listOnly') });
    render(VpnPage, { props: { subsection: 'vpn' } });
    const group = await screen.findByRole('radiogroup', { name: 'Область VPN' });
    const radios = within(group).getAllByRole('radio');
    expect(radios.map((r) => r.getAttribute('aria-label'))).toEqual([
      'Весь компьютер',
      'Только сайты',
    ]);
    // Одна выбрана.
    expect(radios.filter((r) => r.getAttribute('aria-checked') === 'true')).toHaveLength(1);
    expect(screen.getByRole('radio', { name: 'Только сайты' }).getAttribute('aria-checked')).toBe(
      'true',
    );
    expect(screen.getByText('Российские сервисы работают как обычно')).toBeTruthy();
    // Включить VPN можно в любой области.
    await screen.findByText('Серверы / 2');
    const power = screen.getByRole('button', { name: 'Включить VPN' });
    expect(power.getAttribute('aria-disabled')).toBe('false');
    await fireEvent.click(screen.getByRole('radio', { name: 'Весь компьютер' }));
    expect(screen.getByRole('button', { name: 'Включить VPN' }).getAttribute('aria-disabled')).toBe(
      'false',
    );
  });

  it('picks an area at once and sends it; pressing the chosen one again changes nothing', async () => {
    let mode = 'listOnly';
    backend({
      vpn_snapshot: () => withMode(mode),
      vpn_set_mode: (args: { mode: string }) => {
        mode = args.mode;
        return { state: 'disconnected', message: 'Не подключено', since: null };
      },
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText('Серверы / 2');
    const all = screen.getByRole('radio', { name: 'Весь компьютер' });
    await fireEvent.click(all);
    expect(all.getAttribute('aria-checked')).toBe('true');
    expect(screen.getByRole('radio', { name: 'Только сайты' }).getAttribute('aria-checked')).toBe(
      'false',
    );
    await waitFor(() => expect(calls('vpn_set_mode')[0]?.[1]).toEqual({ mode: 'all' }));
    await fireEvent.click(all);
    expect(calls('vpn_set_mode')).toHaveLength(1);
  });

  it('shows «Переключаем…» at once while the area changes and keeps the switch on', async () => {
    let mode = 'listOnly';
    let release: () => void = () => {};
    const gate = new Promise<void>((resolve) => (release = resolve));
    const on = {
      state: 'connected',
      serverId: 's1',
      serverName: 'Server 1',
      since: 1000,
      message: 'Подключено · Server 1',
      verified: true,
    };
    backend({
      vpn_snapshot: () => ({ ...withMode(mode), connection: on }) as never,
      vpn_set_mode: async (args: { mode: string }) => {
        await gate;
        mode = args.mode;
        return on;
      },
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText('Серверы / 2');
    await fireEvent.click(screen.getByRole('radio', { name: 'Весь компьютер' }));
    expect(await screen.findByText('Переключаем…')).toBeTruthy();
    const power = screen.getByRole('button', { name: 'Переключаем…' });
    expect(power.getAttribute('aria-pressed')).toBe('true');
    release();
    await waitFor(() => expect(screen.queryByText('Переключаем…')).toBeNull());
    expect(screen.getByRole('button', { name: 'Выключить VPN' })).toBeTruthy();
  });

  it('goes back to the previous area and says why when it could not be set', async () => {
    backend({
      vpn_snapshot: () => withMode('listOnly'),
      vpn_set_mode: () => Promise.reject({ kind: 'probe', message: 'Не удалось сохранить' }),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText('Серверы / 2');
    await fireEvent.click(screen.getByRole('radio', { name: 'Весь компьютер' }));
    expect(await screen.findByText('Не удалось сохранить')).toBeTruthy();
    await waitFor(() =>
      expect(screen.getByRole('radio', { name: 'Только сайты' }).getAttribute('aria-checked')).toBe(
        'true',
      ),
    );
  });

  it('turns the VPN switch off at once and shows a short reason when turning on fails', async () => {
    backend({
      vpn_snapshot: () => withMode('all'),
      vpn_connect: () =>
        Promise.reject({
          kind: 'needsReinstall',
          message: 'Для этого режима нужна переустановка программы',
        }),
    });
    render(VpnPage, { props: { subsection: 'vpn' } });
    await screen.findByText('Серверы / 2');
    await fireEvent.click(screen.getByRole('button', { name: 'Включить VPN' }));
    expect(await screen.findByText('Для этого режима нужна переустановка программы')).toBeTruthy();
    const again = await screen.findByRole('button', { name: 'Включить VPN' });
    expect(again.getAttribute('aria-pressed')).toBe('false');
  });
});
