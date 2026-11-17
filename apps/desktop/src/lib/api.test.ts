import { describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { getSystemOverview, toCommandError } from './api';
import type { SystemOverview } from './types';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

const overview: SystemOverview = {
  cpu: { usagePercent: 12, logicalCores: 8 },
  memory: { totalBytes: 100, usedBytes: 50, usagePercent: 50 },
  disks: [],
};

describe('getSystemOverview', () => {
  it('invokes get_system_overview and returns the snapshot', async () => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(overview);
    await expect(getSystemOverview()).resolves.toEqual(overview);
    expect(invokeMock).toHaveBeenCalledWith('get_system_overview');
  });

  it('rejects with the command error unchanged', async () => {
    const error = { kind: 'unsupported', message: 'Сведения о системе недоступно на macOS' };
    invokeMock.mockReset();
    invokeMock.mockImplementation(() => Promise.reject(error));
    const caught = await getSystemOverview().catch((err: unknown) => err);
    expect(caught).toEqual(error);
  });
});

describe('toCommandError', () => {
  it('passes a command error through', () => {
    const error = { kind: 'probe', message: 'x' };
    expect(toCommandError(error)).toEqual(error);
  });

  it('wraps any other rejection as a probe error', () => {
    expect(toCommandError(new Error('сбой'))).toEqual({ kind: 'probe', message: 'Error: сбой' });
  });
});
