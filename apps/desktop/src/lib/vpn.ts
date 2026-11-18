/** Чистая логика раздела VPN: проверка ссылки, разбор подписки, таймер и заглушки. Без системных вызовов. */

export const VPN_CONNECT_REASON = 'Подключение не подключено: нужен драйвер TUN';
export const VPN_TUN_MODE_REASON = 'Режим TUN требует драйвер TUN';
export const IMPORT_REASON = 'Загрузка подписки пока не подключена';
export const NETWORK_CHANGE_REASON = 'Изменение сети пока не подключено';
export const NETWORK_STATUS_OFF = 'выключено';

export type StubResult = { ok: false; reason: string };

export type SubscriptionUrlResult = { ok: true; url: string } | StubResult;

/** Проверяет ссылку подписки: только https://, без пробелов, с хостом. */
export function validateSubscriptionUrl(input: string): SubscriptionUrlResult {
  const value = input.trim();
  if (value === '') {
    return { ok: false, reason: 'Вставь ссылку подписки' };
  }
  if (/\s/.test(value)) {
    return { ok: false, reason: 'Ссылка не должна содержать пробелов' };
  }
  let parsed: URL;
  try {
    parsed = new URL(value);
  } catch {
    return { ok: false, reason: 'Некорректная ссылка' };
  }
  if (parsed.protocol !== 'https:') {
    return { ok: false, reason: 'Разрешены только ссылки https://' };
  }
  return { ok: true, url: value };
}

export type ServerScheme = 'vless' | 'vmess' | 'trojan' | 'ss';

export interface ServerEntry {
  scheme: ServerScheme;
  /** Имя из фрагмента ссылки (после #). */
  name: string;
}

export interface ParsedSubscription {
  servers: ServerEntry[];
  /** Непустые строки, которые не распознаны; в список не попадают. */
  unknownCount: number;
}

const SUPPORTED_SCHEMES: readonly ServerScheme[] = ['vless', 'vmess', 'trojan', 'ss'];
const UNNAMED = 'Без названия';

function fragmentName(line: string): string {
  const hash = line.indexOf('#');
  if (hash === -1) return UNNAMED;
  const raw = line.slice(hash + 1).trim();
  if (raw === '') return UNNAMED;
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}

/** Разбирает текст подписки: одна ссылка на строку. Распознаёт схему и имя из фрагмента. */
export function parseSubscription(text: string): ParsedSubscription {
  const servers: ServerEntry[] = [];
  let unknownCount = 0;
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (line === '') continue;
    const match = /^([a-z][a-z0-9+.-]*):\/\//i.exec(line);
    const scheme = match?.[1]?.toLowerCase() as ServerScheme | undefined;
    if (scheme && SUPPORTED_SCHEMES.includes(scheme)) {
      servers.push({ scheme, name: fragmentName(line) });
    } else {
      unknownCount += 1;
    }
  }
  return { servers, unknownCount };
}

/** Таймер подключения: секунды в формате ЧЧ:ММ:СС. Некорректные значения дают 00:00:00. */
export function formatElapsed(totalSeconds: number): string {
  const safe = Number.isFinite(totalSeconds) && totalSeconds > 0 ? Math.floor(totalSeconds) : 0;
  const hours = Math.floor(safe / 3600);
  const minutes = Math.floor((safe % 3600) / 60);
  const seconds = safe % 60;
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${pad(hours)}:${pad(minutes)}:${pad(seconds)}`;
}

/** Заглушка подключения: без драйвера TUN ничего не меняет. */
export function requestVpnConnect(): StubResult {
  return { ok: false, reason: VPN_CONNECT_REASON };
}

/** Заглушка изменения сети (Zapret, Xbox DNS): системные настройки не трогаем. */
export function requestNetworkChange(): StubResult {
  return { ok: false, reason: NETWORK_CHANGE_REASON };
}
