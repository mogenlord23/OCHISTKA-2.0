/** Чистая логика раздела VPN: проверка ссылки, состояния кнопки, форматы. Без системных вызовов. */
import type { ConnectionStatus, ServerView, VpnSnapshot } from './vpnApi';

export const NO_SERVERS_REASON = 'Сначала импортируй подписку и выбери сервер';
export const TUN_MODE_REASON =
  'Режим TUN (весь компьютер) пока не поддерживается: нужны права администратора и драйвер';
export const IMPORTING_LABEL = 'Загружаем подписку…';
export const NETWORK_CHANGE_REASON = 'Пока недоступно: появится в одном из следующих обновлений';
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

export const PREPARING_LABEL = 'Подготовка VPN…';
export const RETRY_LABEL = 'Повторить';
export const PREPARE_FAILED_PREFIX = 'Не удалось подготовить VPN';

/** Подпись на кнопке во время подготовки: «Подготовка VPN… 42%». */
export function preparingLabel(percent: number | null): string {
  return percent && percent > 0 ? `${PREPARING_LABEL} ${Math.min(100, percent)}%` : PREPARING_LABEL;
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
export function connectUi(
  snapshot: VpnSnapshot | null,
  busy: boolean,
  options: { preparing?: number | null; retry?: boolean } = {},
): ConnectUi {
  const state = snapshot?.connection.state ?? 'disconnected';
  if (options.preparing !== undefined && options.preparing !== null) {
    return {
      label: preparingLabel(options.preparing),
      pressed: false,
      busy: true,
      disabled: true,
      reason: null,
    };
  }
  if (state === 'connecting' || (busy && state !== 'connected')) {
    return { label: 'Подключаем…', pressed: false, busy: true, disabled: true, reason: null };
  }
  if (state === 'connected') {
    return { label: 'Отключить', pressed: true, busy, disabled: busy, reason: null };
  }
  const base = { label: options.retry ? RETRY_LABEL : 'Подключить', pressed: false, busy: false };
  if (!snapshot || snapshot.servers.length === 0 || snapshot.selected === null) {
    return { ...base, disabled: true, reason: NO_SERVERS_REASON };
  }
  // Файлы VPN, если их ещё нет, приложение скачивает само при нажатии: кнопка доступна.
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

/** Состояние переключателя дополнительного инструмента (Zapret, Xbox DNS). */
export type ToolStatus = 'off' | 'on' | 'pending' | 'error';

export interface ToolModel {
  status: ToolStatus;
  /** Функция реализована. Пока нет: переключатель выключен и недоступен, успех не имитируется. */
  available: boolean;
  /** Короткая фраза: почему недоступно или что пошло не так. */
  reason: string | null;
}

/** Свойства Switch по состоянию. «Вкл» показываем только при подтверждённом статусе `on`. */
export function toolSwitchProps(tool: ToolModel) {
  return {
    checked: tool.available && tool.status === 'on',
    pending: tool.available && tool.status === 'pending',
    error: tool.available && tool.status === 'error',
    disabled: !tool.available || tool.status === 'pending',
  };
}

/** Короткое состояние инструмента для подписи у кнопки: только то, что подтверждено. */
export function toolStateText(tool: ToolModel): string {
  if (!tool.available) return 'Выключено · недоступно';
  switch (tool.status) {
    case 'on':
      return 'Включено';
    case 'pending':
      return 'Подождите…';
    case 'error':
      return 'Ошибка';
    default:
      return 'Выключено';
  }
}

/** Результат попытки: успех только после проверки; при ошибке откат в «выкл» и фраза. */
export function applyToolResult(
  tool: ToolModel,
  result: { ok: true } | { ok: false; reason: string },
  wanted: boolean,
): ToolModel {
  return result.ok
    ? { ...tool, status: wanted ? 'on' : 'off', reason: null }
    : { ...tool, status: 'error', reason: result.reason };
}

/** Оба инструмента пока не реализованы: честное состояние без имитации. */
export const TOOL_UNAVAILABLE: ToolModel = {
  status: 'off',
  available: false,
  reason: NETWORK_CHANGE_REASON,
};

/** Заглушка изменения сети (Zapret, Xbox DNS): системные настройки не трогаем. */
export function requestNetworkChange(): StubResult {
  return { ok: false, reason: NETWORK_CHANGE_REASON };
}

/** Текст импорта по итогам: сколько серверов найдено и, если часть записей пропущена, почему. */
export function importMessage(count: number, skipped: number, issues: readonly string[]): string {
  const base = `Импорт завершён: серверов ${count}.`;
  return skipped > 0
    ? `${base} Часть записей пропущена (${skipped}): ${issues.slice(0, 2).join('; ')}.`
    : base;
}
