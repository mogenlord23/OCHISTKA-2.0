/** Семь разделов приложения, по порядку правой панели. Подразделы — вкладки на странице. */
export type SectionId =
  'viruses' | 'vpn' | 'processes' | 'programs' | 'disks' | 'computer' | 'design';

export interface Subsection {
  id: string;
  title: string;
}

export interface Section {
  id: SectionId;
  title: string;
  /** Цвет раздела из палитры оформления. */
  accent: string;
  subsections: readonly Subsection[];
}

export const SECTIONS: readonly Section[] = [
  {
    id: 'viruses',
    title: 'Вирусы',
    accent: '#b4ff00',
    subsections: [
      { id: 'scan', title: 'Проверка' },
      { id: 'findings', title: 'Находки' },
      { id: 'log', title: 'Журнал проверки' },
    ],
  },
  {
    id: 'vpn',
    title: 'VPN',
    accent: '#ff009d',
    subsections: [
      { id: 'subscription', title: 'Подписка и импорт' },
      { id: 'servers', title: 'Серверы' },
      { id: 'routes', title: 'Маршруты и исключения' },
    ],
  },
  {
    id: 'processes',
    title: 'Процессы',
    accent: '#147cff',
    subsections: [
      { id: 'processes', title: 'Процессы' },
      { id: 'services', title: 'Службы' },
    ],
  },
  { id: 'programs', title: 'Программы', accent: '#f5ff00', subsections: [] },
  { id: 'disks', title: 'Диски', accent: '#ff7a00', subsections: [] },
  {
    id: 'computer',
    title: 'Компьютер',
    accent: '#cf00ff',
    subsections: [
      { id: 'components', title: 'Комплектующие и графики' },
      { id: 'settings', title: 'Настройки' },
      { id: 'boost', title: 'Boost' },
      { id: 'drivers', title: 'Драйверы' },
    ],
  },
  { id: 'design', title: 'Оформление', accent: '#00e5c3', subsections: [] },
];
