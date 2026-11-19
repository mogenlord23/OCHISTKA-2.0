import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

/**
 * Правило владельца: одно скругление во всём приложении, значения только из токенов
 * (--radius-xs/sm/md/lg/pill в tokens.ts). Свои числа в стилях компонентов запрещены.
 * Разрешены: токены, 0, inherit, 50% и 9999px (круги).
 * Иконки SVG (атрибут rx) это геометрия рисунка, не стили, их тест не проверяет.
 */
const SRC = join(__dirname);

/** Файлы, которые ещё переносят на токены (владельцы блоков B и C). Список только сокращается. */
const PENDING = new Set<string>([]);
const PENDING_PREFIX: string[] = [];

function walk(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) walk(full, out);
    else if (name.endsWith('.svelte')) out.push(full);
  }
  return out;
}

const ALLOWED_PART = /^(var\(--radius-(xs|sm|md|lg|pill)\)|0|inherit|50%|9999px)$/;

export function rawRadii(css: string): string[] {
  const bad: string[] = [];
  const re = /border(?:-(?:top|bottom|start|end)-(?:left|right|start|end))?-radius\s*:\s*([^;}]+)/g;
  for (const m of css.matchAll(re)) {
    const parts = m[1]
      .replace(/\s*\/\s*/g, ' ')
      .trim()
      .split(/\s+/);
    if (!parts.every((p) => ALLOWED_PART.test(p))) bad.push(m[0].trim());
  }
  return bad;
}

describe('скругления только из токенов', () => {
  it('распознаёт свои числа', () => {
    expect(rawRadii('a{border-radius: 10px;}')).toEqual(['border-radius: 10px']);
    expect(rawRadii('a{border-top-left-radius: 6px}')).toHaveLength(1);
    expect(rawRadii('a{border-radius: 0 0 var(--radius-lg) var(--radius-lg);}')).toEqual([]);
    expect(rawRadii('a{border-radius: 50%;} b{border-radius: 9999px}')).toEqual([]);
    expect(rawRadii('a{border-radius: var(--radius-sm, 8px);}')).toHaveLength(1);
  });

  const files = walk(SRC).filter((f) => {
    const name = f.split(/[\\/]/).pop() ?? '';
    return !PENDING.has(name) && !PENDING_PREFIX.some((p) => name.startsWith(p));
  });

  it('проверяет компоненты', () => {
    expect(files.length).toBeGreaterThan(10);
  });

  for (const file of files) {
    it(file.slice(SRC.length + 1), () => {
      expect(rawRadii(readFileSync(file, 'utf8'))).toEqual([]);
    });
  }
});
