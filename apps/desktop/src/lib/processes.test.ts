import { describe, expect, it } from 'vitest';
import {
  filterProcesses,
  formatCpu,
  iconColor,
  isProtectedPid,
  isProtectedProcess,
  REFRESH_MS,
  sortProcesses,
} from './processes';
import { nextSort } from './sortState';
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

describe('sortProcesses', () => {
  it('sorts by name when no column is chosen', () => {
    expect(sortProcesses(LIST, null).map((p) => p.pid)).toEqual([100, 2048, 7]);
  });

  it('first click on CPU or PID puts the largest first, second click the smallest', () => {
    const cpu1 = nextSort(null, 'cpu' as const, 'desc');
    expect(sortProcesses(LIST, cpu1).map((p) => p.pid)).toEqual([100, 2048, 7]);
    const cpu2 = nextSort(cpu1, 'cpu' as const, 'desc');
    expect(sortProcesses(LIST, cpu2).map((p) => p.pid)).toEqual([7, 2048, 100]);
    const pid1 = nextSort(null, 'pid' as const, 'desc');
    expect(sortProcesses(LIST, pid1).map((p) => p.pid)).toEqual([2048, 100, 7]);
  });

  it('keeps processes without a memory figure last in both directions', () => {
    const none: ProcessInfo = {
      pid: 1,
      name: 'System',
      cpuPercent: 0,
      memoryBytes: 0,
      startTimeUnix: 0,
    };
    const desc = sortProcesses([none, ...LIST], { key: 'memory', dir: 'desc' });
    const asc = sortProcesses([none, ...LIST], { key: 'memory', dir: 'asc' });
    expect(desc.map((p) => p.pid)).toEqual([100, 2048, 7, 1]);
    expect(asc.map((p) => p.pid)).toEqual([7, 2048, 100, 1]);
  });

  it('formats CPU with a decimal comma', () => {
    expect(formatCpu(3.97)).toBe('3,97%');
  });
});

describe('protected process names', () => {
  it('blocks critical Windows processes by name, ignoring case', () => {
    expect(isProtectedProcess({ pid: 700, name: 'LSASS.exe' })).toBe(true);
    expect(isProtectedProcess({ pid: 900, name: 'svchost.exe' })).toBe(true);
    expect(isProtectedProcess({ pid: 4, name: 'System' })).toBe(true);
    expect(isProtectedProcess({ pid: 1234, name: 'notepad.exe' })).toBe(false);
  });
});
