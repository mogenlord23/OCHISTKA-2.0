import { describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import {
  assessFile,
  clamavStatus,
  defaultScanTarget,
  getSystemOverview,
  scanStart,
  toCommandError,
} from './api';
import type { ScanReport, SystemOverview } from './types';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

const overview: SystemOverview = {
  cpu: { usagePercent: 12, logicalCores: 8 },
  memory: { totalBytes: 100, usedBytes: 50, usagePercent: 50 },
  disks: [],
};

const report: ScanReport = { scannedFiles: 3, infectedFiles: 0, errors: 0, findings: [] };

describe('getSystemOverview', () => {
  it('invokes get_system_overview and returns the snapshot', async () => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(overview);
    await expect(getSystemOverview()).resolves.toEqual(overview);
    expect(invokeMock).toHaveBeenCalledWith('get_system_overview');
  });

  it('rejects with the command error unchanged', async () => {
    invokeMock.mockReset();
    const error = { kind: 'unsupported', message: 'Сведения о системе недоступно на macOS' };
    invokeMock.mockImplementation(() => Promise.reject(error));
    const caught = await getSystemOverview().catch((err: unknown) => err);
    expect(caught).toEqual(error);
  });
});

describe('scan commands', () => {
  it('starts a scan with the target folder', async () => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(report);
    await expect(scanStart('C:\\Users\\Slava\\Downloads')).resolves.toEqual(report);
    expect(invokeMock).toHaveBeenCalledWith('scan_start', {
      target: 'C:\\Users\\Slava\\Downloads',
    });
  });

  it('reads the ClamAV engine status', async () => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ available: false, reason: 'clamscan.exe не найден' });
    await expect(clamavStatus()).resolves.toEqual({
      available: false,
      reason: 'clamscan.exe не найден',
    });
    expect(invokeMock).toHaveBeenCalledWith('clamav_status');
  });

  it('assesses one file by its path', async () => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ score: 40, reasons: [] });
    await expect(assessFile('C:\\a.exe')).resolves.toEqual({ score: 40, reasons: [] });
    expect(invokeMock).toHaveBeenCalledWith('assess_file', { path: 'C:\\a.exe' });
  });

  it('reads the default scan folder', async () => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue('C:\\Users\\Slava\\Downloads');
    await expect(defaultScanTarget()).resolves.toBe('C:\\Users\\Slava\\Downloads');
    expect(invokeMock).toHaveBeenCalledWith('default_scan_target');
  });
});

describe('toCommandError', () => {
  it('passes a command error through, including the scan kinds', () => {
    const error = { kind: 'engineMissing', message: 'движок недоступен: x' };
    expect(toCommandError(error)).toEqual(error);
  });

  it('wraps any other rejection as a probe error', () => {
    expect(toCommandError(new Error('сбой'))).toEqual({ kind: 'probe', message: 'Error: сбой' });
  });
});
