import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
import Navigation from './Navigation.svelte';
import source from './Navigation.svelte?raw';
import { SECTIONS } from '../lib/sections';

class TestPointerEvent extends MouseEvent {
  pointerId: number;
  constructor(type: string, init: MouseEventInit & { pointerId?: number } = {}) {
    super(type, init);
    this.pointerId = init.pointerId ?? 1;
  }
}

beforeAll(() => {
  (globalThis as unknown as { PointerEvent: typeof TestPointerEvent }).PointerEvent =
    TestPointerEvent;
});
afterEach(() => cleanup());

describe('Navigation rail', () => {
  it('renders all seven tiles with a visible label each', () => {
    render(Navigation, { active: 'vpn', onSelect: () => {} });
    const nav = screen.getByRole('navigation', { name: 'Навигация' });
    const buttons = within(nav).getAllByRole('button');
    expect(buttons).toHaveLength(SECTIONS.length);
    for (const button of buttons) {
      expect(button.querySelector('.label')?.textContent?.trim()).toBeTruthy();
    }
  });

  it('gives the side rail and inter-tile gaps the same inset', () => {
    expect(source).toMatch(/--gap:\s*var\(--frame-inset, 14px\)/);
    expect(source).toMatch(/gap:\s*var\(--gap\)/);
    expect(source).toMatch(/padding:\s*var\(--gap\)/);
    expect(source).toMatch(/width:\s*var\(--tile\)/);
    expect(source).toMatch(/height:\s*var\(--tile\)/);
  });

  it('applies the selected state immediately to a single tile', () => {
    render(Navigation, { active: 'processes', onSelect: () => {} });
    const selected = screen.getByRole('button', { name: 'Процессы', current: 'page' });
    expect(selected.classList.contains('active')).toBe(true);
    expect(screen.getAllByRole('button', { current: 'page' })).toHaveLength(1);
    const nav = screen.getByRole('navigation');
    expect(nav.style.getPropertyValue('--active-index')).toBe('2');
  });

  it('selects on click and on press-and-drag across tiles, never on hover', async () => {
    const onSelect = vi.fn();
    render(Navigation, { active: 'vpn', onSelect });
    const nav = screen.getByRole('navigation', { name: 'Навигация' });
    const tiles = within(nav).getAllByRole('button');
    tiles.forEach((tile, i) => {
      tile.getBoundingClientRect = () =>
        ({ left: 0, right: 88, top: i * 96, bottom: i * 96 + 88 }) as DOMRect;
    });
    nav.getBoundingClientRect = () => ({ left: 0, right: 110, top: 0, bottom: 700 }) as DOMRect;
    const fire = (el: Element, type: string, y: number, buttons: number) =>
      el.dispatchEvent(
        new PointerEvent(type, {
          bubbles: true,
          cancelable: true,
          clientX: 40,
          clientY: y,
          buttons,
        }),
      );
    fire(nav, 'pointermove', 100, 0);
    expect(onSelect).not.toHaveBeenCalled();
    fire(tiles[0], 'pointerdown', 40, 1);
    fire(nav, 'pointermove', 96 + 40, 1);
    fire(nav, 'pointermove', 192 + 40, 1);
    fire(nav, 'pointerup', 232, 0);
    expect(onSelect.mock.calls.map((c) => c[0])).toEqual(['vpn', 'viruses', 'processes']);
    await fireEvent.click(tiles[4]);
    expect(onSelect).toHaveBeenLastCalledWith('disks');
  });
});
