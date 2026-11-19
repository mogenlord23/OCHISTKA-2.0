import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearSnapshot, loadSnapshot, saveSnapshot, type ComputerSnapshot } from './computerCache';

const snap: ComputerSnapshot = {
  overview: null,
  hardware: null,
  cpuHistory: [1, 2],
  gpuHistory: [],
  ramHistory: [3],
  updatedAt: 1000,
};

afterEach(() => {
  clearSnapshot();
  vi.restoreAllMocks();
});

describe('computerCache', () => {
  it('returns null when nothing was saved', () => {
    expect(loadSnapshot()).toBeNull();
  });

  it('keeps the last snapshot in memory and in storage', () => {
    saveSnapshot(snap, 10_000, true);
    expect(loadSnapshot()).toEqual(snap);
    expect(JSON.parse(localStorage.getItem('computerSnapshot.v1') ?? 'null')).toEqual(snap);
  });

  it('writes to storage at most every 5 seconds', () => {
    const spy = vi.spyOn(Storage.prototype, 'setItem');
    saveSnapshot(snap, 10_000, true);
    saveSnapshot({ ...snap, cpuHistory: [9] }, 11_000);
    expect(spy).toHaveBeenCalledTimes(1);
    expect(loadSnapshot()?.cpuHistory).toEqual([9]);
    saveSnapshot({ ...snap, cpuHistory: [7] }, 16_000);
    expect(spy).toHaveBeenCalledTimes(2);
  });

  it('survives broken or unavailable storage', () => {
    localStorage.setItem('computerSnapshot.v1', '{oops');
    expect(loadSnapshot()).toBeNull();
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    expect(() => saveSnapshot(snap, 99_000, true)).not.toThrow();
    expect(loadSnapshot()).toEqual(snap);
  });
});
