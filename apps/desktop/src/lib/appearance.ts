import { WALLPAPER_PHOTOS } from './appearanceWallpapers';

/**
 * Варианты оформления: фото владельца (превью), сплошные цвета, SVG-курсоры и цвета.
 * Данные портированы из старого приложения. Выбор плитки применяется к Windows через команды ядра.
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

/** Описание курсора: цвета и узор заливки. Тот же рисунок Rust превращает в `.cur`. */
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

/** Контур стрелки в сетке 32×32: те же точки и центровка, что в `appearance_cursor.rs`. */
const ARROW_BASE: readonly (readonly [number, number])[] = [
  [0, 0],
  [0, 16.5],
  [3.8, 12.8],
  [6.6, 19.4],
  [9.2, 18.3],
  [6.4, 11.8],
  [11.6, 11.8],
];

/** Контур по центру холста 32×32; остриё (первая точка) — горячая точка курсора. */
export const ARROW_POINTS: readonly (readonly [number, number])[] = (() => {
  const scaled = ARROW_BASE.map(([x, y]) => [x * 1.15, y * 1.15] as const);
  const xs = scaled.map((p) => p[0]);
  const ys = scaled.map((p) => p[1]);
  const dx = (32 - (Math.max(...xs) - Math.min(...xs))) / 2 - Math.min(...xs);
  const dy = (32 - (Math.max(...ys) - Math.min(...ys))) / 2 - Math.min(...ys);
  return scaled.map(([x, y]) => [x + dx, y + dy] as const);
})();

/**
 * SVG-стрелка 32×32: белая внешняя линия, чёрная тонкая линия, заливка с узором. Это тот же
 * рисунок, что Rust кладёт в `.cur`. `uid` делает id узора уникальным на странице.
 */
export function cursorSvg(def: CursorDef, uid: string, size = 64): string {
  const { color, secondary, pattern } = def;
  const points = ARROW_POINTS.map(([x, y]) => `${x.toFixed(2)},${y.toFixed(2)}`).join(' ');
  const fill = pattern === 'plain' ? color : `url(#p-${uid})`;
  let defs = '';
  if (pattern !== 'plain') {
    let body = `<rect width="8" height="8" fill="${color}"/>`;
    let transform = '';
    if (pattern === 'stripes') {
      transform = ' patternTransform="rotate(-45)"';
      body += `<rect width="8" height="3" fill="${secondary}"/>`;
    } else if (pattern === 'dots') {
      body += `<circle cx="2" cy="2" r="1.5" fill="${secondary}"/><circle cx="6" cy="6" r="1" fill="#fff"/>`;
    } else if (pattern === 'checker') {
      body += `<rect width="4" height="4" fill="${secondary}"/><rect x="4" y="4" width="4" height="4" fill="${secondary}"/>`;
    } else {
      body += `<rect y="2.75" width="8" height="2.5" fill="${secondary}"/><rect x="3.5" width="1" height="8" fill="#fff"/>`;
    }
    defs = `<defs><pattern id="p-${uid}" width="8" height="8" patternUnits="userSpaceOnUse"${transform}>${body}</pattern></defs>`;
  }
  return (
    `<svg viewBox="0 0 32 32" width="${size}" height="${size}" aria-hidden="true" focusable="false">${defs}` +
    `<polygon points="${points}" fill="#fff" stroke="#fff" stroke-width="4" stroke-linejoin="round"/>` +
    `<polygon points="${points}" fill="${fill}" stroke="#000" stroke-width="1.4" stroke-linejoin="round"/></svg>`
  );
}

/** Акцентные цвета Windows: 53 плитки в порядке экрана владельца (9 в ряду). */
export const ACCENT_COLORS: readonly string[] = [
  '#B4FF00',
  '#FF009D',
  '#087FFF',
  '#FF7A00',
  '#F5FF00',
  '#CF00FF',
  '#00FFF0',
  '#FF244D',
  '#8AFF00',
  '#FF00DD',
  '#00C8FF',
  '#FF5E00',
  '#B400FF',
  '#FFEE00',
  '#00FF7F',
  '#E6FF00',
  '#00F5FF',
  '#FF3864',
  '#6DFF00',
  '#FF6B00',
  '#D0FF00',
  '#9D00FF',
  '#00FFCC',
  '#FF008C',
  '#1FFF5C',
  '#00D9A0',
  '#22D3FF',
  '#5A3CFF',
  '#8A5CFF',
  '#E63CFF',
  '#FF5CC8',
  '#FF4818',
  '#FFA800',
  '#B83CFF',
  '#E0FF00',
  '#FF3050',
  '#00FFB4',
  '#00A8FF',
  '#FF0070',
  '#FF9A00',
  '#78FF00',
  '#000000',
  '#FFFFFF',
  '#808080',
  '#BEBEBE',
  '#333333',
  '#D9BFA6',
  '#8B4513',
  '#003366',
  '#008080',
  '#800000',
  '#008000',
  '#4B0082',
];

export const COLOR_SCHEMES: readonly AppearanceTile[] = ACCENT_COLORS.map((hex, i) => ({
  id: `color-${pad2(i + 1)}`,
  label: `Цвет ${i + 1}`,
  description: `Цвет ${i + 1}`,
  preview: hex,
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
  return kind === 'cursor' ? 'cursor-white' : tilesFor(kind)[0].id;
}
