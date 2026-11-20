import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import Subtabs from './Subtabs.svelte';
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

const items = SECTIONS.find((s) => s.id === 'computer')!.subsections;

describe('Subtabs', () => {
  it('renders a tablist with one tab per subsection and roving tabindex', () => {
    render(Subtabs, { items, active: items[1].id, onSelect: () => {} });
    const list = screen.getByRole('tablist', { name: 'Подразделы' });
    const tabs = screen.getAllByRole('tab');
    expect(list).toBeTruthy();
    expect(tabs).toHaveLength(items.length);
    expect(tabs.map((t) => t.getAttribute('aria-selected'))).toEqual([
      'false',
      'true',
      'false',
      'false',
    ]);
    expect(tabs.map((t) => t.tabIndex)).toEqual([-1, 0, -1, -1]);
  });

  it('click and keyboard activation select the tab', async () => {
    const onSelect = vi.fn();
    render(Subtabs, { items, active: items[0].id, onSelect });
    await fireEvent.click(screen.getAllByRole('tab')[2]);
    expect(onSelect).toHaveBeenLastCalledWith(items[2].id);
  });

  it('arrow keys, Home and End move the selection', async () => {
    const onSelect = vi.fn();
    render(Subtabs, { items, active: items[0].id, onSelect });
    const first = screen.getAllByRole('tab')[0];
    await fireEvent.keyDown(first, { key: 'ArrowRight' });
    expect(onSelect).toHaveBeenLastCalledWith(items[1].id);
    await fireEvent.keyDown(first, { key: 'ArrowLeft' });
    expect(onSelect).toHaveBeenLastCalledWith(items[items.length - 1].id);
    await fireEvent.keyDown(first, { key: 'End' });
    expect(onSelect).toHaveBeenLastCalledWith(items[items.length - 1].id);
    await fireEvent.keyDown(first, { key: 'Home' });
    expect(onSelect).toHaveBeenLastCalledWith(items[0].id);
  });

  it('pressing selects at once and dragging with the button held selects the next tabs', () => {
    const onSelect = vi.fn();
    render(Subtabs, { items, active: items[0].id, onSelect });
    const list = screen.getByRole('tablist');
    const tabs = screen.getAllByRole('tab');
    tabs.forEach((tab, i) => {
      tab.getBoundingClientRect = () =>
        ({ left: i * 100, right: i * 100 + 98, top: 0, bottom: 38 }) as DOMRect;
    });
    list.getBoundingClientRect = () => ({ left: 0, right: 400, top: 0, bottom: 38 }) as DOMRect;
    const fire = (el: Element, type: string, x: number, buttons: number) =>
      el.dispatchEvent(
        new PointerEvent(type, {
          bubbles: true,
          cancelable: true,
          clientX: x,
          clientY: 10,
          buttons,
        }),
      );
    fire(tabs[0], 'pointerdown', 20, 1);
    fire(list, 'pointermove', 120, 1);
    fire(list, 'pointermove', 130, 1);
    fire(list, 'pointermove', 250, 1);
    fire(list, 'pointerup', 250, 0);
    fire(list, 'pointermove', 350, 0);
    expect(onSelect.mock.calls.map((c) => c[0])).toEqual([items[0].id, items[1].id, items[2].id]);
  });
});
