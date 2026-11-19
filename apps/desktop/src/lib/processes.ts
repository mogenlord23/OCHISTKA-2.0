import { SECTION_COLORS } from './tokens';
import type { ProcessInfo } from './processesApi';
import { sortList, type SortState } from './sortState';
import { formatCpuPercent } from './units';

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

/** Критические процессы Windows. Зеркало `PROTECTED_NAMES` в ядре (processes.rs): здесь только
    серая кнопка и причина, а решение всегда принимает бэкенд. */
export const PROTECTED_NAMES: readonly string[] = [
  'system',
  'system idle process',
  'registry',
  'memory compression',
  'secure system',
  'smss.exe',
  'csrss.exe',
  'wininit.exe',
  'winlogon.exe',
  'services.exe',
  'lsass.exe',
  'lsaiso.exe',
  'svchost.exe',
  'dwm.exe',
  'fontdrvhost.exe',
  'sihost.exe',
  'ctfmon.exe',
  'audiodg.exe',
];

/** PID, которые интерфейс не предлагает завершать: простой (0) и System (4). */
export function isProtectedPid(pid: number): boolean {
  return pid === 0 || pid === 4;
}

/** Защищён ли процесс: по PID или по имени из списка критических. */
export function isProtectedProcess(proc: { pid: number; name: string }): boolean {
  return isProtectedPid(proc.pid) || PROTECTED_NAMES.includes(proc.name.trim().toLowerCase());
}

/** Почему нельзя завершить процесс, или `null`, если можно. */
export function protectedReason(proc: { pid: number; name: string }): string | null {
  return isProtectedProcess(proc)
    ? 'Системный процесс Windows: его завершение остановит систему или сеанс'
    : null;
}

/** ЦП в процентах до сотых: «3,97%». */
export const formatCpu = formatCpuPercent;

export type ProcessSortKey = 'name' | 'pid' | 'cpu' | 'memory';

/** Значение процесса для сортировки по столбцу. Память 0 — «нет данных», уходит в конец. */
export function processSortValue(proc: ProcessInfo, key: ProcessSortKey): number | string | null {
  switch (key) {
    case 'name':
      return proc.name;
    case 'pid':
      return proc.pid;
    case 'cpu':
      return proc.cpuPercent;
    case 'memory':
      return proc.memoryBytes > 0 ? proc.memoryBytes : null;
  }
}

/** Список процессов в порядке выбранной сортировки. Без сортировки — по имени. */
export function sortProcesses(
  list: readonly ProcessInfo[],
  state: SortState<ProcessSortKey> | null,
): ProcessInfo[] {
  const { key, dir } = state ?? { key: 'name' as const, dir: 'asc' as const };
  return sortList(list, (proc) => processSortValue(proc, key), dir);
}
