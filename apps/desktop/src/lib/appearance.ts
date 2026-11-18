import { SECTION_COLORS } from './tokens';
import type { SectionId } from './sections';

/**
 * Встроенные варианты оформления. Это только данные: обоев-картинок нет (фон рисуется CSS),
 * курсоров-файлов нет (только названия), цвета берутся из палитры разделов.
 * Применение к Windows не подключено: см. AppearancePage.svelte.
 */

export const APPEARANCE_KINDS = ['wallpaper', 'cursor', 'color'] as const;
export type AppearanceKind = (typeof APPEARANCE_KINDS)[number];

export interface AppearanceTile {
  id: string;
  label: string;
  description: string;
  /** Значение для фона превью: CSS-градиент или цвет. */
  preview: string;
}

export const WALLPAPERS: readonly AppearanceTile[] = [
  {
    id: 'aurora',
    label: 'Аврора',
    description: 'Тёмно-синий переход в мятный',
    preview: 'linear-gradient(135deg, #0f2027, #2c5364 50%, #7fffd4)',
  },
  {
    id: 'sunset',
    label: 'Закат',
    description: 'Персиковый переход в фиолетовый',
    preview: 'linear-gradient(160deg, #ff7e5f, #feb47b 45%, #6a0572)',
  },
  {
    id: 'ocean',
    label: 'Океан',
    description: 'Два оттенка синего сверху вниз',
    preview: 'linear-gradient(180deg, #00c6ff, #0072ff)',
  },
  {
    id: 'graphite',
    label: 'Графит',
    description: 'Сдержанный серый переход',
    preview: 'linear-gradient(135deg, #232526, #414345)',
  },
  {
    id: 'mint',
    label: 'Мята',
    description: 'Светло-зелёный переход',
    preview: 'linear-gradient(120deg, #a8ff78, #78ffd6)',
  },
  {
    id: 'dusk',
    label: 'Сумерки',
    description: 'Сиреневый с коралловым бликом',
    preview: 'linear-gradient(140deg, #3a1c71, #d76d77 55%, #ffaf7b)',
  },
  {
    id: 'ember',
    label: 'Угли',
    description: 'Жёлтое свечение на тёмном фоне',
    preview: 'radial-gradient(circle at 30% 20%, #ffd200, #ff4e00 60%, #1a0500)',
  },
  {
    id: 'night',
    label: 'Ночь',
    description: 'Почти чёрный переход в синий',
    preview: 'linear-gradient(to bottom, #000428, #004e92)',
  },
];

/** Названия схем курсора. Файлов курсора нет: выбор только запоминается. */
export const CURSORS: readonly AppearanceTile[] = [
  {
    id: 'classic',
    label: 'Классический',
    description: 'Обычный указатель, стандартный размер',
    preview: '#1f1f1f',
  },
  {
    id: 'large',
    label: 'Крупный',
    description: 'Увеличенный указатель для лучшей видимости',
    preview: '#1f1f1f',
  },
  {
    id: 'inverted',
    label: 'Инверсный',
    description: 'Светлый контур на тёмном фоне',
    preview: '#1f1f1f',
  },
  {
    id: 'dark',
    label: 'Тёмный',
    description: 'Тёмный контур для светлых окон',
    preview: '#1f1f1f',
  },
];

const COLOR_NAMES: Record<SectionId, string> = {
  vpn: 'Розовый',
  viruses: 'Лаймовый',
  processes: 'Синий',
  programs: 'Оранжевый',
  disks: 'Жёлтый',
  computer: 'Фиолетовый',
  design: 'Коралловый',
};

/** Цвета страниц разделов из палитры tokens.ts. */
export const COLOR_SCHEMES: readonly AppearanceTile[] = (
  Object.keys(SECTION_COLORS) as SectionId[]
).map((id) => ({
  id,
  label: COLOR_NAMES[id],
  description: `Цвет раздела «${COLOR_NAMES[id].toLowerCase()}»`,
  preview: SECTION_COLORS[id],
}));

export function tilesFor(kind: AppearanceKind): readonly AppearanceTile[] {
  switch (kind) {
    case 'wallpaper':
      return WALLPAPERS;
    case 'cursor':
      return CURSORS;
    case 'color':
      return COLOR_SCHEMES;
  }
}

/** Первый вариант списка считается выбранным по умолчанию. */
export function defaultTileId(kind: AppearanceKind): string {
  return tilesFor(kind)[0].id;
}
