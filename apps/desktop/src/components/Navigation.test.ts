import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
import Navigation from './Navigation.svelte';
import source from './Navigation.svelte?raw';
import tauriConf from '../../src-tauri/tauri.conf.json';
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

  it('keeps a fixed 88px square tile that does not shrink with the window', () => {
    expect(source).toMatch(/--tile:\s*88px;/);
    expect(source).not.toMatch(/clamp\(/);
    expect(source).not.toMatch(/@media/);
    expect(source).toMatch(/width:\s*var\(--tile\)/);
    expect(source).toMatch(/height:\s*var\(--tile\)/);
  });

  it('window minimum height fits title bar offset, 7 tiles and the collapsed dock', () => {
    // 12 (верх) + 7 * 88 + 6 * 8 (промежутки) = 676; ниже 8 + стрелка 20 + отступ 8.
    const rail = 12 + 7 * 88 + 6 * 8;
    const dockCollapsed = 8 + 20 + 8;
    const win = tauriConf.app.windows[0];
    expect(win.minHeight).toBeGreaterThanOrEqual(rail + dockCollapsed);
    expect(win.height).toBeGreaterThanOrEqual(win.minHeight);
  });

  it('has a sliding indicator behind the tiles and transparent active tile', () => {
    expect(source).toMatch(/class="ind"/);
    expect(source).toMatch(/\.item\.active\s*\{\s*background:\s*transparent;/);
    render(Navigation, { active: 'processes', onSelect: () => {} });
    const nav = screen.getByRole('navigation', { name: 'Навигация' });
    expect(nav.querySelector('.ind')).toBeTruthy();
    expect(nav.querySelectorAll('[aria-current="page"]')).toHaveLength(1);
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
