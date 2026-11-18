/** Причины, по которым функции раздела «Компьютер» пока выключены. Показываются в интерфейсе. */

import { formatBytes, formatPercent } from './format';
import type { GpuInfo, MotherboardInfo } from './types';

export const UNSUPPORTED_OS_REASON = 'Нет поддержки этой ОС в текущей версии';

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

/** Процент для верхней строки: «42%». */
export function shortPercent(value: number): string {
  return `${Math.round(value)}%`;
}

/** Процент в карточке сведений с запятой: «0,3%». */
export function detailPercent(value: number): string {
  const rounded = Math.round(value * 10) / 10;
  return `${String(rounded).replace('.', ',')}%`;
}

/** Время замера «16:54:24». */
export function clockTime(date: Date): string {
  const two = (n: number) => String(n).padStart(2, '0');
  return `${two(date.getHours())}:${two(date.getMinutes())}:${two(date.getSeconds())}`;
}

/** Название процессора; пока не прочитано — прочерк. */
export function cpuName(hardware: { cpuBrand: string | null } | null): string {
  return hardware?.cpuBrand ?? 'Процессор';
}

/** Объём памяти для плитки: «23,9 ГБ». */
export function ramTotal(totalBytes: number): string {
  return formatBytes(totalBytes).replace('.', ',');
}

/**
 * Названия дисков в одну строку. Если модели недоступны, честно: «SSD · C:».
 * Тип и буква берутся из списка разделов.
 */
export function drivesLine(
  models: readonly string[],
  disks: readonly { mountPoint: string; kind: 'ssd' | 'hdd' | 'unknown' }[],
): string {
  if (models.length > 0) {
    return models.join(' / ');
  }
  if (disks.length === 0) {
    return 'Диски не найдены';
  }
  return disks
    .map((disk) => {
      const letter = disk.mountPoint.replace(/[\\/]+$/, '');
      const kind = disk.kind === 'ssd' ? 'SSD' : disk.kind === 'hdd' ? 'HDD' : 'Диск';
      return `${kind} · ${letter}`;
    })
    .join(' / ');
}

/** Операционная система одной строкой: «Майкрософт Windows 11 Домашняя · 10.0.26200». */
export function systemLine(osName: string, osVersion: string): string {
  const name = osName || 'Система не определена';
  return osVersion ? `${name} · ${osVersion}` : name;
}

/** Видеокарта целиком: «NVIDIA GeForce GTX 1060 3GB», без повтора производителя. */
export function gpuFullName(gpu: GpuInfo): string {
  const { vendor, model } = gpu;
  if (model && vendor && !model.toLowerCase().includes(vendor.toLowerCase())) {
    return `${vendor} ${model}`;
  }
  return model ?? vendor ?? '—';
}

/** Плата одной строкой: «ASRock · Z170 Extreme4». */
export function boardLine(board: MotherboardInfo): string {
  const parts = [board.vendor, board.name].filter((part): part is string => Boolean(part));
  return parts.length > 0 ? parts.join(' · ') : '—';
}

const NAME_NOISE: readonly RegExp[] = [
  /[®™]/g,
  /\((?:R|TM|tm|r)\)/g,
  /@\s*[\d.,]+\s*[GM]Hz/gi,
  /\b[\d.,]+\s*GHz\b/gi,
  /\b\d+(?:st|nd|rd|th)\s+Gen\b/gi,
  /\b\d+-Core\b/gi,
  /\b(?:CPU|Processor)\b/gi,
];

/** Короткое название комплектующей: без частот, знаков торговой марки, повторов и лишней длины. */
export function shortHardwareName(raw: string | null | undefined, max = 32): string {
  let text = raw ?? '';
  for (const noise of NAME_NOISE) {
    text = text.replace(noise, ' ');
  }
  const words: string[] = [];
  for (const word of text.split(/\s+/).filter(Boolean)) {
    if (words.length === 0 || words[words.length - 1].toLowerCase() !== word.toLowerCase()) {
      words.push(word);
    }
  }
  const joined = words.join(' ');
  if (joined === '') {
    return '—';
  }
  if (joined.length <= max) {
    return joined;
  }
  const cut = joined.slice(0, max - 1);
  const space = cut.lastIndexOf(' ');
  return `${(space > 0 ? cut.slice(0, space) : cut).trimEnd()}…`;
}

/** Название для карточки дисков: первая модель и число остальных, иначе «SSD · C:». */
export function driveCardName(
  models: readonly string[],
  disks: readonly { mountPoint: string; kind: 'ssd' | 'hdd' | 'unknown' }[],
): string {
  if (models.length === 0) {
    return drivesLine(models, disks);
  }
  const first = shortHardwareName(models[0]);
  return models.length > 1 ? `${first} +${models.length - 1}` : first;
}
