import { describe, expect, it } from 'vitest';
import { filterProcesses, iconColor, isProtectedPid, REFRESH_MS } from './processes';
import type { ProcessInfo } from './processesApi';

const LIST: ProcessInfo[] = [
  { pid: 100, name: 'chrome.exe', cpuPercent: 3.2, memoryBytes: 500, startTimeUnix: 1 },
  { pid: 2048, name: 'Explorer.EXE', cpuPercent: 0.1, memoryBytes: 200, startTimeUnix: 2 },
  { pid: 7, name: 'svchost.exe', cpuPercent: 0, memoryBytes: 50, startTimeUnix: 3 },
];

describe('filterProcesses', () => {
  it('returns everything for an empty or blank query', () => {
    expect(filterProcesses(LIST, '')).toEqual(LIST);
    expect(filterProcesses(LIST, '   ')).toHaveLength(3);
  });

  it('matches the name case-insensitively', () => {
    expect(filterProcesses(LIST, 'EXPLORER').map((p) => p.pid)).toEqual([2048]);
  });

  it('matches the PID as text', () => {
    expect(filterProcesses(LIST, '204').map((p) => p.pid)).toEqual([2048]);
    expect(filterProcesses(LIST, '7').map((p) => p.pid)).toEqual([7]);
  });

  it('returns an empty list when nothing matches', () => {
    expect(filterProcesses(LIST, 'nothing-like-this')).toEqual([]);
  });

  it('does not change the input list', () => {
    const copy = [...LIST];
    filterProcesses(LIST, 'chrome');
    expect(LIST).toEqual(copy);
  });
});

describe('process helpers', () => {
  it('marks PID 0 and PID 4 as protected in the UI', () => {
    expect(isProtectedPid(0)).toBe(true);
    expect(isProtectedPid(4)).toBe(true);
    expect(isProtectedPid(5)).toBe(false);
  });

  it('gives the same icon colour for the same name', () => {
    expect(iconColor('chrome.exe')).toBe(iconColor('chrome.exe'));
  });

  it('refreshes every three seconds', () => {
    expect(REFRESH_MS).toBe(3000);
  });
});
