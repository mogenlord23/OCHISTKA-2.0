import { prefetchIcons } from './iconLoader';
import { getPathIcons } from './programsApi';
import { exePath, type ProcessInfo } from './processesApi';

/**
 * Фоновая загрузка значков процессов сразу после загрузки списка. Ключ значка — имя процесса;
 * для каждой пачки имён берётся путь exe у первого процесса с этим именем, затем значки по путям
 * одним запросом. Имя без пути (нет доступа) остаётся без значка и рисуется буквой.
 */
export function prefetchProcessIcons(processes: readonly ProcessInfo[]): void {
  const pidByName = new Map<string, number>();
  for (const proc of processes) {
    if (!pidByName.has(proc.name)) pidByName.set(proc.name, proc.pid);
  }
  prefetchIcons([...pidByName.keys()], async (names) => {
    const paths = await Promise.all(
      names.map((name) => exePath(pidByName.get(name) ?? 0).catch(() => null)),
    );
    const wanted = [...new Set(paths.filter((path): path is string => !!path))];
    const found = wanted.length > 0 ? await getPathIcons(wanted) : new Map();
    return new Map(names.map((name, i) => [name, paths[i] ? (found.get(paths[i]) ?? null) : null]));
  });
}
