import { afterEach, beforeEach, expect, it } from 'vitest';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { cachedIcon, resetIconCache } from './iconLoader';
import { prefetchProcessIcons } from './processIcons';
import type { ProcessInfo } from './processesApi';

beforeEach(() => resetIconCache());
afterEach(() => clearMocks());

const proc = (pid: number, name: string): ProcessInfo => ({
  pid,
  name,
  cpuPercent: 0,
  memoryBytes: 1,
  startTimeUnix: 1,
});

it('looks up each distinct process name once and maps path icons back to names', async () => {
  const asked: string[] = [];
  mockIPC((cmd, args) => {
    const request = (args as { request: Record<string, unknown> }).request;
    asked.push(`${cmd}:${String(request.action)}`);
    if (request.action === 'exePath') {
      return { type: 'exePath', path: request.pid === 7 ? null : `C:\\A\\${request.pid}.exe` };
    }
    const paths = request.paths as string[];
    return { type: 'iconBatch', items: paths.map((key) => ({ key, dataUrl: `data:${key}` })) };
  });
  prefetchProcessIcons([proc(1, 'a.exe'), proc(2, 'a.exe'), proc(7, 'locked.exe')]);
  await new Promise((resolve) => setTimeout(resolve, 50));
  expect(cachedIcon('a.exe')).toBe('data:C:\\A\\1.exe');
  expect(cachedIcon('locked.exe')).toBeNull();
  expect(asked.filter((entry) => entry.endsWith('exePath')).length).toBe(2);
});
