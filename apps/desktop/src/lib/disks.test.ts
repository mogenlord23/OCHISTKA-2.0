import { afterEach, describe, expect, it } from 'vitest';
import { driveLetter, formatSize, kindLabel, localizeCategoryTitle, modelLabel, tf } from './disks';
import { i18n } from './i18n.svelte';

afterEach(() => {
  i18n.lang = 'ru';
});

describe('driveLetter', () => {
  it('turns a Windows mount point into a letter', () => {
    expect(driveLetter('C:\\')).toBe('C:');
    expect(driveLetter('d:/')).toBe('D:');
  });
  it('keeps other mount points as they are', () => {
    expect(driveLetter('/')).toBe('/');
    expect(driveLetter('/mnt/data')).toBe('/mnt/data');
  });
});

describe('kindLabel and modelLabel', () => {
  it('names the drive type', () => {
    expect(kindLabel('ssd')).toBe('SSD');
    expect(kindLabel('hdd')).toBe('HDD');
    expect(kindLabel('unknown')).toBe('Тип неизвестен');
    i18n.lang = 'en';
    expect(kindLabel('unknown')).toBe('Type unknown');
  });
  it('shows the short model or says it is unknown', () => {
    expect(modelLabel('Kingston NV2')).toBe('Kingston NV2');
    expect(modelLabel(null)).toBe('Модель неизвестна');
    expect(modelLabel('  ')).toBe('Модель неизвестна');
    i18n.lang = 'en';
    expect(modelLabel(null)).toBe('Model unknown');
  });
});

describe('formatSize', () => {
  it('uses Russian units by default and English units in EN', () => {
    expect(formatSize(1536)).toBe('1.5 КБ');
    i18n.lang = 'en';
    expect(formatSize(1536)).toBe('1.5 KB');
    expect(formatSize(10)).toBe('10 B');
  });
});

describe('tf', () => {
  it('substitutes placeholders', () => {
    expect(tf('disks.moved', { moved: 3, size: '2.0 КБ' })).toContain('3');
  });
});

describe('localizeCategoryTitle', () => {
  it('keeps backend text in Russian and translates known ids in English', () => {
    expect(localizeCategoryTitle('chrome-cache', 'Кэш Chrome')).toBe('Кэш Chrome');
    i18n.lang = 'en';
    expect(localizeCategoryTitle('chrome-cache', 'Кэш Chrome')).toBe('Chrome cache');
    expect(localizeCategoryTitle('firefox-cache-abc.default', 'Кэш Firefox (abc.default)')).toBe(
      'Firefox cache (abc.default)',
    );
    expect(localizeCategoryTitle('unknown-id', 'Что-то')).toBe('Что-то');
  });
});
