const UNITS = ['Б', 'КБ', 'МБ', 'ГБ', 'ТБ'] as const;

/** Размер в двоичных единицах: 0 → "0 Б", 1536 → "1.5 КБ". */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} Б`;
  }
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(1)} ${UNITS[unit]}`;
}

/** Процент без дробной части: 12.4 → "12 %". */
export function formatPercent(value: number): string {
  return `${Math.round(value)} %`;
}
