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
    id: 'solid-lime',
    label: 'Лайм',
    description: 'Сплошной цвет #b4ff00',
    preview: 'linear-gradient(#b4ff00, #b4ff00)',
  },
  {
    id: 'solid-orange',
    label: 'Апельсин',
    description: 'Сплошной цвет #ff7a00',
    preview: 'linear-gradient(#ff7a00, #ff7a00)',
  },
  {
    id: 'solid-cyan',
    label: 'Бирюза',
    description: 'Сплошной цвет #00f5f0',
    preview: 'linear-gradient(#00f5f0, #00f5f0)',
  },
  {
    id: 'solid-magenta',
    label: 'Маджента',
    description: 'Сплошной цвет #ff009d',
    preview: 'linear-gradient(#ff009d, #ff009d)',
  },
  {
    id: 'solid-lemon',
    label: 'Лимон',
    description: 'Сплошной цвет #f5ff00',
    preview: 'linear-gradient(#f5ff00, #f5ff00)',
  },
  {
    id: 'solid-fuchsia',
    label: 'Фуксия',
    description: 'Сплошной цвет #f000ff',
    preview: 'linear-gradient(#f000ff, #f000ff)',
  },
  {
    id: 'solid-violet',
    label: 'Фиалка',
    description: 'Сплошной цвет #a000ff',
    preview: 'linear-gradient(#a000ff, #a000ff)',
  },
  {
    id: 'solid-chartreuse',
    label: 'Шартрез',
    description: 'Сплошной цвет #dcff00',
    preview: 'linear-gradient(#dcff00, #dcff00)',
  },
  {
    id: 'solid-green',
    label: 'Трава',
    description: 'Сплошной цвет #5cff00',
    preview: 'linear-gradient(#5cff00, #5cff00)',
  },
  {
    id: 'solid-indigo',
    label: 'Индиго',
    description: 'Сплошной цвет #8c00ff',
    preview: 'linear-gradient(#8c00ff, #8c00ff)',
  },
  {
    id: 'solid-black',
    label: 'Чёрный',
    description: 'Сплошной цвет #0a0a0a',
    preview: 'linear-gradient(#0a0a0a, #0a0a0a)',
  },
  {
    id: 'solid-white',
    label: 'Белый',
    description: 'Сплошной цвет #f7f7f7',
    preview: 'linear-gradient(#f7f7f7, #f7f7f7)',
  },
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
