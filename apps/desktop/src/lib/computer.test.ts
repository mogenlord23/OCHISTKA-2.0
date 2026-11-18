import { describe, expect, it } from 'vitest';
import {
  boardName,
  DRIVERS_REASON,
  gpuMemoryLine,
  gpuName,
  gpuUsageLine,
  SYSTEM_CHANGE_REASON,
  UNSUPPORTED_OS_REASON,
} from './computer';

const GB = 1024 ** 3;

describe('computer section reasons', () => {
  it('gives the exact reason texts shown in the interface', () => {
    expect(SYSTEM_CHANGE_REASON).toBe(
      'Раздел в разработке: системные настройки меняют Windows, пока не подключено',
    );
    expect(UNSUPPORTED_OS_REASON).toBe('Нет поддержки этой ОС в текущей версии');
    expect(DRIVERS_REASON).toMatch(/^Раздел в разработке:/);
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
