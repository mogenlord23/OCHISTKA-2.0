import { describe, expect, it } from 'vitest';
import { countryName, flagUrl, numberedLabels, serverLabel } from './countryFlags';

describe('flags and country names', () => {
  it('has a local flag for the supported codes and none for unknown ones', () => {
    for (const code of ['LV', 'DE', 'NL', 'FI', 'RU', 'US', 'uk', 'gb']) {
      expect(flagUrl(code), code).toBeTruthy();
    }
    expect(flagUrl('ZZ')).toBeNull();
    expect(flagUrl(null)).toBeNull();
    expect(flagUrl('UK')).toBe(flagUrl('GB'));
  });

  it('names countries in the interface language', () => {
    expect(countryName('LV', 'ru')).toBe('Латвия');
    expect(countryName('DE', 'ru')).toBe('Германия');
    expect(countryName('NL', 'ru')).toBe('Нидерланды');
    expect(countryName('FI', 'ru')).toBe('Финляндия');
    expect(countryName('LV', 'en')).toBe('Latvia');
    expect(countryName('ZZ', 'ru')).toBeNull();
    expect(countryName('EU', 'ru')).toBe('Европа');
  });

  it('does not repeat the flag or the country and keeps what tells servers apart', () => {
    expect(serverLabel('🇱🇻 Латвия', 'LV', 'ru')).toBe('Латвия');
    expect(serverLabel('Latvia 01', 'LV', 'ru')).toBe('Латвия');
    expect(serverLabel('🇩🇪 Germany - Frankfurt #2', 'DE', 'ru')).toBe('Германия');
    expect(serverLabel('[NL] Amsterdam', 'NL', 'ru')).toBe('Нидерланды');
    expect(serverLabel('FI-03', 'FI', 'en')).toBe('Finland');
    expect(serverLabel('Finland', 'FI', 'en')).toBe('Finland');
    expect(serverLabel('USA Dallas', 'US', 'ru')).toBe('США');
    expect(serverLabel('UK London', 'UK', 'ru')).toBe('Великобритания');
    // Без флага: код страны в начале не показываем, название остаётся.
    expect(serverLabel('ЕУ Европа', 'EU', 'ru')).toBe('Европа');
    expect(serverLabel('[XX] Hysteria', 'XX', 'ru')).toBe('Hysteria');
    expect(numberedLabels(['Нидерланды', 'Латвия', 'Нидерланды'])).toEqual(['Нидерланды', 'Латвия', 'Нидерланды 2']);
  });

  it('keeps the original name when the country is unknown', () => {
    expect(serverLabel('Mystery 7', null, 'ru')).toBe('Mystery 7');
    expect(serverLabel('Mystery 7', 'ZZ', 'ru')).toBe('Mystery 7');
  });
});

describe('флаги', () => {
  it('every server country has a flag, including Europe; an unknown code has none', () => {
    for (const code of ['EU', 'DE', 'LV', 'NL', 'US', 'GB', 'JP', 'BR', 'AE', 'XK', 'UN']) {
      expect(flagUrl(code), code).not.toBeNull();
    }
    expect(flagUrl('XX')).toBeNull();
    expect(flagUrl('ZZ')).toBeNull();
  });
});
