/** Причины, по которым функции раздела «Компьютер» пока выключены. Показываются в интерфейсе. */

import { formatBytes, formatPercent } from './format';
import type { GpuInfo, MotherboardInfo } from './types';

export const SYSTEM_CHANGE_REASON =
  'Раздел в разработке: системные настройки меняют Windows, пока не подключено';

export const UNSUPPORTED_OS_REASON = 'Нет поддержки этой ОС в текущей версии';

export const DRIVERS_REASON =
  'Раздел в разработке: список драйверов Windows появится после подключения чтения INF и Windows Update';

/** Подпись, пока комплектующие ещё не прочитаны. */
export const HARDWARE_LOADING = 'Читаем…';

/** Название видеокарты: модель, иначе производитель, иначе прочерк. */
export function gpuName(gpu: GpuInfo): string {
  return gpu.model ?? gpu.vendor ?? '—';
}

/** Строка памяти видеокарты: «3.0 ГБ из 8.0 ГБ» или null, если память не известна. */
export function gpuMemoryLine(gpu: GpuInfo): string | null {
  if (gpu.totalMemoryBytes === null) {
    return null;
  }
  const used = gpu.usedMemoryBytes;
  return used === null
    ? `Память: ${formatBytes(gpu.totalMemoryBytes)}`
    : `${formatBytes(used)} из ${formatBytes(gpu.totalMemoryBytes)}`;
}

/** Строка загрузки видеокарты или null, если ОС её не отдаёт. */
export function gpuUsageLine(gpu: GpuInfo): string | null {
  return gpu.usagePercent === null ? null : `Загрузка: ${formatPercent(gpu.usagePercent)}`;
}

/** Название платы: модель, иначе производитель, иначе прочерк. */
export function boardName(board: MotherboardInfo): string {
  return board.name ?? board.vendor ?? '—';
}
