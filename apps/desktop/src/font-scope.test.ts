import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import App from './App.svelte';
import appSource from './App.svelte?raw';
import { BODY_FONT } from './lib/tokens';

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
 * FND-0021: переменная --font-body задаётся на .shell (style:--font-body), а пользовательские
 * свойства CSS наследуются только вниз. Правило, читающее var(--font-body), стоявшее на body
 * (предок .shell), никогда её не видело, и BODY_FONT не действовал.
 *
 * Как и в focus-ring.test.ts, CSS компонента разбирается из исходника (?raw): jsdom не
 * применяет стили Svelte-компонентов, поэтому getComputedStyle здесь бесполезен.
 */

interface Rule {
  selector: string;
  body: string;
}

const STYLE_START = appSource.indexOf('<style');
if (STYLE_START < 0) {
  throw new Error('<style> block not found in App.svelte');
}
const MARKUP = appSource.slice(0, STYLE_START);
const STYLE_MATCH = /<style[^>]*>([\s\S]*?)<\/style>/.exec(appSource);
if (!STYLE_MATCH) {
  throw new Error('<style> block is not closed in App.svelte');
}
/** Текст стилей без комментариев: их содержимое не должно засчитываться как объявления. */
const CSS = STYLE_MATCH[1].replace(/\/\*[\s\S]*?\*\//g, '');

/** Снимает обёртку :global( ... ), оставляя внутренний селектор (с учётом вложенных скобок). */
function unwrapGlobal(selector: string): string {
  const marker = ':global(';
  let out = '';
  let i = 0;
  while (i < selector.length) {
    if (selector.startsWith(marker, i)) {
      let depth = 1;
      let j = i + marker.length;
      let inner = '';
      while (j < selector.length && depth > 0) {
        const ch = selector[j];
        if (ch === '(') depth++;
        if (ch === ')') depth--;
        if (depth > 0) inner += ch;
        j++;
      }
      out += inner;
      i = j;
    } else {
      out += selector[i];
      i++;
    }
  }
  return out;
}

/** Делит список селекторов по запятым верхнего уровня (запятые внутри скобок не считаются). */
function splitSelectors(list: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let current = '';
  for (const ch of list) {
    if (ch === '(') depth++;
    if (ch === ')') depth--;
    if (ch === ',' && depth === 0) {
      parts.push(current.trim());
      current = '';
    } else {
      current += ch;
    }
  }
  if (current.trim()) parts.push(current.trim());
  return parts;
}

/** Плоские правила `селектор { тело }`. Вложенных блоков в App.svelte нет. */
function parseRules(css: string): Rule[] {
  const rules: Rule[] = [];
  const re = /([^{}]+)\{([^{}]*)\}/g;
  let match: RegExpExecArray | null;
  while ((match = re.exec(css)) !== null) {
    rules.push({ selector: match[1].trim(), body: match[2] });
  }
  return rules;
}

/** Селекторы правила после снятия :global(), раздельно по запятым. */
function selectorParts(rule: Rule): string[] {
  return splitSelectors(unwrapGlobal(rule.selector));
}

/** Правило адресует body, html или :root (последний составной селектор в цепочке). */
function targetsDocumentRoot(rule: Rule): boolean {
  return selectorParts(rule).some((part) => {
    const compounds = part.split(/[\s>+~]+/).filter(Boolean);
    const last = compounds[compounds.length - 1] ?? '';
    return /^(body|html|:root)(?![\w-])/.test(last);
  });
}

function targetsShell(rule: Rule): boolean {
  return selectorParts(rule).includes('.shell');
}

/** Имена пользовательских свойств, которые правило читает через var(--name ...). */
function varsRead(rule: Rule): string[] {
  return [...rule.body.matchAll(/var\(\s*(--[\w-]+)/g)].map((m) => m[1]);
}

/** Имена свойств, задаваемых на .shell через style:--name={...} в разметке. */
function shellInlineVars(): string[] {
  const shellTag = /<div\s+class="shell"([^>]*(?:\{[^}]*\}[^>]*)*)>/.exec(MARKUP);
  if (!shellTag) {
    throw new Error('.shell element not found in App.svelte markup');
  }
  return [...shellTag[1].matchAll(/style:(--[\w-]+)\s*=/g)].map((m) => m[1]);
}

const RULES = parseRules(CSS);

describe('FND-0021: --font-body is read on the element that defines it, not on an ancestor', () => {
  it('FND-0021: the .shell rule declares font-family using var(--font-body', () => {
    const shellRules = RULES.filter(targetsShell);
    expect(shellRules.length, '.shell rule not found in App.svelte').toBeGreaterThan(0);
    const fontFamilies = shellRules
      .map((rule) => /(^|[;\s])font-family\s*:\s*([^;]+)/.exec(rule.body)?.[2] ?? '')
      .filter(Boolean);
    expect(fontFamilies.length, '.shell has no font-family declaration').toBeGreaterThan(0);
    expect(fontFamilies.some((value) => value.includes('var(--font-body'))).toBe(true);
  });

  it('FND-0021: no body/html/:root rule reads var(--font-body (it can never reach them)', () => {
    const rootRules = RULES.filter(targetsDocumentRoot);
    // Защита от пустой проверки: правило для body в App.svelte существует.
    expect(rootRules.length, 'no body/html/:root rule parsed - parser is broken').toBeGreaterThan(
      0,
    );
    for (const rule of rootRules) {
      expect(rule.body, `rule "${rule.selector}" reads --font-body`).not.toContain(
        'var(--font-body',
      );
    }
  });

  it('FND-0021: at runtime the .shell element has --font-body set to BODY_FONT', () => {
    const { container } = render(App);
    const shell = container.querySelector('.shell') as HTMLElement | null;
    expect(shell).not.toBeNull();
    expect(BODY_FONT.length).toBeGreaterThan(0);
    expect(shell!.style.getPropertyValue('--font-body')).toBe(BODY_FONT);
  });

  it('FND-0021: no body/html/:root rule reads any variable that is set on .shell (whole bug class)', () => {
    const shellVars = shellInlineVars();
    // Защита от пустой проверки: список переменных .shell разобран и содержит --font-body.
    expect(shellVars).toContain('--font-body');
    expect(shellVars.length).toBeGreaterThan(5);

    const rootRules = RULES.filter(targetsDocumentRoot);
    expect(rootRules.length).toBeGreaterThan(0);

    const offenders = rootRules.flatMap((rule) =>
      varsRead(rule)
        .filter((name) => shellVars.includes(name))
        .map((name) => `${rule.selector} reads ${name}`),
    );
    expect(offenders).toEqual([]);
  });
});
