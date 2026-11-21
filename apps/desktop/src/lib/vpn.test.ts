import { describe, expect, it } from 'vitest';
import {
  NO_SERVERS_REASON,
  connectUi,
  displayName,
  elapsedSince,
  formatElapsed,
  formatLatency,
  importMessage,
  joinEntries,
  normalizePasted,
  prepareImport,
  requestNetworkChange,
  NETWORK_CHANGE_REASON,
  TOOL_UNAVAILABLE,
  applyToolResult,
  toolStateText,
  toolSwitchProps,
  type ToolModel,
  splitEntries,
  statusLine,
  validateSubscriptionUrl,
} from './vpn';
import type { VpnSnapshot } from './vpnApi';

function snapshot(
  over: Partial<VpnSnapshot> = {},
  state: VpnSnapshot['connection']['state'] = 'disconnected',
): VpnSnapshot {
  return {
    servers: [
      {
        id: 'a',
        name: 'A',
        protocol: 'vless',
        host: 'h',
        port: 443,
        country: null,
        latencyMs: null,
        measured: false,
      },
    ],
    selected: 'a',
    subscriptionUrl: '',
    rules: { directDomains: [], proxyDomains: [], directApps: [], proxyApps: [], presets: [] },
    presets: [],
    core: { available: true },
    connection: {
      state,
      serverId: null,
      serverName: null,
      since: null,
      message: 'Не подключено',
      verified: false,
    },
    ...over,
  };
}

describe('validateSubscriptionUrl', () => {
  it('accepts an https link and returns it trimmed', () => {
    expect(validateSubscriptionUrl('  https://example.com/sub?token=1  ')).toEqual({
      ok: true,
      url: 'https://example.com/sub?token=1',
    });
  });

  it('rejects http and other schemes with the https-only reason', () => {
    for (const bad of ['http://example.com', 'ftp://example.com/x', 'vless://a@h:1']) {
      expect(validateSubscriptionUrl(bad)).toEqual({
        ok: false,
        reason: 'Разрешены только ссылки https://',
      });
    }
  });

  it('rejects empty, whitespace-only, spaced and non-URL input', () => {
    expect(validateSubscriptionUrl('')).toEqual({ ok: false, reason: 'Вставь ссылку подписки' });
    expect(validateSubscriptionUrl('   ')).toEqual({ ok: false, reason: 'Вставь ссылку подписки' });
    expect(validateSubscriptionUrl('https://a b.com').ok).toBe(false);
    expect(validateSubscriptionUrl('justtext')).toEqual({
      ok: false,
      reason: 'Некорректная ссылка',
    });
  });
});

describe('prepareImport', () => {
  it('passes https links and pasted server lists through', () => {
    expect(prepareImport(' https://s.example/sub ')).toEqual({
      ok: true,
      input: 'https://s.example/sub',
    });
    const list = 'vless://a@h:1#A\ntrojan://b@h:2#B';
    expect(prepareImport(list)).toEqual({ ok: true, input: list });
  });

  it('refuses http:// before anything is sent', () => {
    expect(prepareImport('http://s.example/sub')).toEqual({
      ok: false,
      reason: 'Разрешены только ссылки https://',
    });
  });

  it('refuses empty input', () => {
    expect(prepareImport('  ').ok).toBe(false);
  });

  it('normalizePasted splits links glued by spaces into lines', () => {
    expect(normalizePasted('vless://a@h:1#A trojan://b@h:2#B  ss://x@h:3#C')).toBe(
      'vless://a@h:1#A\ntrojan://b@h:2#B\nss://x@h:3#C',
    );
    expect(normalizePasted('https://s.example/sub')).toBe('https://s.example/sub');
  });
});

describe('connectUi', () => {
  it('is disabled with a reason until a server is imported', () => {
    expect(connectUi(null, false)).toMatchObject({ disabled: true, reason: NO_SERVERS_REASON });
    expect(connectUi(snapshot({ servers: [], selected: null }), false)).toMatchObject({
      disabled: true,
      reason: NO_SERVERS_REASON,
    });
  });

  it('stays available when the files are missing: the app fetches them on connect', () => {
    const ui = connectUi(snapshot({ core: { available: false } }), false);
    expect(ui).toMatchObject({ disabled: false, label: 'Подключить', reason: null });
  });

  it('shows the preparation percent on the button and the retry label after a failure', () => {
    expect(connectUi(snapshot(), true, { preparing: 0 })).toMatchObject({
      label: 'Подготовка VPN…',
      busy: true,
      disabled: true,
    });
    expect(connectUi(snapshot(), true, { preparing: 42 }).label).toBe('Подготовка VPN… 42%');
    expect(connectUi(snapshot(), false, { retry: true })).toMatchObject({
      label: 'Повторить',
      disabled: false,
    });
  });

  it('is ready to connect when a server is selected and the core is present', () => {
    expect(connectUi(snapshot(), false)).toEqual({
      label: 'Подключить',
      pressed: false,
      busy: false,
      disabled: false,
      reason: null,
    });
  });

  it('shows a busy, disabled button while connecting or while a request is running', () => {
    expect(connectUi(snapshot({}, 'connecting'), false)).toMatchObject({
      busy: true,
      disabled: true,
      label: 'Подключаем…',
    });
    expect(connectUi(snapshot(), true)).toMatchObject({ busy: true, disabled: true });
  });

  it('offers to disconnect when connected', () => {
    expect(connectUi(snapshot({}, 'connected'), false)).toMatchObject({
      label: 'Отключить',
      pressed: true,
      disabled: false,
    });
  });

  it('after an error the button can try again', () => {
    expect(connectUi(snapshot({}, 'error'), false).disabled).toBe(false);
  });
});

describe('statusLine', () => {
  it('uses the hint only while disconnected', () => {
    const idle = snapshot().connection;
    expect(statusLine(idle, 'Сначала выбери сервер')).toBe('Сначала выбери сервер');
    expect(statusLine(idle, null)).toBe('Не подключено');
    expect(statusLine({ ...idle, state: 'connected', message: 'Подключено · A' }, 'x')).toBe(
      'Подключено · A',
    );
    expect(statusLine(null, null)).toBe('Не подключено');
  });
});

describe('formatLatency', () => {
  it('distinguishes not measured, no answer and a value', () => {
    expect(formatLatency({ measured: false, latencyMs: null })).toBe('');
    expect(formatLatency({ measured: true, latencyMs: null })).toBe('нет ответа');
    expect(formatLatency({ measured: true, latencyMs: 42 })).toBe('42 мс');
  });
});

describe('elapsedSince / formatElapsed', () => {
  it('counts from the Unix-second start and never goes negative', () => {
    expect(elapsedSince(1000, 1_754_000)).toBe(754);
    expect(elapsedSince(2000, 1_000_000)).toBe(0);
    expect(elapsedSince(null, 5)).toBe(0);
  });

  it('formats seconds as HH:MM:SS', () => {
    expect(formatElapsed(0)).toBe('00:00:00');
    expect(formatElapsed(754)).toBe('00:12:34');
    expect(formatElapsed(3661)).toBe('01:01:01');
  });

  it('keeps counting past 99 hours and clamps invalid input to zero', () => {
    expect(formatElapsed(100 * 3600)).toBe('100:00:00');
    expect(formatElapsed(-5)).toBe('00:00:00');
    expect(formatElapsed(Number.NaN)).toBe('00:00:00');
  });
});

describe('names and route entries', () => {
  it('displayName strips a leading flag emoji only', () => {
    expect(displayName('🇱🇻 Латвия')).toBe('Латвия');
    expect(displayName('Latvia')).toBe('Latvia');
  });

  it('splitEntries accepts lines, commas and semicolons and drops blanks', () => {
    expect(splitEntries('a.ru, b.ru\n\nc.ru;  ')).toEqual(['a.ru', 'b.ru', 'c.ru']);
    expect(joinEntries(['a', 'b'])).toBe('a\nb');
  });

  it('importMessage reports skipped records', () => {
    expect(importMessage(3, 0, [])).toBe('Импорт завершён: серверов 3.');
    expect(importMessage(3, 2, ['a', 'b'])).toBe(
      'Импорт завершён: серверов 3. Часть записей пропущена (2): a; b.',
    );
  });
});

describe('stubs', () => {
  it('a network change returns the not-connected reason', () => {
    expect(requestNetworkChange()).toEqual({
      ok: false,
      reason: NETWORK_CHANGE_REASON,
    });
  });
});

describe('tool switches (Zapret, Xbox DNS)', () => {
  it('an unimplemented tool stays off and disabled with a reason, never on', () => {
    expect(TOOL_UNAVAILABLE.reason).toMatch(/Пока недоступно/);
    expect(toolSwitchProps(TOOL_UNAVAILABLE)).toEqual({
      checked: false,
      pending: false,
      error: false,
      disabled: true,
    });
    // Даже ошибочно выставленный статус «вкл» у нереализованного инструмента не показывает успех.
    expect(toolSwitchProps({ ...TOOL_UNAVAILABLE, status: 'on' }).checked).toBe(false);
  });

  it('maps off, on, pending and error for an available tool', () => {
    const base = { available: true, reason: null };
    expect(toolSwitchProps({ ...base, status: 'off' })).toMatchObject({ checked: false });
    expect(toolSwitchProps({ ...base, status: 'on' })).toMatchObject({ checked: true });
    expect(toolSwitchProps({ ...base, status: 'pending' })).toMatchObject({
      pending: true,
      disabled: true,
    });
    expect(toolSwitchProps({ ...base, status: 'error' })).toMatchObject({
      error: true,
      checked: false,
    });
  });

  it('success shows on only after a confirmed result, failure rolls back with a phrase', () => {
    const tool: ToolModel = { status: 'pending', available: true, reason: null };
    expect(applyToolResult(tool, { ok: true }, true).status).toBe('on');
    expect(applyToolResult(tool, { ok: true }, false).status).toBe('off');
    const failed = applyToolResult(tool, { ok: false, reason: 'Не вышло' }, true);
    expect(failed).toMatchObject({ status: 'error', reason: 'Не вышло' });
    expect(toolSwitchProps(failed).checked).toBe(false);
  });
});

describe('toolStateText', () => {
  it('shows only confirmed states', () => {
    expect(toolStateText(TOOL_UNAVAILABLE)).toBe('Выключено · недоступно');
    const base = { ...TOOL_UNAVAILABLE, available: true, reason: null };
    expect(toolStateText({ ...base, status: 'off' })).toBe('Выключено');
    expect(toolStateText({ ...base, status: 'on' })).toBe('Включено');
    expect(toolStateText({ ...base, status: 'pending' })).toBe('Подождите…');
    expect(toolStateText({ ...base, status: 'error' })).toBe('Ошибка');
  });
});
