import { SECTION_COLORS } from './tokens';

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
    id: 'viruses',
    title: 'Вирусы',
    accent: SECTION_COLORS.viruses,
    lead: 'Сначала проверка. Решение об удалении принимаешь ты.',
    subsections: [
      { id: 'scan', title: 'Проверка' },
      { id: 'findings', title: 'Находки' },
      { id: 'log', title: 'Журнал проверки' },
    ],
  },
  {
    id: 'vpn',
    title: 'VPN',
    accent: SECTION_COLORS.vpn,
    lead: 'Твоё подключение. Твои правила.',
    subsections: [
      { id: 'subscription', title: 'Подписка и импорт' },
      { id: 'servers', title: 'Серверы' },
      { id: 'routes', title: 'Маршруты и исключения' },
    ],
  },
  {
    id: 'processes',
    title: 'Процессы',
    accent: SECTION_COLORS.processes,
    lead: 'Что запущено и сколько ресурсов занимает прямо сейчас.',
    subsections: [
      { id: 'processes', title: 'Процессы' },
      { id: 'services', title: 'Службы' },
    ],
  },
  {
    id: 'programs',
    title: 'Программы',
    accent: SECTION_COLORS.programs,
    lead: '',
    subsections: [],
  },
  {
    id: 'disks',
    title: 'Диски',
    accent: SECTION_COLORS.disks,
    lead: 'Что занимает место и что можно убрать. Удаляем только через карантин.',
    subsections: [
      { id: 'cleanup', title: 'Глубокая очистка' },
      { id: 'files', title: 'Очистка файлов' },
      { id: 'space', title: 'Анализ пространства' },
      { id: 'privacy', title: 'Приватность' },
    ],
  },
  {
    id: 'computer',
    title: 'Компьютер',
    accent: SECTION_COLORS.computer,
    lead: 'Комплектующие, текущая нагрузка, настройки, Boost и драйверы.',
    subsections: [
      { id: 'components', title: 'Комплектующие и графики' },
      { id: 'settings', title: 'Настройки' },
      { id: 'boost', title: 'Boost' },
      { id: 'drivers', title: 'Драйверы' },
    ],
  },
  { id: 'design', title: 'Оформление', accent: SECTION_COLORS.design, lead: '', subsections: [] },
];
