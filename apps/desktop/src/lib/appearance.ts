import { SECTION_COLORS } from './tokens';
import type { SectionId } from './sections';
import { WALLPAPER_PHOTOS } from './appearanceWallpapers';

/**
 * Варианты оформления: фото владельца (превью), сплошные цвета, SVG-курсоры и цвета.
 * Данные портированы из старого приложения. Применение к Windows не подключено.
 */

export const APPEARANCE_KINDS = ['wallpaper', 'cursor', 'color'] as const;
export type AppearanceKind = (typeof APPEARANCE_KINDS)[number];

export interface AppearanceTile {
  id: string;
  label: string;
  description: string;
  /** Значение для фона превью: CSS-градиент или цвет. */
  preview: string;
  /** Адрес превью-картинки (обои-фото). */
  thumb?: string;
}

/** Пары «основной цвет, второй цвет» из старого приложения (AppearanceArtwork.cs), только данные. */
export const ACID_PAIRS: readonly (readonly [string, string])[] = [
  ['#B4FF00', '#FF009D'],
  ['#FF009D', '#00FFFF'],
  ['#087FFF', '#F5FF00'],
  ['#FF7A00', '#CF00FF'],
  ['#F5FF00', '#087FFF'],
  ['#CF00FF', '#B4FF00'],
  ['#00FFF0', '#FF009D'],
  ['#FF244D', '#F5FF00'],
  ['#B4FF00', '#087FFF'],
  ['#FF009D', '#F5FF00'],
  ['#087FFF', '#00FFF0'],
  ['#FF7A00', '#B4FF00'],
  ['#F5FF00', '#FF009D'],
  ['#CF00FF', '#00FFFF'],
  ['#8AFF00', '#FF6600'],
  ['#FF00DD', '#087FFF'],
  ['#00C8FF', '#F5FF00'],
  ['#FF5E00', '#00FFF0'],
  ['#B400FF', '#FF009D'],
  ['#FFEE00', '#CF00FF'],
  ['#00FF7F', '#001BFF'],
  ['#E6FF00', '#FF1493'],
  ['#00F5FF', '#7A00FF'],
  ['#FF3864', '#00FFD5'],
  ['#6DFF00', '#FF00A8'],
  ['#FF6B00', '#00A2FF'],
  ['#D0FF00', '#00FF95'],
  ['#9D00FF', '#FFEA00'],
  ['#00FFCC', '#FF3D00'],
  ['#FF008C', '#8AFF00'],
];

const pad2 = (n: number) => String(n).padStart(2, '0');

/** Сплошные обои старого приложения: каждая третья пара и два «узора» (чёрный, белый). */
const SOLID_WALLPAPERS: readonly { id: string; label: string; color: string }[] = [
  ...ACID_PAIRS.filter((_, i) => i % 3 === 0).map(([color], k) => ({
    id: `solid-acid-${pad2(k * 3 + 1)}`,
    label: `Цвет ${color}`,
    color,
  })),
  { id: 'solid-black', label: 'Чёрный', color: '#080808' },
  { id: 'solid-white', label: 'Белый', color: '#FFFFFF' },
];

export const WALLPAPERS: readonly AppearanceTile[] = [
  ...SOLID_WALLPAPERS.map((s) => ({
    id: s.id,
    label: s.label,
    description: `Сплошной цвет ${s.color}`,
    preview: s.color,
  })),
  ...WALLPAPER_PHOTOS.map((photo) => ({
    id: photo.id,
    label: photo.title,
    description: `${photo.title}, ${photo.width}×${photo.height}`,
    preview: '#161616',
    thumb: photo.thumb,
  })),
];

export type CursorPattern = 'plain' | 'stripes' | 'dots' | 'checker' | 'cross';

/** Описание курсора: цвета и узор заливки. Рисуется как SVG-стрелка, файлов нет. */
export interface CursorDef {
  color: string;
  secondary: string;
  pattern: CursorPattern;
}

/** Однотонные курсоры из старого приложения, в порядке экрана. */
const PLAIN_CURSORS: readonly { id: string; label: string; color: string }[] = [
  { id: 'cursor-black', label: 'Чёрный', color: '#000000' },
  { id: 'cursor-white', label: 'Белый', color: '#FFFFFF' },
  { id: 'cursor-gray', label: 'Серый', color: '#808080' },
  { id: 'cursor-red', label: 'Красный', color: '#FF0000' },
  { id: 'cursor-blood', label: 'Бордовый', color: '#860000' },
  { id: 'cursor-lime', label: 'Лаймовый', color: '#8AFF00' },
  { id: 'cursor-blue', label: 'Синий', color: '#0066FF' },
  { id: 'cursor-purple', label: 'Фиолетовый', color: '#B400FF' },
  { id: 'cursor-yellow', label: 'Жёлтый', color: '#FFF000' },
  { id: 'cursor-orange', label: 'Оранжевый', color: '#FF7300' },
  { id: 'cursor-pink', label: 'Розовый', color: '#FF00B8' },
  { id: 'cursor-cyan', label: 'Бирюзовый', color: '#00FFFF' },
];

/** Узор по номеру группы из четырёх пар: 1 полосы, 2 точки, 3 шахматка, дальше сетка. */
const PATTERN_BY_GROUP: readonly CursorPattern[] = ['plain', 'stripes', 'dots', 'checker'];
const PATTERN_NAMES: Record<CursorPattern, string> = {
  plain: 'однотонный',
  stripes: 'полосы',
  dots: 'точки',
  checker: 'шахматка',
  cross: 'сетка',
};

/** 35 курсоров: 12 однотонных и 23 узорчатых (пары 5–27 старой таблицы). */
export const CURSOR_DEFS: readonly (AppearanceTile & { cursor: CursorDef })[] = [
  ...PLAIN_CURSORS.map((c) => ({
    id: c.id,
    label: c.label,
    description: `Курсор «${c.label.toLowerCase()}»`,
    preview: '#161616',
    cursor: { color: c.color, secondary: c.color, pattern: 'plain' as CursorPattern },
  })),
  ...ACID_PAIRS.slice(4, 27).map(([color, secondary], k) => {
    const index = k + 4;
    const pattern = PATTERN_BY_GROUP[Math.floor(index / 4)] ?? 'cross';
    return {
      id: `cursor-acid-${pad2(index + 1)}`,
      label: `${PATTERN_NAMES[pattern]} ${color}`,
      description: `Курсор: ${PATTERN_NAMES[pattern]}, ${color} и ${secondary}`,
      preview: '#161616',
      cursor: { color, secondary, pattern },
    };
  }),
];

export const CURSORS: readonly AppearanceTile[] = CURSOR_DEFS;

const ARROW_PATH =
  'M12,7 Q10,6 10,10 L10,48 Q10,51 13,49 L23,40 L31,57 Q33,60 36,58 L41,55 Q43,54 41,51 L32,35 L46,34 Q50,34 47,31 L14,8 Q13,7 12,7 Z';

/** SVG-стрелка с белой обводкой. `uid` делает id узора уникальным на странице. Данные только свои. */
export function cursorSvg(def: CursorDef, uid: string): string {
  const { color, secondary, pattern } = def;
  const fill = pattern === 'plain' ? color : `url(#p-${uid})`;
  let defs = '';
  if (pattern !== 'plain') {
    let body = `<rect width="16" height="16" fill="${color}"/>`;
    if (pattern === 'stripes') {
      body += `<path d="M-4,16 L16,-4 M4,20 L20,4" stroke="${secondary}" stroke-width="6"/>`;
    } else if (pattern === 'dots') {
      body += `<circle cx="4" cy="4" r="3" fill="${secondary}"/><circle cx="12" cy="12" r="2" fill="#fff"/>`;
    } else if (pattern === 'checker') {
      body += `<rect width="8" height="8" fill="${secondary}"/><rect x="8" y="8" width="8" height="8" fill="${secondary}"/>`;
    } else {
      body += `<path d="M0,8 H16" stroke="${secondary}" stroke-width="5"/><path d="M8,0 V16" stroke="#fff" stroke-width="2"/>`;
    }
    defs = `<defs><pattern id="p-${uid}" width="16" height="16" patternUnits="userSpaceOnUse">${body}</pattern></defs>`;
  }
  return (
    `<svg viewBox="0 0 64 64" width="64" height="64" aria-hidden="true" focusable="false">${defs}` +
    `<path d="${ARROW_PATH}" fill="#fff" stroke="#fff" stroke-width="5" stroke-linejoin="round"/>` +
    `<path d="${ARROW_PATH}" fill="${fill}" stroke="#000" stroke-width="2.4" stroke-linejoin="round"/></svg>`
  );
}

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
const SECTION_SCHEMES: readonly AppearanceTile[] = (Object.keys(SECTION_COLORS) as SectionId[]).map(
  (id) => ({
    id,
    label: COLOR_NAMES[id],
    description: `Цвет раздела «${COLOR_NAMES[id].toLowerCase()}»`,
    preview: SECTION_COLORS[id],
  }),
);

/** Цветовые плитки старого приложения: основной цвет и второй, по диагонали. */
const ACID_SCHEMES: readonly AppearanceTile[] = ACID_PAIRS.map(([color, secondary], i) => ({
  id: `acid-${pad2(i + 1)}`,
  label: `Пара ${pad2(i + 1)}`,
  description: `Цвета ${color} и ${secondary}`,
  preview: `linear-gradient(135deg, ${color} 55%, ${secondary} 55%)`,
}));

export const COLOR_SCHEMES: readonly AppearanceTile[] = [...SECTION_SCHEMES, ...ACID_SCHEMES];

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
  return kind === 'cursor' ? 'cursor-white' : tilesFor(kind)[0].id;
}
