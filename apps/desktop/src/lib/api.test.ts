import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import {
  assessFile,
  clamavStatus,
  defaultScanTarget,
  getSystemOverview,
  isEngineStatus,
  isScanReport,
  isSystemOverview,
  scanStart,
  toCommandError,
} from './api';
import type { Finding, ScanReport, SystemOverview } from './types';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

const overview: SystemOverview = {
  cpu: { usagePercent: 12, logicalCores: 8 },
  memory: { totalBytes: 100, usedBytes: 50, usagePercent: 50 },
  disks: [],
  osName: 'Windows 11',
  osVersion: '24H2',
};

const finding: Finding = {
  path: 'C:\\Users\\Slava\\Downloads\\a.pdf.exe',
  signature: 'Win.Test.X',
};

const report: ScanReport = {
  scannedFiles: 3,
  infectedFiles: 0,
  errors: 0,
  skippedFiles: 0,
  findings: [],
};

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
    invokeMock.mockResolvedValue({ available: false, reason: 'движок не собран' });
    await expect(clamavStatus()).resolves.toEqual({
      available: false,
      reason: 'движок не собран',
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

describe('runtime shape checks', () => {
  it('accepts well-formed backend answers', () => {
    expect(isSystemOverview(overview)).toBe(true);
    expect(isScanReport(report)).toBe(true);
    expect(isEngineStatus({ available: true, reason: '' })).toBe(true);
  });

  it('rejects a disk without a known kind or an overview without the OS fields', () => {
    expect(
      isSystemOverview({
        ...overview,
        disks: [
          {
            displayName: 'C:',
            mountPoint: 'C:\\',
            totalBytes: 1,
            usedBytes: 1,
            usagePercent: 1,
            kind: 'nvme',
          },
        ],
      }),
    ).toBe(false);
    expect(isSystemOverview({ ...overview, osName: undefined })).toBe(false);
  });

  it('rejects snake_case or missing fields', () => {
    expect(isSystemOverview({ ...overview, cpu: { usage_percent: 1, logical_cores: 2 } })).toBe(
      false,
    );
    expect(isScanReport({ scannedFiles: 1, infectedFiles: 0, errors: 0 })).toBe(false);
    expect(isEngineStatus({ available: 'yes', reason: '' })).toBe(false);
    expect(isSystemOverview(null)).toBe(false);
  });

  it('rejects a malformed finding or disk inside an otherwise valid answer', () => {
    expect(isScanReport({ ...report, findings: [{ path: 'C:\\a.exe' }] })).toBe(false);
    expect(isSystemOverview({ ...overview, disks: [{ displayName: 'C:' }] })).toBe(false);
  });

  it('rejects the command with a probe error when the answer has the wrong shape', async () => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ unexpected: true });
    const caught = await getSystemOverview().catch((err: unknown) => err);
    expect(toCommandError(caught)).toEqual({
      kind: 'probe',
      message: 'Ответ команды get_system_overview имеет неожиданную форму',
    });
  });

  it('passes a scan report with findings through', async () => {
    invokeMock.mockReset();
    const withFindings: ScanReport = {
      scannedFiles: 5,
      infectedFiles: 1,
      errors: 0,
      skippedFiles: 0,
      findings: [finding],
    };
    invokeMock.mockResolvedValue(withFindings);
    await expect(scanStart('C:\\x')).resolves.toEqual(withFindings);
  });
});

/** Имена полей типа из файла, который сгенерировал ts-rs из DTO на Rust. */
function generatedKeys(typeName: string): string[] {
  const text = readFileSync(join(process.cwd(), 'src/lib/generated', `${typeName}.ts`), 'utf8');
  const body = text.slice(text.indexOf('= {') + 3, text.lastIndexOf('}'));
  return [...body.matchAll(/(?:^|,|\n)\s*(\w+):/g)].map((m) => m[1]).sort();
}

describe('fixtures match the Rust-generated types', () => {
  it('uses exactly the generated field names', () => {
    expect(Object.keys(overview).sort()).toEqual(generatedKeys('SystemOverview'));
    expect(Object.keys(overview.cpu).sort()).toEqual(generatedKeys('CpuStatus'));
    expect(Object.keys(overview.memory).sort()).toEqual(generatedKeys('MemoryStatus'));
    expect(Object.keys(report).sort()).toEqual(generatedKeys('ScanReport'));
    expect(Object.keys(finding).sort()).toEqual(generatedKeys('Finding'));
    expect(Object.keys({ available: true, reason: '' }).sort()).toEqual(
      generatedKeys('EngineStatus'),
    );
    const disk = {
      displayName: 'C:',
      mountPoint: 'C:\\',
      totalBytes: 1,
      usedBytes: 1,
      usagePercent: 100,
      kind: 'ssd',
    };
    expect(Object.keys(disk).sort()).toEqual(generatedKeys('DiskStatus'));
    expect(Object.keys(overview).sort()).toContain('osName');
    expect(Object.keys(overview).sort()).toContain('osVersion');
  });
});
