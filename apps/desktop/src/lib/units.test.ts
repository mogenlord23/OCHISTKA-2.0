import { describe, expect, it } from 'vitest';
import { formatCpuPercent, formatMemory, formatSize } from './units';

describe('formatSize', () => {
  it('uses a decimal comma and Russian units', () => {
    expect(formatSize(Math.round(11.9 * 1024 * 1024))).toBe('11,9 МБ');
    expect(formatSize(Math.round(1.2 * 1024 ** 3))).toBe('1,2 ГБ');
    expect(formatSize(2048)).toBe('2,0 КБ');
  });

  it('never shows kilo-sized raw bytes', () => {
    expect(formatSize(1023)).toBe('1023 Б');
    expect(formatSize(1024)).toBe('1,0 КБ');
    expect(formatSize(0)).toBe('0 Б');
    expect(formatSize(-1)).toBe('—');
  });
});

describe('formatMemory', () => {
  it('shows «Нет данных» for zero', () => {
    expect(formatMemory(0)).toBe('Нет данных');
    expect(formatMemory(1024 * 1024)).toBe('1,0 МБ');
  });
});

describe('formatCpuPercent', () => {
  it('shows hundredths without trailing zeros', () => {
    expect(formatCpuPercent(3.97)).toBe('3,97%');
    expect(formatCpuPercent(0.41)).toBe('0,41%');
    expect(formatCpuPercent(1.5)).toBe('1,5%');
    expect(formatCpuPercent(0)).toBe('0%');
    expect(formatCpuPercent(0.001)).toBe('0%');
  });
});
