import { describe, expect, it } from 'vitest';
import {
  boardLine,
  boardName,
  clockTime,
  cpuName,
  detailPercent,
  drivesLine,
  gpuFullName,
  shortPercent,
  systemLine,
  ramTotal,
  gpuMemoryLine,
  gpuName,
  gpuUsageLine,
  UNSUPPORTED_OS_REASON,
} from './computer';

const GB = 1024 ** 3;

describe('computer section reasons', () => {
  it('gives the exact reason texts shown in the interface', () => {
    expect(UNSUPPORTED_OS_REASON).toBe('Нет поддержки этой ОС в текущей версии');
  });
});

describe('hardware labels', () => {
  const gpu = {
    vendor: 'NVIDIA',
    model: null,
    usagePercent: 12.4,
    totalMemoryBytes: 8 * GB,
    usedMemoryBytes: 2 * GB,
  };

  it('names a GPU by model, then vendor, then a dash', () => {
    expect(gpuName({ ...gpu, model: 'RTX 4070' })).toBe('RTX 4070');
    expect(gpuName(gpu)).toBe('NVIDIA');
    expect(gpuName({ ...gpu, vendor: null })).toBe('—');
  });

  it('shows usage and memory only when the OS reports them', () => {
    expect(gpuUsageLine(gpu)).toBe('Загрузка: 12 %');
    expect(gpuUsageLine({ ...gpu, usagePercent: null })).toBeNull();
    expect(gpuMemoryLine(gpu)).toBe('2.0 ГБ из 8.0 ГБ');
    expect(gpuMemoryLine({ ...gpu, usedMemoryBytes: null })).toBe('Память: 8.0 ГБ');
    expect(gpuMemoryLine({ ...gpu, totalMemoryBytes: null })).toBeNull();
  });

  it('names a board by model, then vendor, then a dash', () => {
    expect(boardName({ vendor: 'ASUS', name: 'ROG', version: null })).toBe('ROG');
    expect(boardName({ vendor: 'ASUS', name: null, version: null })).toBe('ASUS');
    expect(boardName({ vendor: null, name: null, version: null })).toBe('—');
  });
});

describe('tile labels', () => {
  it('formats percents the way the owner reference does', () => {
    expect(shortPercent(41.6)).toBe('42%');
    expect(detailPercent(0.26)).toBe('0,3%');
    expect(detailPercent(12)).toBe('12%');
    expect(clockTime(new Date(2026, 0, 1, 8, 5, 9))).toBe('08:05:09');
  });

  it('names the CPU, RAM and system without percentages', () => {
    expect(cpuName({ cpuBrand: 'Intel(R) Core(TM) i7-6700 CPU @ 3.40GHz' })).toContain('i7-6700');
    expect(cpuName(null)).toBe('Процессор');
    expect(ramTotal(24 * 1024 ** 3)).toBe('24,0 ГБ');
    expect(systemLine('Майкрософт Windows 11 Домашняя', '10.0.26200')).toBe(
      'Майкрософт Windows 11 Домашняя · 10.0.26200',
    );
    expect(systemLine('', '')).toBe('Система не определена');
  });

  it('joins disk models, or honestly falls back to kind and letter', () => {
    const disks = [
      { mountPoint: 'C:\\', kind: 'ssd' as const },
      { mountPoint: 'D:\\', kind: 'hdd' as const },
    ];
    expect(drivesLine(['KINGSTON SHFS37A120G', 'WDC WD10EZEX-21WN4A0'], disks)).toBe(
      'KINGSTON SHFS37A120G / WDC WD10EZEX-21WN4A0',
    );
    expect(drivesLine([], disks)).toBe('SSD · C: / HDD · D:');
    expect(drivesLine([], [])).toBe('Диски не найдены');
  });

  it('does not repeat the GPU vendor and joins the board', () => {
    const gpu = {
      vendor: 'NVIDIA',
      model: 'GeForce GTX 1060 3GB',
      usagePercent: null,
      totalMemoryBytes: null,
      usedMemoryBytes: null,
    };
    expect(gpuFullName(gpu)).toBe('NVIDIA GeForce GTX 1060 3GB');
    expect(gpuFullName({ ...gpu, model: 'NVIDIA GeForce GTX 1060' })).toBe(
      'NVIDIA GeForce GTX 1060',
    );
    expect(boardLine({ vendor: 'ASRock', name: 'Z170 Extreme4', version: null })).toBe(
      'ASRock · Z170 Extreme4',
    );
    expect(boardLine({ vendor: null, name: null, version: null })).toBe('—');
  });
});
