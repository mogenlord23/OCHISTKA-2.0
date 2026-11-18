import { describe, expect, it } from 'vitest';
import { filterPrograms, kindLabel, measureBlockReason } from './programs';
import type { InstalledProgram } from './programsApi';

const PROGRAMS: InstalledProgram[] = [
  {
    name: 'Google Chrome',
    publisher: 'Google LLC',
    version: '120.0',
    sizeBytes: null,
    installLocation: 'C:\\Program Files\\Google\\Chrome',
    diskLetter: 'C:',
    kind: 'win32',
  },
  {
    name: '7-Zip',
    publisher: 'Igor Pavlov',
    version: '23.01',
    sizeBytes: 5 * 1024 * 1024,
    installLocation: null,
    diskLetter: null,
    kind: 'win32',
  },
];

describe('filterPrograms', () => {
  it('returns everything for an empty or blank query', () => {
    expect(filterPrograms(PROGRAMS, '')).toHaveLength(2);
    expect(filterPrograms(PROGRAMS, '   ')).toHaveLength(2);
  });

  it('matches the name case-insensitively', () => {
    expect(filterPrograms(PROGRAMS, 'CHROME').map((item) => item.name)).toEqual(['Google Chrome']);
  });

  it('matches the publisher too', () => {
    expect(filterPrograms(PROGRAMS, 'pavlov').map((item) => item.name)).toEqual(['7-Zip']);
  });

  it('returns an empty list when nothing matches', () => {
    expect(filterPrograms(PROGRAMS, 'нет такой')).toEqual([]);
  });
});

describe('kindLabel and measureBlockReason', () => {
  it('labels the three program kinds', () => {
    expect(kindLabel('win32')).toBe('Win32');
    expect(kindLabel('store')).toBe('Store');
    expect(kindLabel('other')).toBe('Другое');
  });

  it('explains why a folder cannot be measured without an install location', () => {
    expect(measureBlockReason(PROGRAMS[0])).toBeNull();
    expect(measureBlockReason(PROGRAMS[1])).toBe('В реестре не указана папка установки');
  });
});
