import { describe, expect, it } from 'vitest';
import { formatBytes, formatPercent } from './format';

describe('formatBytes', () => {
  it('formats zero and small values in bytes', () => {
    expect(formatBytes(0)).toBe('0 Б');
    expect(formatBytes(500)).toBe('500 Б');
  });

  it('uses one decimal for kilobytes and above', () => {
    expect(formatBytes(1536)).toBe('1.5 КБ');
    expect(formatBytes(1073741824)).toBe('1.0 ГБ');
  });
});

describe('formatPercent', () => {
  it('rounds to a whole percent', () => {
    expect(formatPercent(12.4)).toBe('12 %');
  });
});
