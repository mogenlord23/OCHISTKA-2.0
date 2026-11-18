import { i18n, t, type Key } from './i18n.svelte';
import type { DiskKind } from './generated/DiskKind';

/** Строка с подстановкой `{name}`. */
export function tf(key: Key, vars: Record<string, string | number>): string {
  return t(key).replace(/\{(\w+)\}/g, (_, name: string) => String(vars[name] ?? ''));
}

/** `C:\` → `C:`; не-Windows точки монтирования остаются как есть. */
export function driveLetter(mountPoint: string): string {
  const match = /^([a-zA-Z]):[\\/]?$/.exec(mountPoint);
  return match ? `${match[1].toUpperCase()}:` : mountPoint;
}

export function kindLabel(kind: DiskKind): string {
  if (kind === 'ssd') return 'SSD';
  if (kind === 'hdd') return 'HDD';
  return t('disks.kind.unknown');
}

/** Короткая модель или честное «Модель неизвестна». Модель уже сокращена на стороне Rust. */
export function modelLabel(model: string | null | undefined): string {
  const text = (model ?? '').trim();
  return text === '' ? t('disks.model.unknown') : text;
}

const UNIT_KEYS: Key[] = [
  'disks.unit.b',
  'disks.unit.kb',
  'disks.unit.mb',
  'disks.unit.gb',
  'disks.unit.tb',
];

/** Размер в двоичных единицах на языке интерфейса. */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} ${t(UNIT_KEYS[0])}`;
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNIT_KEYS.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(1)} ${t(UNIT_KEYS[unit])}`;
}

const CATEGORY_KEYS: Record<string, Key> = {
  temp: 'disks.cat.temp',
  tmp: 'disks.cat.tmp',
  'local-temp': 'disks.cat.localTemp',
  'chrome-cache': 'disks.cat.chromeCache',
  'edge-cache': 'disks.cat.edgeCache',
  recent: 'disks.cat.recent',
  'chrome-history': 'disks.cat.chromeHistory',
  'edge-history': 'disks.cat.edgeHistory',
};

/** Название категории очистки: известные id переводятся, остальное — текст бэкенда. */
export function localizeCategoryTitle(id: string, title: string): string {
  if (i18n.lang === 'ru') return title;
  const key = CATEGORY_KEYS[id];
  if (key) return t(key);
  const profile = /\(([^)]*)\)\s*$/.exec(title)?.[1] ?? '';
  if (id.startsWith('firefox-cache-')) return tf('disks.cat.firefoxCache', { name: profile });
  if (id.startsWith('firefox-history-')) return tf('disks.cat.firefoxHistory', { name: profile });
  return title;
}

export function dateLocale(): string {
  return i18n.lang === 'ru' ? 'ru-RU' : 'en-GB';
}
