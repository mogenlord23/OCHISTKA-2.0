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
  /** Цвет страницы раздела из палитры оформления. */
  accent: string;
  /** Короткая строка под заголовком. */
  lead: string;
  subsections: readonly Subsection[];
}

export const SECTIONS: readonly Section[] = [
  {
    id: 'vpn',
    title: 'VPN',
    accent: '#ff009d',
    lead: 'Твоё подключение. Твои правила.',
    subsections: [
      { id: 'subscription', title: 'Подписка и импорт' },
      { id: 'servers', title: 'Серверы' },
      { id: 'routes', title: 'Маршруты и исключения' },
    ],
  },
  {
    id: 'viruses',
    title: 'Вирусы',
    accent: '#b4ff00',
    lead: 'Сначала проверка. Решение об удалении принимаешь ты.',
    subsections: [
      { id: 'scan', title: 'Проверка' },
      { id: 'findings', title: 'Находки' },
      { id: 'log', title: 'Журнал проверки' },
    ],
  },
  {
    id: 'processes',
    title: 'Процессы',
    accent: '#147cff',
    lead: 'Что запущено и сколько ресурсов занимает прямо сейчас.',
    subsections: [
      { id: 'processes', title: 'Процессы' },
      { id: 'services', title: 'Службы' },
    ],
  },
  { id: 'programs', title: 'Программы', accent: '#f5ff00', lead: '', subsections: [] },
  { id: 'disks', title: 'Диски', accent: '#ff7a00', lead: '', subsections: [] },
  {
    id: 'computer',
    title: 'Компьютер',
    accent: '#cf00ff',
    lead: 'Комплектующие, текущая нагрузка, настройки, Boost и драйверы.',
    subsections: [
      { id: 'components', title: 'Комплектующие и графики' },
      { id: 'settings', title: 'Настройки' },
      { id: 'boost', title: 'Boost' },
      { id: 'drivers', title: 'Драйверы' },
    ],
  },
  { id: 'design', title: 'Оформление', accent: '#00e5c3', lead: '', subsections: [] },
];
