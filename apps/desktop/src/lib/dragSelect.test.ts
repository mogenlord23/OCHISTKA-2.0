import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { dragSelect, hitIndex, type DragSelectSource } from './dragSelect';

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

const WIDTH = 100;

/** Ряд из n пунктов шириной 100 px, высотой 40; рамка ряда на 10 px шире с каждой стороны. */
function setup(n = 4, axis: 'x' | 'y' = 'x') {
  const bar = document.createElement('div');
  const items: HTMLElement[] = [];
  for (let i = 0; i < n; i++) {
    const b = document.createElement('button');
    b.textContent = String(i);
    bar.appendChild(b);
    items.push(b);
    const lo = 10 + i * WIDTH;
    b.getBoundingClientRect = () =>
      (axis === 'x'
        ? { left: lo, right: lo + WIDTH, top: 10, bottom: 50, width: WIDTH, height: 40 }
        : {
            left: 10,
            right: 110,
            top: lo,
            bottom: lo + WIDTH,
            width: 100,
            height: WIDTH,
          }) as DOMRect;
  }
  const total = 20 + n * WIDTH;
  bar.getBoundingClientRect = () =>
    (axis === 'x'
      ? { left: 0, right: total, top: 0, bottom: 60 }
      : { left: 0, right: 120, top: 0, bottom: total }) as DOMRect;
  document.body.appendChild(bar);
  const calls: Array<[number, DragSelectSource]> = [];
  const action = dragSelect(bar, {
    axis,
    items: () => items,
    onSelect: (i, s) => calls.push([i, s]),
  });
  const at = (i: number) => 10 + i * WIDTH + WIDTH / 2;
  const fire = (el: Element, type: string, x: number, y: number, buttons: number, button = 0) =>
    el.dispatchEvent(
      new PointerEvent(type, {
        bubbles: true,
        cancelable: true,
        clientX: x,
        clientY: y,
        buttons,
        button,
        pointerId: 1,
      }),
    );
  return { bar, items, calls, action, at, fire };
}

afterEach(() => {
  document.body.innerHTML = '';
  document.body.className = '';
});

describe('dragSelect', () => {
  it('press selects at once, click without moving gives one selection', () => {
    const s = setup();
    s.fire(s.items[2], 'pointerdown', s.at(2), 30, 1);
    s.fire(s.bar, 'pointerup', s.at(2), 30, 0);
    expect(s.calls).toEqual([[2, 'press']]);
  });

  it('dragging across items switches once per item entered, in order', () => {
    const s = setup();
    s.fire(s.items[0], 'pointerdown', s.at(0), 30, 1);
    for (let i = 1; i < 4; i++) s.fire(s.bar, 'pointermove', s.at(i), 30, 1);
    s.fire(s.bar, 'pointerup', s.at(3), 30, 0);
    expect(s.calls.map((c) => c[0])).toEqual([0, 1, 2, 3]);
    expect(s.calls.slice(1).every((c) => c[1] === 'drag')).toBe(true);
  });

  it('many moves inside one item give one switch', () => {
    const s = setup();
    s.fire(s.items[0], 'pointerdown', s.at(0), 30, 1);
    for (let k = 0; k < 30; k++) s.fire(s.bar, 'pointermove', 115 + k, 30, 1);
    expect(s.calls.map((c) => c[0])).toEqual([0, 1]);
  });

  it('hover without a pressed button never switches', () => {
    const s = setup();
    for (let i = 0; i < 4; i++) s.fire(s.bar, 'pointermove', s.at(i), 30, 0);
    expect(s.calls).toEqual([]);
  });

  it('moves outside the bar never switch, and the drag ends after release', () => {
    const s = setup();
    s.fire(s.items[0], 'pointerdown', s.at(0), 30, 1);
    s.fire(s.bar, 'pointermove', s.at(2), 200, 1);
    s.fire(s.bar, 'pointermove', 900, 30, 1);
    s.fire(s.bar, 'pointerup', 0, 0, 0);
    s.fire(s.bar, 'pointermove', s.at(3), 30, 1);
    expect(s.calls.map((c) => c[0])).toEqual([0]);
  });

  it('a move with the button already released ends the drag', () => {
    const s = setup();
    s.fire(s.items[0], 'pointerdown', s.at(0), 30, 1);
    s.fire(s.bar, 'pointermove', s.at(1), 30, 0);
    expect(s.calls.map((c) => c[0])).toEqual([0]);
    expect(document.body.classList.contains('dragging')).toBe(false);
  });

  it('marks the body while dragging and clears it on cancel', () => {
    const s = setup();
    s.fire(s.items[1], 'pointerdown', s.at(1), 30, 1);
    expect(document.body.classList.contains('dragging')).toBe(true);
    s.fire(s.bar, 'pointercancel', 0, 0, 0);
    expect(document.body.classList.contains('dragging')).toBe(false);
  });

  it('ignores the right button', () => {
    const s = setup();
    s.fire(s.items[1], 'pointerdown', s.at(1), 30, 2, 2);
    expect(s.calls).toEqual([]);
  });

  it('keyboard click (detail 0) selects, mouse click (detail 1) does not repeat', () => {
    const s = setup();
    s.items[3].click();
    s.items[1].dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 1 }));
    expect(s.calls).toEqual([[3, 'key']]);
  });

  it('measures rects once on press, not on every move', () => {
    const s = setup();
    const spy = vi.spyOn(s.items[1], 'getBoundingClientRect');
    s.fire(s.items[0], 'pointerdown', s.at(0), 30, 1);
    const before = spy.mock.calls.length;
    for (let k = 0; k < 20; k++) s.fire(s.bar, 'pointermove', s.at(1) + k, 30, 1);
    expect(spy.mock.calls.length).toBe(before);
  });

  it('vertical axis: drags down and up, gap between tiles keeps the current one', () => {
    const s = setup(3, 'y');
    const y = (i: number) => 10 + i * WIDTH + WIDTH / 2;
    s.fire(s.items[0], 'pointerdown', 60, y(0), 1);
    s.fire(s.bar, 'pointermove', 60, y(1), 1);
    s.fire(s.bar, 'pointermove', 60, y(2), 1);
    s.fire(s.bar, 'pointermove', 60, y(1), 1);
    expect(s.calls.map((c) => c[0])).toEqual([0, 1, 2, 1]);
  });

  it('hitIndex: nearest tab in a horizontal gap, -1 outside', () => {
    const box = { left: 0, right: 100, top: 0, bottom: 50 };
    const rects = [
      { left: 5, right: 40, top: 5, bottom: 45 },
      { left: 50, right: 95, top: 5, bottom: 45 },
    ];
    expect(hitIndex('x', box, rects, 44, 20)).toBe(0);
    expect(hitIndex('x', box, rects, 48, 20)).toBe(1);
    expect(hitIndex('x', box, rects, 120, 20)).toBe(-1);
  });
});
