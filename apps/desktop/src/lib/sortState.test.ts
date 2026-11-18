import { describe, expect, it } from 'vitest';
import { ariaSort, nextSort, sortList } from './sortState';

describe('nextSort', () => {
  it('starts with the given direction and flips on repeated clicks', () => {
    const first = nextSort(null, 'cpu', 'desc');
    expect(first).toEqual({ key: 'cpu', dir: 'desc' });
    expect(nextSort(first, 'cpu', 'desc')).toEqual({ key: 'cpu', dir: 'asc' });
    expect(nextSort({ key: 'cpu', dir: 'asc' }, 'cpu', 'desc')).toEqual({
      key: 'cpu',
      dir: 'desc',
    });
  });

  it('a different column starts over with its first direction', () => {
    expect(nextSort({ key: 'cpu', dir: 'asc' }, 'name', 'asc')).toEqual({
      key: 'name',
      dir: 'asc',
    });
  });
});

describe('ariaSort', () => {
  it('maps the state to aria-sort', () => {
    expect(ariaSort({ key: 'a', dir: 'asc' }, 'a')).toBe('ascending');
    expect(ariaSort({ key: 'a', dir: 'desc' }, 'a')).toBe('descending');
    expect(ariaSort({ key: 'a', dir: 'desc' }, 'b')).toBe('none');
    expect(ariaSort(null, 'a')).toBe('none');
  });
});

describe('sortList', () => {
  it('compares Russian text and numbers naturally and keeps ties stable', () => {
    const items = [{ n: 'Яблоко' }, { n: 'арбуз' }, { n: 'Файл 10' }, { n: 'Файл 2' }];
    expect(sortList(items, (i) => i.n, 'asc').map((i) => i.n)).toEqual([
      'арбуз',
      'Файл 2',
      'Файл 10',
      'Яблоко',
    ]);
    const ties = [
      { k: 1, id: 'a' },
      { k: 1, id: 'b' },
    ];
    expect(sortList(ties, (i) => i.k, 'desc').map((i) => i.id)).toEqual(['a', 'b']);
  });

  it('puts missing values last in both directions', () => {
    const items = [{ v: null }, { v: 2 }, { v: 5 }] as Array<{ v: number | null }>;
    expect(sortList(items, (i) => i.v, 'desc').map((i) => i.v)).toEqual([5, 2, null]);
    expect(sortList(items, (i) => i.v, 'asc').map((i) => i.v)).toEqual([2, 5, null]);
  });
});
