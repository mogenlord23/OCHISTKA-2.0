import cursorShapes from '../../../../crates/ochistka-core/assets/cursor-shapes.json';
import solidWallpapers from '../../../../crates/ochistka-core/assets/solid-wallpapers.json';
import { WALLPAPER_PHOTOS } from './appearanceWallpapers';

/**
 * Варианты оформления: фото владельца (превью), SVG-курсоры и цвета Windows.
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

/** Сплошные обои в стиле приложения: Rust рисует PNG в родном разрешении экрана (тот же JSON). */
export const SOLID_WALLPAPERS: readonly AppearanceTile[] = (
  solidWallpapers as { id: string; label: string; color: string }[]
).map((item) => ({
  id: item.id,
  label: item.label,
  description: `Сплошной цвет, ${item.label.toLowerCase()}`,
  preview: item.color,
}));

/** Сначала сплошные обои (кислотные, чёрный, белый…), затем 146 фото владельца из gallery.json. */
export const WALLPAPERS: readonly AppearanceTile[] = [
  ...SOLID_WALLPAPERS,
  ...WALLPAPER_PHOTOS.map((photo) => ({
    id: photo.id,
    label: photo.title,
    description: `${photo.title}, ${photo.width}×${photo.height}`,
    preview: '#161616',
    thumb: photo.thumb,
  })),
];

export type CursorPattern =
  | 'plain'
  | 'stripes'
  | 'dots'
  | 'checker'
  | 'cross'
  | 'gradient'
  | 'split'
  | 'zigzag'
  | 'rings'
  | 'diamonds'
  | 'neon';

/** Стили заливки, добавленные к старым (в том же порядке, что `Pattern::ALL` в Rust). */
const NEW_PATTERNS: readonly CursorPattern[] = [
  'gradient',
  'split',
  'zigzag',
  'rings',
  'diamonds',
  'neon',
];

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
  gradient: 'градиент',
  split: 'диагональ',
  zigzag: 'зигзаг',
  rings: 'кольца',
  diamonds: 'ромбы',
  neon: 'неон',
};

/** 53 курсора: 35 из старого приложения (12 однотонных, 23 узорчатых) и 18 новых стилей. */
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
  ...NEW_PATTERNS.flatMap((pattern, i) =>
    [0, 1, 2].map((j) => {
      const index = (i * 4 + j * 10 + 1) % ACID_PAIRS.length;
      const [color, secondary] = ACID_PAIRS[index];
      return {
        id: `cursor-${pattern}-${j + 1}`,
        label: `${PATTERN_NAMES[pattern]} ${color}`,
        description: `Курсор: ${PATTERN_NAMES[pattern]}, ${color} и ${secondary}`,
        preview: '#161616',
        cursor: { color, secondary, pattern },
      };
    }),
  ),
];

export const CURSORS: readonly AppearanceTile[] = CURSOR_DEFS;

/** Путь стрелки в сетке 64×64: тот же файл читает Rust, когда рисует `.cur`. */
export const ARROW_PATH: string = cursorShapes.arrow;

/** Сдвиг, который ставит стрелку (рамка 10…47 × 6…60) в центр сетки 64×64. */
const ARROW_SHIFT = 'translate(3.5 -1)';

/** Содержимое `<defs>` с заливкой узора `id`; пустая строка для однотонного и неона. */
function paintDefs(pattern: CursorPattern, color: string, second: string, id: string): string {
  const tile = (w: number, h: number, body: string) =>
    `<pattern id="${id}" width="${w}" height="${h}" patternUnits="userSpaceOnUse">` +
    `<rect width="${w}" height="${h}" fill="${color}"/>${body}</pattern>`;
  switch (pattern) {
    case 'stripes':
      return tile(
        16,
        16,
        `<polygon points="5.1,0 14.9,0 0,14.9 0,5.1" fill="${second}"/>` +
          `<polygon points="5.1,16 16,5.1 16,14.9 14.9,16" fill="${second}"/>`,
      );
    case 'dots':
      return tile(
        16,
        16,
        `<circle cx="4" cy="4" r="3" fill="${second}"/><circle cx="12" cy="12" r="2" fill="#fff"/>`,
      );
    case 'checker':
      return tile(
        16,
        16,
        `<rect width="8" height="8" fill="${second}"/><rect x="8" y="8" width="8" height="8" fill="${second}"/>`,
      );
    case 'cross':
      return tile(
        16,
        16,
        `<rect y="5.5" width="16" height="5" fill="${second}"/><rect x="7" width="2" height="16" fill="#fff"/>`,
      );
    case 'zigzag':
      return tile(
        12,
        14,
        `<polygon points="0,-6 6,0 12,-6 12,-1 6,5 0,-1" fill="${second}"/>` +
          `<polygon points="0,8 6,14 12,8 12,13 6,19 0,13" fill="${second}"/>`,
      );
    case 'diamonds':
      return tile(16, 16, `<polygon points="8,3 13,8 8,13 3,8" fill="${second}"/>`);
    case 'gradient':
      return (
        `<linearGradient id="${id}" gradientUnits="userSpaceOnUse" x1="0" y1="6" x2="0" y2="58">` +
        `<stop offset="0" stop-color="${color}"/><stop offset="1" stop-color="${second}"/></linearGradient>`
      );
    case 'split':
      return (
        `<linearGradient id="${id}" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="64" y2="64">` +
        `<stop offset="0.453" stop-color="${color}"/><stop offset="0.453" stop-color="${second}"/></linearGradient>`
      );
    case 'rings':
      return (
        `<radialGradient id="${id}" gradientUnits="userSpaceOnUse" cx="10.5" cy="7" r="10" spreadMethod="repeat">` +
        `<stop offset="0.4" stop-color="${second}"/><stop offset="0.4" stop-color="${color}"/></radialGradient>`
      );
    default:
      return '';
  }
}

/**
 * SVG-стрелка 64×64: белая внешняя линия, тонкий контур, заливка с узором. Те же фигуры, шаг
 * узора и толщины линий, что Rust кладёт в `.cur`. `uid` делает id узора уникальным на странице.
 */
export function cursorSvg(def: CursorDef, uid: string, size = 64): string {
  const { color, secondary, pattern } = def;
  const id = `p-${uid}`;
  const neon = pattern === 'neon';
  const defs = paintDefs(pattern, color, secondary, id);
  const fill = neon ? '#121216' : defs ? `url(#${id})` : color;
  const stroke = neon ? color : '#000';
  return (
    `<svg viewBox="0 0 64 64" width="${size}" height="${size}" aria-hidden="true" focusable="false">` +
    (defs ? `<defs>${defs}</defs>` : '') +
    `<g transform="${ARROW_SHIFT}">` +
    `<path d="${ARROW_PATH}" fill="#fff" stroke="#fff" stroke-width="5" stroke-linejoin="round"/>` +
    `<path d="${ARROW_PATH}" fill="${fill}" stroke="${stroke}" stroke-width="2.4" stroke-linejoin="round"/>` +
    `</g></svg>`
  );
}

/** Акцентные цвета Windows: сначала кислотные, потом обычные, в конце белый, серые и чёрный. Без повторов и близнецов. */
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
  '#00FF7F',
  '#FF3864',
  '#6DFF00',
  '#D0FF00',
  '#9D00FF',
  '#00FFCC',
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
  '#00A8FF',
  '#FF0070',
  '#00E5FF',
  '#18FFFF',
  '#FF4081',
  '#651FFF',
  '#00FF41',
  '#FFD600',
  '#E53935',
  '#D81B60',
  '#8E24AA',
  '#5E35B1',
  '#3949AB',
  '#1E88E5',
  '#039BE5',
  '#00ACC1',
  '#00897B',
  '#43A047',
  '#7CB342',
  '#C0CA33',
  '#FDD835',
  '#6D4C41',
  '#8B4513',
  '#003366',
  '#800000',
  '#008000',
  '#4B0082',
  '#D9BFA6',
  '#FF8A80',
  '#2E7D32',
  '#0D47A1',
  '#B71C1C',
  '#006064',
  '#BCAAA4',
  '#455A64',
  '#FFFFFF',
  '#E0E0E0',
  '#BEBEBE',
  '#808080',
  '#2B2B2B',
  '#000000',
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
