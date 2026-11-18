import { SECTION_COLORS } from './tokens';
import type { ProcessInfo } from './processesApi';

/** Интервал обновления списка процессов. */
export const REFRESH_MS = 3000;

/** Фильтр поиска: подстрока в имени (без учёта регистра) или в PID. Пустой запрос — весь список. */
export function filterProcesses(list: readonly ProcessInfo[], query: string): ProcessInfo[] {
  const needle = query.trim().toLowerCase();
  if (needle === '') return [...list];
  return list.filter(
    (proc) => proc.name.toLowerCase().includes(needle) || String(proc.pid).includes(needle),
  );
}

const ICON_COLORS = [
  SECTION_COLORS.processes,
  SECTION_COLORS.programs,
  SECTION_COLORS.disks,
  SECTION_COLORS.computer,
  SECTION_COLORS.viruses,
  SECTION_COLORS.vpn,
];

/** Цвет иконки-квадрата: один и тот же для одного имени. Иконки из файлов не читаем. */
export function iconColor(name: string): string {
  let hash = 0;
  for (const ch of name) hash = (hash * 31 + ch.codePointAt(0)!) >>> 0;
  return ICON_COLORS[hash % ICON_COLORS.length];
}

/** PID, которые интерфейс не предлагает завершать: простой (0) и System (4). */
export function isProtectedPid(pid: number): boolean {
  return pid === 0 || pid === 4;
}

/** ЦП в процентах с одним знаком после запятой. */
export function formatCpu(percent: number): string {
  return `${percent.toFixed(1)}%`;
}
