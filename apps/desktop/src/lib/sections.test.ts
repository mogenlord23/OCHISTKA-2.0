import { describe, expect, it } from 'vitest';
import { SECTIONS } from './sections';

describe('SECTIONS', () => {
  it('lists ten sections in the order from CLAUDE.md', () => {
    expect(SECTIONS.map((section) => section.title)).toEqual([
      'Главная',
      'Очистка',
      'Программы',
      'Процессы',
      'Сеть',
      'Безопасность',
      'Виджеты',
      'Оптимизация',
      'Драйверы',
      'Настройки',
    ]);
  });

  it('has unique ids', () => {
    expect(new Set(SECTIONS.map((section) => section.id)).size).toBe(10);
  });
});
