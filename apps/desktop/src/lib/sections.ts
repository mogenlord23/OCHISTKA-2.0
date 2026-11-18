import { SECTION_COLORS } from './tokens';
import type { Key } from './i18n.svelte';

/** Семь разделов приложения, по порядку правой панели. Подразделы — вкладки на странице. */
export type SectionId =
  'viruses' | 'vpn' | 'processes' | 'programs' | 'disks' | 'computer' | 'design';

export interface Subsection {
  id: string;
  /** Русский текст: данные для тестов и запасной вариант. На экране — t(titleKey). */
  title: string;
  titleKey: Key;
}

export interface Section {
  id: SectionId;
  /** Русский текст: данные для тестов и запасной вариант. На экране — t(titleKey). */
  title: string;
  titleKey: Key;
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
    titleKey: 'section.viruses',
    accent: SECTION_COLORS.viruses,
    lead: 'Сначала проверка. Решение об удалении принимаешь ты.',
    subsections: [
      { id: 'scan', title: 'Проверка', titleKey: 'sub.viruses.scan' },
      { id: 'findings', title: 'Находки', titleKey: 'sub.viruses.findings' },
      { id: 'log', title: 'Журнал проверки', titleKey: 'sub.viruses.log' },
    ],
  },
  {
    id: 'vpn',
    title: 'VPN',
    titleKey: 'section.vpn',
    accent: SECTION_COLORS.vpn,
    lead: 'Твоё подключение. Твои правила.',
    subsections: [
      { id: 'subscription', title: 'Подписка и импорт', titleKey: 'sub.vpn.subscription' },
      { id: 'servers', title: 'Серверы', titleKey: 'sub.vpn.servers' },
      { id: 'routes', title: 'Маршруты и исключения', titleKey: 'sub.vpn.routes' },
    ],
  },
  {
    id: 'processes',
    title: 'Процессы',
    titleKey: 'section.processes',
    accent: SECTION_COLORS.processes,
    lead: 'Что запущено и сколько ресурсов занимает прямо сейчас.',
    subsections: [
      { id: 'processes', title: 'Процессы', titleKey: 'sub.processes.processes' },
      { id: 'services', title: 'Службы', titleKey: 'sub.processes.services' },
    ],
  },
  {
    id: 'programs',
    title: 'Программы',
    titleKey: 'section.programs',
    accent: SECTION_COLORS.programs,
    lead: 'Что установлено и что запускается вместе с Windows. Ничего не удаляем.',
    subsections: [
      { id: 'installed', title: 'Установленные', titleKey: 'sub.programs.installed' },
      { id: 'startup', title: 'Автозагрузка', titleKey: 'sub.programs.startup' },
    ],
  },
  {
    id: 'disks',
    title: 'Диски',
    titleKey: 'section.disks',
    accent: SECTION_COLORS.disks,
    lead: 'Что занимает место и что можно убрать. Удаляем только через карантин.',
    subsections: [
      { id: 'cleanup', title: 'Глубокая очистка', titleKey: 'sub.disks.cleanup' },
      { id: 'files', title: 'Очистка файлов', titleKey: 'sub.disks.files' },
      { id: 'space', title: 'Анализ пространства', titleKey: 'sub.disks.space' },
      { id: 'privacy', title: 'Приватность', titleKey: 'sub.disks.privacy' },
    ],
  },
  {
    id: 'computer',
    title: 'Компьютер',
    titleKey: 'section.computer',
    accent: SECTION_COLORS.computer,
    lead: 'Комплектующие, текущая нагрузка, настройки, Boost и драйверы.',
    subsections: [
      { id: 'components', title: 'Комплектующие и графики', titleKey: 'sub.computer.components' },
      { id: 'settings', title: 'Настройки', titleKey: 'sub.computer.settings' },
      { id: 'boost', title: 'Boost', titleKey: 'sub.computer.boost' },
      { id: 'drivers', title: 'Драйверы', titleKey: 'sub.computer.drivers' },
    ],
  },
  {
    id: 'design',
    title: 'Оформление',
    titleKey: 'section.design',
    accent: SECTION_COLORS.design,
    lead: '',
    subsections: [
      { id: 'wallpapers', title: 'Обои', titleKey: 'sub.design.wallpapers' },
      { id: 'cursors', title: 'Курсоры', titleKey: 'sub.design.cursors' },
      { id: 'colors', title: 'Цвета', titleKey: 'sub.design.colors' },
    ],
  },
];
