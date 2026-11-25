import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import iconSource from './ComputerIcon.svelte?raw';
import ComputerIcon, { type IconKind } from './ComputerIcon.svelte';

afterEach(cleanup);

const KINDS: IconKind[] = ['cpu', 'gpu', 'ram', 'board', 'disks', 'system'];

/** Рамка рисунка без фонового квадрата: объединение прямоугольников и кругов. */
function artBounds(svg: SVGElement) {
  const shapes = [...svg.querySelectorAll('rect, circle')].slice(1);
  let left = Infinity;
  let top = Infinity;
  let right = -Infinity;
  let bottom = -Infinity;
  for (const shape of shapes) {
    const n = (name: string) => Number(shape.getAttribute(name) ?? 0);
    const box =
      shape.tagName === 'circle'
        ? { x: n('cx') - n('r'), y: n('cy') - n('r'), w: 2 * n('r'), h: 2 * n('r') }
        : { x: n('x'), y: n('y'), w: n('width'), h: n('height') };
    left = Math.min(left, box.x);
    top = Math.min(top, box.y);
    right = Math.max(right, box.x + box.w);
    bottom = Math.max(bottom, box.y + box.h);
  }
  return { left, top, right, bottom };
}

describe('ComputerIcon', () => {
  it.each(KINDS)('%s: the drawing is centred exactly in the 88 px tile', (kind) => {
    const { container } = render(ComputerIcon, { kind });
    const { left, top, right, bottom } = artBounds(container.querySelector('svg') as SVGElement);
    expect((left + right) / 2).toBe(44);
    expect((top + bottom) / 2).toBe(44);
  });

  it('keeps one corner radius and size for every tile', () => {
    expect(iconSource).toMatch(/\.icon-art\s*\{[^}]*border-radius:\s*var\(--radius-adaptive\)/);
    for (const kind of KINDS) {
      const { container } = render(ComputerIcon, { kind });
      const tile = container.querySelector('svg.icon-art rect') as SVGRectElement;
      expect([
        tile.getAttribute('width'),
        tile.getAttribute('height'),
        tile.getAttribute('rx'),
      ]).toEqual(['88', '88', null]);
      cleanup();
    }
  });
});
