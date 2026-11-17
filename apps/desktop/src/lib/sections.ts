/** Десять разделов из CLAUDE.md, в порядке левой навигации. */
export type SectionId =
  | 'home'
  | 'cleanup'
  | 'programs'
  | 'processes'
  | 'network'
  | 'security'
  | 'widgets'
  | 'optimization'
  | 'drivers'
  | 'settings';

export interface Section {
  id: SectionId;
  title: string;
}

export const SECTIONS: readonly Section[] = [
  { id: 'home', title: 'Главная' },
  { id: 'cleanup', title: 'Очистка' },
  { id: 'programs', title: 'Программы' },
  { id: 'processes', title: 'Процессы' },
  { id: 'network', title: 'Сеть' },
  { id: 'security', title: 'Безопасность' },
  { id: 'widgets', title: 'Виджеты' },
  { id: 'optimization', title: 'Оптимизация' },
  { id: 'drivers', title: 'Драйверы' },
  { id: 'settings', title: 'Настройки' },
];
