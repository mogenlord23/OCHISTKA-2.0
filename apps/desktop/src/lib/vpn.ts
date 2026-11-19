/** Чистая логика раздела VPN: проверка ссылки, состояния кнопки, форматы. Без системных вызовов. */
import type { ConnectionStatus, ServerView, VpnSnapshot } from './vpnApi';

export const NO_SERVERS_REASON = 'Сначала импортируй подписку и выбери сервер';
export const TUN_MODE_REASON =
  'Режим TUN (весь компьютер) пока не поддерживается: нужны права администратора и драйвер';
export const IMPORTING_LABEL = 'Загружаем подписку…';
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

/**
 * Что можно отправить на импорт. Ссылку проверяем сразу (http:// отклоняем), остальное
 * (список ссылок серверов, base64, JSON) отдаём Rust как есть.
 */
export function prepareImport(input: string): { ok: true; input: string } | StubResult {
  const value = input.trim();
  if (value === '') return { ok: false, reason: 'Вставь ссылку подписки или список серверов' };
  if (/^http:\/\//i.test(value) && !/\s/.test(value)) {
    return { ok: false, reason: 'Разрешены только ссылки https://' };
  }
  return { ok: true, input: value };
}

/** Строка из вставленного поля: между ссылками серверов оставляем переводы строк. */
export function normalizePasted(input: string): string {
  return input.replace(/\s+(?=(vless|vmess|trojan|ss|hysteria2|hy2|tuic):\/\/)/gi, '\n');
}

export interface ConnectUi {
  label: string;
  pressed: boolean;
  busy: boolean;
  disabled: boolean;
  /** Почему кнопка недоступна; показывается в строке состояния. */
  reason: string | null;
}

/** Состояние главной кнопки «Подключить» по снимку и занятости (импорт, ожидание ответа). */
export function connectUi(snapshot: VpnSnapshot | null, busy: boolean): ConnectUi {
  const state = snapshot?.connection.state ?? 'disconnected';
  if (state === 'connecting' || (busy && state !== 'connected')) {
    return { label: 'Подключаем…', pressed: false, busy: true, disabled: true, reason: null };
  }
  if (state === 'connected') {
    return { label: 'Отключить', pressed: true, busy, disabled: busy, reason: null };
  }
  const base = { label: 'Подключить', pressed: false, busy: false };
  if (!snapshot || snapshot.servers.length === 0 || snapshot.selected === null) {
    return { ...base, disabled: true, reason: NO_SERVERS_REASON };
  }
  if (!snapshot.core.available) {
    return {
      ...base,
      disabled: true,
      reason: `VPN-ядро не найдено${snapshot.core.reason ? `: ${snapshot.core.reason}` : ''}`,
    };
  }
  return { ...base, disabled: false, reason: null };
}

/** Строка состояния под таймером. */
export function statusLine(connection: ConnectionStatus | null, hint: string | null): string {
  if (!connection) return 'Не подключено';
  if (connection.state === 'disconnected') return hint ?? connection.message;
  return connection.message;
}

/** Задержка сервера: «45 мс», «нет ответа» или пусто, если ещё не мерили. */
export function formatLatency(server: Pick<ServerView, 'latencyMs' | 'measured'>): string {
  if (!server.measured) return '';
  return server.latencyMs === null ? 'нет ответа' : `${server.latencyMs} мс`;
}

/** Секунды с начала подключения (Unix-секунды `since`). */
export function elapsedSince(since: number | null, nowMs: number): number {
  if (since === null) return 0;
  return Math.max(0, Math.floor(nowMs / 1000) - since);
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

/** Имя сервера без флага-эмодзи в начале (флаг показываем кодом страны). */
export function displayName(name: string): string {
  const points = [...name].slice(0, 2).map((c) => c.codePointAt(0) ?? 0);
  const isFlag = points.length === 2 && points.every((p) => p >= 0x1f1e6 && p <= 0x1f1ff);
  return isFlag ? [...name].slice(2).join('').trim() : name;
}

/** Записи списка из текста: по одной на строку, допустимы запятая и точка с запятой. */
export function splitEntries(text: string): string[] {
  return text
    .split(/[\n,;]+/)
    .map((item) => item.trim())
    .filter((item) => item !== '');
}

export function joinEntries(list: readonly string[]): string {
  return list.join('\n');
}

/** Заглушка изменения сети (Zapret, Xbox DNS): системные настройки не трогаем. */
export function requestNetworkChange(): StubResult {
  return { ok: false, reason: NETWORK_CHANGE_REASON };
}

/** Текст импорта по итогам: сколько серверов и сколько записей пропущено. */
export function importMessage(count: number, skipped: number, issues: readonly string[]): string {
  const base = `Серверов: ${count}.`;
  return skipped > 0
    ? `${base} Пропущено записей: ${skipped} (${issues.slice(0, 2).join('; ')}).`
    : base;
}
