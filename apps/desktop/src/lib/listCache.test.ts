import { beforeEach, describe, expect, it } from 'vitest';
import { cachedList, clearListCache, rememberList } from './listCache';

describe('listCache', () => {
  beforeEach(clearListCache);

  it('returns null before the first list and the last list afterwards', () => {
    expect(cachedList('x')).toBeNull();
    rememberList('x', [1, 2]);
    expect(cachedList<number>('x')).toEqual([1, 2]);
    rememberList('x', [3]);
    expect(cachedList<number>('x')).toEqual([3]);
  });

  it('keeps lists of different sections apart and does not share the array', () => {
    const list = [1];
    rememberList('a', list);
    list.push(2);
    expect(cachedList<number>('a')).toEqual([1]);
    expect(cachedList('b')).toBeNull();
  });
});
