/**
 * Какие CSS-курсоры (как на https://abacktools.com/ru/tools/design/utilities/css-cursor-tester)
 * подменяются схемой OCHISTKA в Windows, а какие рисует сам браузер или программа.
 *
 * - system: браузер на Windows берёт системный курсор этой роли, поэтому схема его заменяет;
 * - maybe: зависит от версии браузера (часть версий использует системную роль, часть свою картинку);
 * - browser: браузер рисует картинку сам, схема Windows на неё не влияет. Подмена потребовала бы
 *   вмешательства в чужие процессы, этого OCHISTKA не делает.
 *
 * Роли — значения `HKCU\Control Panel\Cursors`; все 17 ролей схемы рисуются в одном стиле.
 */
export type CursorSupport = 'system' | 'maybe' | 'browser';

export interface CssCursor {
  css: string;
  support: CursorSupport;
  /** Роль Windows для system/maybe. */
  role?: string;
}

const sys = (css: string, role: string): CssCursor => ({ css, support: 'system', role });
const maybe = (css: string, role: string): CssCursor => ({ css, support: 'maybe', role });
const own = (css: string): CssCursor => ({ css, support: 'browser' });

export const CSS_CURSORS: readonly CssCursor[] = [
  sys('default', 'Arrow'),
  sys('pointer', 'Hand'),
  sys('text', 'IBeam'),
  own('vertical-text'),
  sys('wait', 'Wait'),
  sys('progress', 'AppStarting'),
  sys('move', 'SizeAll'),
  sys('crosshair', 'Crosshair'),
  sys('help', 'Help'),
  sys('not-allowed', 'No'),
  sys('n-resize', 'SizeNS'),
  sys('s-resize', 'SizeNS'),
  sys('e-resize', 'SizeWE'),
  sys('w-resize', 'SizeWE'),
  sys('ne-resize', 'SizeNESW'),
  sys('sw-resize', 'SizeNESW'),
  sys('nw-resize', 'SizeNWSE'),
  sys('se-resize', 'SizeNWSE'),
  sys('ns-resize', 'SizeNS'),
  sys('ew-resize', 'SizeWE'),
  sys('nesw-resize', 'SizeNESW'),
  sys('nwse-resize', 'SizeNWSE'),
  maybe('col-resize', 'SizeWE'),
  maybe('row-resize', 'SizeNS'),
  own('cell'),
  own('zoom-in'),
  own('zoom-out'),
  own('grab'),
  own('grabbing'),
  own('copy'),
  own('alias'),
];
