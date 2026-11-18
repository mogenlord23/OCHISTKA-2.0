/** Направление сортировки: `desc` — от большего к меньшему. */
export type SortDir = 'asc' | 'desc';

export interface SortState<K extends string> {
  key: K;
  dir: SortDir;
}

/**
 * Клик по заголовку. Новый столбец: числовой — сначала по убыванию (больше сверху),
 * текстовый — по алфавиту. Повторный клик по тому же столбцу меняет направление.
 */
export function nextSort<K extends string>(
  current: SortState<K> | null,
  key: K,
  firstDir: SortDir,
): SortState<K> {
  if (current && current.key === key) {
    return { key, dir: current.dir === 'desc' ? 'asc' : 'desc' };
  }
  return { key, dir: firstDir };
}

/** Значение `aria-sort` для заголовка столбца. */
export function ariaSort<K extends string>(
  state: SortState<K> | null,
  key: K,
): 'ascending' | 'descending' | 'none' {
  if (!state || state.key !== key) return 'none';
  return state.dir === 'asc' ? 'ascending' : 'descending';
}

const collator = new Intl.Collator('ru', { sensitivity: 'base', numeric: true });

/**
 * Сортирует копию списка. Значения `null`/`undefined` («нет данных») всегда в конце,
 * при любом направлении. Равные по значению строки сохраняют прежний порядок.
 */
export function sortList<T>(
  list: readonly T[],
  value: (item: T) => number | string | null | undefined,
  dir: SortDir,
): T[] {
  const sign = dir === 'asc' ? 1 : -1;
  return list
    .map((item, index) => ({ item, index, v: value(item) }))
    .sort((a, b) => {
      const aNone = a.v === null || a.v === undefined;
      const bNone = b.v === null || b.v === undefined;
      if (aNone || bNone) return aNone === bNone ? a.index - b.index : aNone ? 1 : -1;
      const cmp =
        typeof a.v === 'number' && typeof b.v === 'number'
          ? a.v - b.v
          : collator.compare(String(a.v), String(b.v));
      return cmp !== 0 ? sign * cmp : a.index - b.index;
    })
    .map((entry) => entry.item);
}
