import { describe, expect, it } from 'vitest';
import {
  commandExePath,
  filterPrograms,
  kindLabel,
  measureBlockReason,
  sortPrograms,
  startupActionLabel,
} from './programs';
import { nextSort } from './sortState';
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
    id: 'google chrome|120.0|google llc',
    iconPath: null,
    canUninstall: true,
    uninstallReason: null,
  },
  {
    name: '7-Zip',
    publisher: 'Igor Pavlov',
    version: '23.01',
    sizeBytes: 5 * 1024 * 1024,
    installLocation: null,
    diskLetter: null,
    kind: 'win32',
    id: '7-zip|23.01|igor pavlov',
    iconPath: null,
    canUninstall: false,
    uninstallReason: 'В реестре нет команды удаления',
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

describe('sortPrograms', () => {
  const unknown = { ...PROGRAMS[0], name: 'No Size', id: 'x', sizeBytes: null };
  const small = { ...PROGRAMS[0], name: 'Small', id: 'y', sizeBytes: 10 };
  const big = { ...PROGRAMS[0], name: 'Big', id: 'z', sizeBytes: 9000 };
  const list = [unknown, small, big];

  it('sorts by name by default and by name on click', () => {
    expect(sortPrograms(list, null).map((p) => p.name)).toEqual(['Big', 'No Size', 'Small']);
  });

  it('puts the largest size first on the first click and the smallest on the second', () => {
    const first = nextSort(null, 'size' as const, 'desc');
    expect(sortPrograms(list, first).map((p) => p.name)).toEqual(['Big', 'Small', 'No Size']);
    const second = nextSort(first, 'size' as const, 'desc');
    expect(sortPrograms(list, second).map((p) => p.name)).toEqual(['Small', 'Big', 'No Size']);
  });

  it('keeps unknown values last for text columns too', () => {
    const noPublisher = { ...PROGRAMS[0], name: 'A', id: 'a', publisher: null };
    const sorted = sortPrograms([noPublisher, PROGRAMS[1]], { key: 'publisher', dir: 'desc' });
    expect(sorted.map((p) => p.name)).toEqual(['7-Zip', 'A']);
  });
});

describe('commandExePath and startupActionLabel', () => {
  it('finds the exe in quoted and unquoted commands', () => {
    expect(commandExePath('"C:\\Program Files\\A\\a.exe" --tray')).toBe(
      'C:\\Program Files\\A\\a.exe',
    );
    expect(commandExePath('C:\\A B\\a.exe /s')).toBe('C:\\A B\\a.exe');
    expect(commandExePath('C:\\A\\a.exe')).toBe('C:\\A\\a.exe');
    expect(commandExePath('cmd /c start')).toBeNull();
    expect(commandExePath(null)).toBeNull();
  });

  it('names the toggle action by the current state', () => {
    expect(startupActionLabel(true)).toBe('Отключить автозагрузку');
    expect(startupActionLabel(false)).toBe('Включить автозагрузку');
  });
});
