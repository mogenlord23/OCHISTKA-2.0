import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import App from './App.svelte';
import appSource from './App.svelte?raw';
import { NEUTRAL, contrastRatio } from './lib/tokens';

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    minimize: vi.fn(),
    toggleMaximize: vi.fn(),
    close: vi.fn(),
  }),
}));

vi.mock('./lib/api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./lib/api')>()),
  getSystemOverview: vi.fn(() => new Promise(() => {})),
  clamavStatus: vi.fn(async () => ({ available: true, reason: '' })),
  defaultScanTarget: vi.fn(async () => 'C:\\Users\\Slava\\Downloads'),
}));

afterEach(() => cleanup());

/**
 * Почему разбирается исходник, а не getComputedStyle: vitest работает под jsdom, а jsdom не
 * применяет CSS компонентов Svelte (стили собираются Vite-плагином в отдельный файл и в тест
 * не попадают). Поэтому правило фокуса не видно через getComputedStyle, и единственный
 * надёжный способ проверить его (FND-0019) - прочитать текст App.svelte (?raw) и разобрать
 * нужный блок. Реальные цвета проверяются отдельно по токенам, а наличие переменной --ink
 * на корневом элементе - в отрисованном приложении.
 */
function focusRuleBlock(): string {
  const match =
    /:global\(\s*button:focus-visible\s*,\s*input:focus-visible\s*\)\s*\{([^}]*)\}/.exec(appSource);
  if (!match) {
    throw new Error('focus-visible rule for button/input not found in App.svelte');
  }
  return match[1];
}

/** Убирает CSS-комментарии, чтобы их текст не засчитывался как объявления. */
function declarations(block: string): string {
  return block.replace(/\/\*[\s\S]*?\*\//g, '');
}

describe('FND-0019: keyboard focus ring is visible on the near-black right rail', () => {
  it('FND-0019: the focus-visible rule declares an outline', () => {
    const css = declarations(focusRuleBlock());
    expect(css).toMatch(/(^|[;\s])outline\s*:\s*[^;]*\S/);
  });

  it('FND-0019: the focus-visible rule adds a light second ring via box-shadow using var(--ink)', () => {
    const css = declarations(focusRuleBlock());
    const shadow = /(^|[;\s])box-shadow\s*:\s*([^;]+);/.exec(css);
    expect(shadow, 'box-shadow declaration is missing').not.toBeNull();
    expect(shadow![2]).toContain('var(--ink)');
    // Кольцо сплошное: без размытия, с ненулевым растяжением (spread).
    expect(shadow![2]).toMatch(/^\s*0\s+0\s+0\s+[1-9]\d*px\s+var\(--ink\)\s*$/);
  });

  it('FND-0019: the inner ring colour has >= 3:1 contrast against the rail background (WCAG 1.4.11)', () => {
    expect(contrastRatio(NEUTRAL.ink, NEUTRAL.black)).toBeGreaterThanOrEqual(3);
  });

  it('FND-0019: the .shell element sets --ink to NEUTRAL.ink so the ring colour is defined', () => {
    const { container } = render(App);
    const shell = container.querySelector('.shell') as HTMLElement | null;
    expect(shell).not.toBeNull();
    expect(shell!.style.getPropertyValue('--ink')).toBe(NEUTRAL.ink);
  });
});
