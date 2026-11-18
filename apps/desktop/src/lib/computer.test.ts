import { describe, expect, it } from 'vitest';
import {
  BOARD_REASON,
  DRIVERS_REASON,
  GPU_REASON,
  SYSTEM_CHANGE_REASON,
  UNSUPPORTED_OS_REASON,
} from './computer';

describe('computer section reasons', () => {
  it('gives the exact reason texts shown in the interface', () => {
    expect(GPU_REASON).toBe('Чтение GPU ещё не реализовано');
    expect(BOARD_REASON).toBe('Нет данных: чтение платы не реализовано');
    expect(SYSTEM_CHANGE_REASON).toBe(
      'Раздел в разработке: системные настройки меняют Windows, пока не подключено',
    );
    expect(UNSUPPORTED_OS_REASON).toBe('Нет поддержки этой ОС в текущей версии');
    expect(DRIVERS_REASON).toMatch(/^Раздел в разработке:/);
  });
});
