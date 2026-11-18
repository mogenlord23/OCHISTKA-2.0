import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cachedIcon, loadIcon, resetIconCache } from './iconLoader';

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
