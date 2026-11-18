import { describe, expect, it } from 'vitest';
import { SECTIONS } from './sections';

describe('SECTIONS', () => {
  it('lists seven sections with viruses first and VPN second', () => {
    expect(SECTIONS.map((section) => section.title)).toEqual([
      'Вирусы',
      'VPN',
      'Процессы',
      'Программы',
      'Диски',
      'Компьютер',
      'Оформление',
    ]);
  });

  it('gives the virus section its three subsections in order', () => {
    const viruses = SECTIONS.find((section) => section.id === 'viruses');
    expect(viruses?.subsections.map((item) => item.title)).toEqual([
      'Проверка',
      'Находки',
      'Журнал проверки',
    ]);
  });

  it('gives the VPN section its three subsections in order', () => {
    const vpn = SECTIONS.find((section) => section.id === 'vpn');
    expect(vpn?.subsections.map((item) => item.title)).toEqual([
      'Подписка и импорт',
      'Серверы',
      'Маршруты и исключения',
    ]);
  });

  it('gives the computer section its four subsections', () => {
    const computer = SECTIONS.find((section) => section.id === 'computer');
    expect(computer?.subsections.map((item) => item.title)).toEqual([
      'Комплектующие и графики',
      'Настройки',
      'Boost',
      'Драйверы',
    ]);
  });

  it('has a page colour and a lead string for every section', () => {
    for (const section of SECTIONS) {
      expect(section.accent).toMatch(/^#[0-9a-f]{6}$/i);
      expect(typeof section.lead).toBe('string');
    }
  });

  it('has unique section ids', () => {
    expect(new Set(SECTIONS.map((section) => section.id)).size).toBe(SECTIONS.length);
  });
});
