import { describe, expect, it } from 'vitest';
import {
  NETWORK_CHANGE_REASON,
  VPN_CONNECT_REASON,
  decodeBase64Utf8,
  decodeSubscriptionBody,
  formatElapsed,
  parseSubscription,
  requestNetworkChange,
  requestVpnConnect,
  validateSubscriptionUrl,
  vmessName,
} from './vpn';

/** base64 в UTF-8 без зависимости от Buffer (в браузере Buffer нет). */
function b64(text: string): string {
  const bytes = new TextEncoder().encode(text);
  return btoa(String.fromCharCode(...bytes));
}

const VMESS_JSON = JSON.stringify({ v: '2', ps: 'Сервер «Берлин»', add: 'h', port: '443' });

describe('validateSubscriptionUrl', () => {
  it('accepts an https link and returns it trimmed', () => {
    expect(validateSubscriptionUrl('  https://example.com/sub?token=1  ')).toEqual({
      ok: true,
      url: 'https://example.com/sub?token=1',
    });
  });

  it.each([
    ['http://example.com/sub', 'http'],
    ['file:///C:/subs/list.txt', 'file'],
    ['ftp://example.com/sub', 'ftp'],
    ['javascript:alert(1)', 'javascript'],
  ])('rejects %s (%s scheme) with the https reason', (input) => {
    expect(validateSubscriptionUrl(input)).toEqual({
      ok: false,
      reason: 'Разрешены только ссылки https://',
    });
  });

  it('rejects an empty string', () => {
    expect(validateSubscriptionUrl('')).toEqual({ ok: false, reason: 'Вставь ссылку подписки' });
  });

  it('rejects a whitespace-only string as empty', () => {
    expect(validateSubscriptionUrl('   \t ')).toEqual({
      ok: false,
      reason: 'Вставь ссылку подписки',
    });
  });

  it('rejects a link with spaces inside', () => {
    expect(validateSubscriptionUrl('https://exa mple.com/sub')).toEqual({
      ok: false,
      reason: 'Ссылка не должна содержать пробелов',
    });
  });

  it('rejects text that is not a URL', () => {
    expect(validateSubscriptionUrl('not a link')).toEqual({
      ok: false,
      reason: 'Ссылка не должна содержать пробелов',
    });
    expect(validateSubscriptionUrl('https://')).toEqual({
      ok: false,
      reason: 'Некорректная ссылка',
    });
  });
});

const SAMPLE = [
  'vless://uuid@1.2.3.4:443?security=tls#Germany%20-%20Fast',
  'vmess://eyJhZGQiOiIxLjIuMy40In0=#Netherlands',
  'trojan://pass@5.6.7.8:443#Finland',
  'ss://YWVzLTI1Ni1nY206cGFzcw@9.9.9.9:8388#Japan',
  '',
  'Some banner text from the provider',
  'https://example.com/not-a-server',
  'vless://uuid@10.0.0.1:443',
].join('\n');

describe('parseSubscription', () => {
  it('lists recognised servers with scheme and name from the fragment', () => {
    const result = parseSubscription(SAMPLE);
    expect(result.servers).toEqual([
      { scheme: 'vless', name: 'Germany - Fast' },
      { scheme: 'vmess', name: 'Netherlands' },
      { scheme: 'trojan', name: 'Finland' },
      { scheme: 'ss', name: 'Japan' },
      { scheme: 'vless', name: 'Без названия' },
    ]);
  });

  it('counts unknown lines instead of listing them, skipping blank lines', () => {
    expect(parseSubscription(SAMPLE).unknownCount).toBe(2);
  });

  it('handles Windows line endings and surrounding spaces', () => {
    const result = parseSubscription('  trojan://a@h:1#One  \r\nvless://b@h:2#Two\r\n');
    expect(result.servers.map((server) => server.name)).toEqual(['One', 'Two']);
    expect(result.unknownCount).toBe(0);
  });

  it('keeps the raw fragment when it is not valid percent-encoding', () => {
    const result = parseSubscription('trojan://a@h:1#100%');
    expect(result.servers).toEqual([{ scheme: 'trojan', name: '100%' }]);
  });

  it('returns no servers for empty text', () => {
    expect(parseSubscription('')).toEqual({ servers: [], unknownCount: 0 });
  });
});

describe('decodeBase64Utf8', () => {
  it('decodes standard and url-safe base64, with or without padding', () => {
    expect(decodeBase64Utf8(b64('hello'))).toBe('hello');
    expect(decodeBase64Utf8(b64('hello').replace(/=+$/, ''))).toBe('hello');
    expect(decodeBase64Utf8(b64('??>>').replace(/\+/g, '-').replace(/\//g, '_'))).toBe('??>>');
  });

  it('returns null for input that is not base64 or not UTF-8', () => {
    expect(decodeBase64Utf8('')).toBeNull();
    expect(decodeBase64Utf8('not base64!')).toBeNull();
    expect(decodeBase64Utf8('a')).toBeNull();
    expect(decodeBase64Utf8(btoa(String.fromCharCode(0xff, 0xfe)))).toBeNull();
  });
});

describe('decodeSubscriptionBody', () => {
  const links = 'vless://id@host:443#One\ntrojan://pw@host:443#Two';

  it('decodes a base64 body into the links it hides', () => {
    expect(decodeSubscriptionBody(b64(links))).toBe(links);
  });

  it('accepts base64 wrapped over several lines', () => {
    const encoded = b64(links);
    const wrapped = encoded.match(/.{1,16}/g)?.join('\r\n') ?? '';
    expect(decodeSubscriptionBody(wrapped)).toBe(links);
  });

  it('leaves a plain list of links untouched', () => {
    expect(decodeSubscriptionBody(links)).toBe(links);
  });

  it('leaves base64 that does not hide links untouched', () => {
    const plain = b64('just some words');
    expect(decodeSubscriptionBody(plain)).toBe(plain);
  });

  it('parseSubscription reads servers from a base64 body', () => {
    const result = parseSubscription(b64(links));
    expect(result.servers).toEqual([
      { scheme: 'vless', name: 'One' },
      { scheme: 'trojan', name: 'Two' },
    ]);
  });
});

describe('vmessName', () => {
  it('takes the name from the ps field of the base64 JSON', () => {
    expect(vmessName(`vmess://${b64(VMESS_JSON)}`)).toBe('Сервер «Берлин»');
  });

  it('returns null when ps is missing, empty or not a value', () => {
    expect(vmessName(`vmess://${b64(JSON.stringify({ add: 'h' }))}`)).toBeNull();
    expect(vmessName(`vmess://${b64(JSON.stringify({ ps: '  ' }))}`)).toBeNull();
    expect(vmessName(`vmess://${b64(JSON.stringify({ ps: { x: 1 } }))}`)).toBeNull();
  });

  it('returns null for broken JSON or a broken payload', () => {
    expect(vmessName(`vmess://${b64('{not json')}`)).toBeNull();
    expect(vmessName('vmess://***')).toBeNull();
  });

  it('accepts a numeric ps value', () => {
    expect(vmessName(`vmess://${b64(JSON.stringify({ ps: 7 }))}`)).toBe('7');
  });

  it('parseSubscription prefers the ps name for vmess and falls back to the fragment', () => {
    const result = parseSubscription(
      [`vmess://${b64(VMESS_JSON)}#ignored`, `vmess://${b64('{"add":"h"}')}#Fallback`].join('\n'),
    );
    expect(result.servers.map((server) => server.name)).toEqual(['Сервер «Берлин»', 'Fallback']);
  });
});

describe('formatElapsed', () => {
  it('formats seconds as HH:MM:SS', () => {
    expect(formatElapsed(0)).toBe('00:00:00');
    expect(formatElapsed(65)).toBe('00:01:05');
    expect(formatElapsed(3725)).toBe('01:02:05');
  });

  it('keeps counting past 99 hours and clamps invalid input to zero', () => {
    expect(formatElapsed(100 * 3600)).toBe('100:00:00');
    expect(formatElapsed(-5)).toBe('00:00:00');
    expect(formatElapsed(Number.NaN)).toBe('00:00:00');
  });
});

describe('stubs', () => {
  it('connecting returns the TUN reason and changes no state', () => {
    expect(requestVpnConnect()).toEqual({ ok: false, reason: VPN_CONNECT_REASON });
    expect(VPN_CONNECT_REASON).toBe('Нужен драйвер TUN');
  });

  it('a network change returns a short reason without a not-connected wording', () => {
    expect(requestNetworkChange()).toEqual({ ok: false, reason: NETWORK_CHANGE_REASON });
    expect(NETWORK_CHANGE_REASON).toBe('Нужен сетевой модуль');
  });
});
