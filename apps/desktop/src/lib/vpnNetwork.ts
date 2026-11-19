import { invoke } from '@tauri-apps/api/core';
import { formatBytes } from './format';
import type { NetworkAdapter } from './generated/NetworkAdapter';
import type { TcpConnection } from './generated/TcpConnection';

export type { NetworkAdapter, TcpConnection };

/** Как часто обновляем вкладки «Сеть» и «Соединения». */
export const NETWORK_REFRESH_MS = 2000;
export const CONNECTIONS_REFRESH_MS = 3000;

export const CONNECTIONS_UNAVAILABLE = 'Список соединений пока не подключён';

export async function listAdapters(): Promise<NetworkAdapter[]> {
  return invoke<NetworkAdapter[]>('get_network_adapters');
}

export async function listConnections(): Promise<TcpConnection[]> {
  return invoke<TcpConnection[]>('get_tcp_connections');
}

/** Сообщение об ошибке команды: поле `message` уже на русском. */
export function errorMessage(err: unknown): string {
  if (typeof err === 'object' && err !== null && 'message' in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}

/** Скорость в человеческих единицах: 0 → "0 Б/с", 1536 → "1.5 КБ/с", 3 МБ → "3.0 МБ/с". */
export function formatRate(bytesPerSecond: number): string {
  return `${formatBytes(bytesPerSecond)}/с`;
}

export type ConnectionSortKey = 'local' | 'remote' | 'state' | 'process' | 'pid';
export type SortDirection = 'asc' | 'desc';

export interface ConnectionSort {
  key: ConnectionSortKey;
  direction: SortDirection;
}

/** Клик по заголовку: тот же столбец меняет направление, другой — начинается с возрастания. */
export function nextSort(current: ConnectionSort, key: ConnectionSortKey): ConnectionSort {
  if (current.key === key) {
    return { key, direction: current.direction === 'asc' ? 'desc' : 'asc' };
  }
  return { key, direction: 'asc' };
}

function compareText(a: string, b: string): number {
  return a.localeCompare(b, 'ru', { numeric: true, sensitivity: 'base' });
}

function compareBy(key: ConnectionSortKey, a: TcpConnection, b: TcpConnection): number {
  switch (key) {
    case 'local':
      return compareText(a.localAddr, b.localAddr) || a.localPort - b.localPort;
    case 'remote':
      return compareText(a.remoteAddr, b.remoteAddr) || a.remotePort - b.remotePort;
    case 'state':
      return compareText(a.state, b.state);
    case 'process':
      return compareText(a.processName ?? '', b.processName ?? '');
    case 'pid':
      return (a.pid ?? -1) - (b.pid ?? -1);
  }
}

/** Новый отсортированный список; исходный не меняется. Равные строки сохраняют порядок. */
export function sortConnections(
  list: readonly TcpConnection[],
  sort: ConnectionSort,
): TcpConnection[] {
  const sign = sort.direction === 'asc' ? 1 : -1;
  return [...list].sort((a, b) => sign * compareBy(sort.key, a, b));
}

/** Адрес с портом; IPv6 в квадратных скобках. */
export function formatEndpoint(addr: string, port: number): string {
  return addr.includes(':') ? `[${addr}]:${port}` : `${addr}:${port}`;
}

/** Устойчивый ключ соединения: по нему держится выбор строки при обновлении списка. */
export function connectionKey(c: TcpConnection): string {
  return `${c.localAddr}:${c.localPort}>${c.remoteAddr}:${c.remotePort}#${c.pid ?? ''}`;
}

export type ConnectionAction = 'copy-remote' | 'copy-local' | 'show-process' | 'open-location';

export interface ConnectionMenuItem {
  id: ConnectionAction;
  label: string;
  disabled?: boolean;
  reason?: string | null;
}

/** Только настоящие действия. Нет процесса — пункты про процесс недоступны, с причиной. */
export function connectionMenuItems(c: TcpConnection): ConnectionMenuItem[] {
  const hasProcess = c.pid !== null && c.pid > 0;
  const noProcess = 'Процесс для этого соединения не определён';
  return [
    { id: 'copy-remote', label: 'Копировать удалённый адрес' },
    { id: 'copy-local', label: 'Копировать локальный адрес' },
    {
      id: 'show-process',
      label: 'Показать процесс',
      disabled: !hasProcess,
      reason: hasProcess ? null : noProcess,
    },
    {
      id: 'open-location',
      label: 'Открыть расположение файла',
      disabled: !hasProcess,
      reason: hasProcess ? null : noProcess,
    },
  ];
}

/** Строка адреса для копирования. */
export function endpointText(c: TcpConnection, which: 'remote' | 'local'): string {
  return which === 'remote'
    ? formatEndpoint(c.remoteAddr, c.remotePort)
    : formatEndpoint(c.localAddr, c.localPort);
}
