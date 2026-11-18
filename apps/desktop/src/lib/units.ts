const UNITS = ['Б', 'КБ', 'МБ', 'ГБ', 'ТБ'] as const;

/** Число с одним знаком после запятой, запятая как разделитель: 11.94 → "11,9". */
function oneDecimal(value: number): string {
  return value.toFixed(1).replace('.', ',');
}

/** Размер для таблиц: «11,9 МБ», «1,2 ГБ». Никогда не показывает «сырые» биты и байты выше 1023. */
export function formatSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '—';
  if (bytes < 1024) return `${Math.round(bytes)} Б`;
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${oneDecimal(value)} ${UNITS[unit]}`;
}

/** Память процесса: ноль значит «система не отдала цифру». */
export function formatMemory(bytes: number): string {
  return bytes <= 0 ? 'Нет данных' : formatSize(bytes);
}

/** ЦП в процентах до сотых без хвостовых нулей: 3.97 → "3,97%", 0 → "0%", 1.5 → "1,5%". */
export function formatCpuPercent(percent: number): string {
  if (!Number.isFinite(percent) || percent <= 0) return '0%';
  const rounded = Math.round(percent * 100) / 100;
  return `${String(rounded).replace('.', ',')}%`;
}
