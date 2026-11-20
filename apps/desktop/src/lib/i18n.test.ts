import { describe, expect, it } from 'vitest';
import { DICTS, KEYS, i18n, t } from './i18n.svelte';

describe('i18n', () => {
  it('has the same keys in Russian and English', () => {
    expect(Object.keys(DICTS.en).sort()).toEqual(Object.keys(DICTS.ru).sort());
  });

  it('returns Russian by default and English after switching', () => {
    i18n.lang = 'ru';
    expect(t('section.viruses')).toBe('Вирусы');
    i18n.lang = 'en';
    expect(t('section.viruses')).toBe('Viruses');
    i18n.lang = 'ru';
  });

  it('every key has a non-empty value in both languages', () => {
    for (const key of KEYS) {
      expect(DICTS.ru[key]).not.toBe('');
      expect(DICTS.en[key]).not.toBe('');
    }
  });
});
