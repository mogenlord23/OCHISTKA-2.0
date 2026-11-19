import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cachedIcon, loadIcon, prefetchIcons, resetIconCache } from './iconLoader';

beforeEach(() => resetIconCache());

describe('loadIcon', () => {
  it('loads once per key (case-insensitive) and then serves the cache', async () => {
    const resolve = vi.fn().mockResolvedValue('data:image/png;base64,QQ==');
    const [a, b] = await Promise.all([
      loadIcon('Chrome.exe', resolve),
      loadIcon('chrome.EXE', resolve),
    ]);
    expect(a).toBe(b);
    expect(resolve).toHaveBeenCalledTimes(1);
    expect(cachedIcon('chrome.exe')).toBe('data:image/png;base64,QQ==');
    await loadIcon('chrome.exe', resolve);
    expect(resolve).toHaveBeenCalledTimes(1);
  });

  it('turns a failing loader into a cached null instead of an error', async () => {
    const resolve = vi.fn().mockRejectedValue(new Error('no access'));
    expect(await loadIcon('x.exe', resolve)).toBeNull();
    expect(cachedIcon('x.exe')).toBeNull();
  });

  it('runs at most four loaders at the same time', async () => {
    let running = 0;
    let peak = 0;
    const releases: Array<() => void> = [];
    const make = () => async () => {
      running++;
      peak = Math.max(peak, running);
      await new Promise<void>((done) => releases.push(done));
      running--;
      return null;
    };
    const jobs = Array.from({ length: 7 }, (_, i) => loadIcon(`k${i}`, make()));
    await vi.waitFor(() => expect(releases.length).toBe(4));
    expect(peak).toBe(4);
    while (releases.length) {
      releases.shift()?.();
      await Promise.resolve();
      await Promise.resolve();
    }
    await vi.waitFor(() => expect(releases.length + running).toBeGreaterThanOrEqual(0));
    while (running > 0 || releases.length) {
      releases.shift()?.();
      await new Promise((r) => setTimeout(r, 0));
    }
    await Promise.all(jobs);
    expect(peak).toBe(4);
  });
});

describe('prefetchIcons', () => {
  it('fetches in chunks, fills the cache and lets a late row reuse the same request', async () => {
    const calls: string[][] = [];
    const fetchBatch = async (keys: string[]) => {
      calls.push(keys);
      return new Map(keys.map((key) => [key, `data:${key}`]));
    };
    prefetchIcons(['a', 'b', 'c', 'A'], fetchBatch, 2);
    const late = vi.fn().mockResolvedValue('never');
    expect(await loadIcon('c', late)).toBe('data:c');
    expect(late).not.toHaveBeenCalled();
    await vi.waitFor(() => expect(cachedIcon('a')).toBe('data:a'));
    expect(calls).toEqual([['a', 'b'], ['c']]);
  });

  it('skips keys that are already known', async () => {
    const fetchBatch = vi.fn(async (keys: string[]) => new Map(keys.map((k) => [k, 'x'])));
    prefetchIcons(['k'], fetchBatch);
    await vi.waitFor(() => expect(cachedIcon('k')).toBe('x'));
    prefetchIcons(['K'], fetchBatch);
    expect(fetchBatch).toHaveBeenCalledTimes(1);
  });

  it('keeps a missing icon as null and does not remember a failed batch', async () => {
    prefetchIcons(['none'], async () => new Map());
    await vi.waitFor(() => expect(cachedIcon('none')).toBeNull());
    prefetchIcons(['boom'], async () => {
      throw new Error('ipc');
    });
    const retry = vi.fn().mockResolvedValue('data:ok');
    expect(await loadIcon('boom', retry)).toBeNull();
    expect(cachedIcon('boom')).toBeUndefined();
    expect(await loadIcon('boom', retry)).toBe('data:ok');
  });
});
