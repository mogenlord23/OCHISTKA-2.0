/** Причины, по которым функции раздела «Компьютер» пока выключены. Показываются в интерфейсе. */

import { formatBytes, formatPercent } from './format';
import type { GpuInfo, MotherboardInfo } from './types';

export const UNSUPPORTED_OS_REASON = 'Нет поддержки этой ОС в текущей версии';

/** Заполнитель, пока комплектующие ещё не прочитаны (без слов «Читаем»). */
export const HARDWARE_LOADING = '—';

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
  return hardware?.cpuBrand ?? '—';
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

/** Убирает знаки (R), (TM), ® и ™ и лишние пробелы. */
function stripMarks(text: string): string {
  return text
    .replace(/\((?:R|TM|C)\)|[®™©]/gi, '')
    .replace(/\s+/g, ' ')
    .trim();
}

/**
 * Короткое название процессора для плитки: «Intel i7-6700 · 3.40 GHz».
 * Убираем знаки, слова CPU/Processor, «N-Core», «with Radeon Graphics»; частоту выносим в конец.
 * Если формат незнакомый, остаётся очищенный оригинал (ничего не выдумываем).
 */
export function cpuShortName(brand: string | null | undefined): string {
  if (!brand) return '—';
  let name = stripMarks(brand);
  let clock = '';
  const at = name.match(/\s*@\s*([\d.,]+)\s*(GHz|MHz)\b/i);
  if (at) {
    clock = `${at[1].replace(',', '.')} ${at[2].toUpperCase() === 'GHZ' ? 'GHz' : 'MHz'}`;
    name = name.replace(at[0], '');
  }
  name = name
    .replace(/\b\d+-Core Processor\b/gi, '')
    .replace(/\bwith\s+Radeon\s+Graphics\b/gi, '')
    .replace(/\b(?:CPU|Processor)\b/gi, '')
    .replace(/^(Intel)\s+Core\s+(i[3579]-)/i, '$1 $2')
    .replace(/\s+/g, ' ')
    .trim();
  return clock ? `${name} · ${clock}` : name;
}

/**
 * Короткое название видеокарты для плитки: «NVIDIA GTX 1060 3GB».
 * Без повтора производителя, без Corporation/Inc, без слова GeForce перед GTX/RTX/GT/MX.
 */
export function gpuShortName(gpu: GpuInfo): string {
  const vendor = gpu.vendor ? stripMarks(gpu.vendor) : '';
  const model = gpu.model ? stripMarks(gpu.model) : '';
  let name = model || vendor;
  if (!name) return '—';
  const brand = /nvidia/i.test(name + vendor)
    ? 'NVIDIA'
    : /advanced micro devices|\bamd\b|\bati\b|radeon/i.test(name + vendor)
      ? 'AMD'
      : /intel/i.test(name + vendor)
        ? 'Intel'
        : '';
  name = name
    .replace(/Advanced Micro Devices,?\s*Inc\.?\s*\[AMD\/ATI\]\s*/i, '')
    .replace(/\b(?:Corporation|Corp\.?|Inc\.?)\b/gi, '')
    .replace(/\bGeForce\s+(?=(?:GTX|RTX|GT|MX)\b)/i, '')
    .replace(/\s+/g, ' ')
    .trim();
  if (brand && !new RegExp(`^${brand}\\b`, 'i').test(name)) {
    name = `${brand} ${name}`;
  }
  // Повтор производителя в начале: «NVIDIA NVIDIA GTX» → «NVIDIA GTX».
  name = name.replace(/^(\S+)\s+\1\b/i, '$1');
  return name;
}

const DRIVE_BRANDS: Record<string, string> = {
  KINGSTON: 'Kingston',
  SAMSUNG: 'Samsung',
  TOSHIBA: 'Toshiba',
  SEAGATE: 'Seagate',
  CRUCIAL: 'Crucial',
  INTEL: 'Intel',
  HITACHI: 'Hitachi',
  SANDISK: 'SanDisk',
  ADATA: 'ADATA',
  TRANSCEND: 'Transcend',
  PATRIOT: 'Patriot',
  CORSAIR: 'Corsair',
  HGST: 'HGST',
  WDC: 'WD',
};

/**
 * Короткое название накопителя: «KINGSTON SHFS37A120G» → «Kingston SHFS37A120G»,
 * «WDC WD10EZEX-21WN4A0» → «WD WD10EZEX». Убираем «ATA Device», «SCSI Disk Device», «NVMe»-приставки и
 * служебный хвост ревизии после дефиса у WD. Объём и модель остаются.
 */
export function driveShortName(raw: string): string {
  let name = stripMarks(raw)
    .replace(/\s*\((?:Standard disk drives|Стандартные дисковые накопители)\)\s*/gi, ' ')
    .replace(/\b(?:ATA|SCSI|SAS|USB)\s+(?:Disk\s+)?Device\b/gi, '')
    .replace(/\bNVMe\s+/i, '')
    .replace(/\s+/g, ' ')
    .trim();
  const [first, ...rest] = name.split(' ');
  const brand = DRIVE_BRANDS[first.toUpperCase()];
  if (brand) name = [brand, ...rest].join(' ');
  if (brand === 'WD') name = name.replace(/\b(WD\d{2,5}[A-Z]{2,5})-[0-9A-Z]+\b/, '$1');
  return name || '—';
}

/** Список накопителей для плитки; без моделей остаётся прежняя честная строка. */
export function drivesShortLine(
  models: readonly string[],
  disks: readonly { mountPoint: string; kind: 'ssd' | 'hdd' | 'unknown' }[],
): string {
  if (models.length === 0) return drivesLine(models, disks);
  return [...new Set(models.map(driveShortName))].join(' / ');
}
