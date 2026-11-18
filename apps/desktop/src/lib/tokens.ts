import type { SectionId } from './sections';

/** Цвета разделов из дизайн-брифа (ориентиры; точные HEX снимаются со скриншотов). */
export const SECTION_COLORS: Record<SectionId, string> = {
  vpn: '#ff009d',
  viruses: '#b4ff00',
  processes: '#147cff',
  programs: '#ff7a00',
  disks: '#f5ff00',
  computer: '#cf00ff',
  design: '#f83c5c',
};

export const HEADING_FONT = "'Arial Black', 'Segoe UI', system-ui, sans-serif";
export const BODY_FONT = "'Segoe UI', system-ui, sans-serif";

function channel(value: number): number {
  const v = value / 255;
  return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
}

/** Относительная яркость цвета по WCAG 2.x. */
function luminance(hex: string): number {
  const n = parseInt(hex.slice(1), 16);
  const r = (n >> 16) & 255;
  const g = (n >> 8) & 255;
  const b = n & 255;
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

/** Контраст между двумя цветами по WCAG (от 1 до 21). */
export function contrastRatio(a: string, b: string): number {
  const la = luminance(a);
  const lb = luminance(b);
  const [light, dark] = la > lb ? [la, lb] : [lb, la];
  return (light + 0.05) / (dark + 0.05);
}

/** Текст страницы: чёрный или белый, тот, что читается лучше на этом фоне. */
export function textOn(background: string): string {
  const black = '#080808';
  const white = '#ffffff';
  return contrastRatio(black, background) >= contrastRatio(white, background) ? black : white;
}

/** Нейтральные цвета интерфейса. Значения совпадают с прежними литералами в компонентах. */
export const NEUTRAL = {
  black: '#080808',
  ink: '#fafafa',
  inkSoft: '#d4d4d4',
  muted: '#8b8b8b',
  surface: '#111111',
  surfaceAlt: '#111111',
  border: '#343434',
} as const;

/** Скругления: маленькое (кнопки), среднее (карточки), большое (панели). */
export const RADIUS = { sm: '8px', md: '12px', lg: '14px' } as const;

/** Размеры текста: подпись, мелкий, основной, ведущий, заголовок блока, раздел, дисплей. */
export const TYPE_SCALE = {
  caption: '11px',
  small: '12px',
  body: '13px',
  lead: '14px',
  title: '16px',
  section: '26px',
  display: '44px',
} as const;

/** Оболочка окна по эталону 0.5.5: почти чёрное окно, тёмные кнопки с тонкой рамкой. */
export const SHELL = {
  window: '#0b0b0b',
  button: '#111111',
  buttonBorder: '#2a2a2a',
  buttonHover: '#1c1c1c',
  railHover: '#1a1a1a',
} as const;
