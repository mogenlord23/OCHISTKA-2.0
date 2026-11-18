import { describe, expect, it } from 'vitest';
import { SECTIONS } from './sections';
import { DICTS } from './i18n.svelte';

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

  it('gives the programs section its installed and startup subsections', () => {
    const programs = SECTIONS.find((section) => section.id === 'programs');
    expect(programs?.subsections.map((item) => item.title)).toEqual([
      'Установленные',
      'Автозагрузка',
    ]);
  });

  it('gives the disks section its four subsections in order', () => {
    const disks = SECTIONS.find((section) => section.id === 'disks');
    expect(disks?.subsections.map((item) => item.title)).toEqual([
      'Глубокая очистка',
      'Очистка файлов',
      'Анализ пространства',
      'Приватность',
    ]);
  });

  it('gives the design section its wallpapers, cursors and colours subsections in order', () => {
    const design = SECTIONS.find((section) => section.id === 'design');
    expect(design?.subsections).toEqual([
      { id: 'wallpapers', title: 'Обои', titleKey: 'sub.design.wallpapers' },
      { id: 'cursors', title: 'Курсоры', titleKey: 'sub.design.cursors' },
      { id: 'colors', title: 'Цвета', titleKey: 'sub.design.colors' },
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

  it('gives every section and subsection a translation key present in Russian and English', () => {
    for (const section of SECTIONS) {
      expect(DICTS.ru[section.titleKey]).toBeTruthy();
      expect(DICTS.en[section.titleKey]).toBeTruthy();
      for (const sub of section.subsections) {
        expect(DICTS.ru[sub.titleKey]).toBeTruthy();
        expect(DICTS.en[sub.titleKey]).toBeTruthy();
      }
    }
  });
});
