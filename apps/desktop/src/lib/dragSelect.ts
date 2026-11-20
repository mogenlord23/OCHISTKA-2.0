/**
 * Переключение зажатой мышью для рядов вкладок и правой панели (pointer events, не HTML5 Drag and Drop).
 *
 * Правила:
 * - нажатие на пункт выбирает его сразу;
 * - пока кнопка зажата, пункт под курсором выбирается один раз при входе в него;
 * - наведение без нажатой кнопки и движение за пределами панели ничего не переключают;
 * - границы пунктов считаются один раз при нажатии, а не на каждое движение мыши;
 * - клик с клавиатуры (detail 0) выбирает пункт, мышиный клик уже обработан при нажатии.
 */
export type DragSelectSource = 'press' | 'drag' | 'key';

export interface DragSelectOptions {
  /** Ось ряда: 'x' для вкладок сверху, 'y' для правой панели. */
  axis: 'x' | 'y';
  /** Текущие пункты по порядку (читается при нажатии и при клавиатурном клике). */
  items: () => readonly HTMLElement[];
  onSelect: (index: number, source: DragSelectSource) => void;
}

interface DragState {
  pointerId: number;
  last: number;
  box: DOMRect;
  rects: DOMRect[];
}

/** Индекс пункта под точкой; -1, если точка за панелью или в зазоре, где нет ближайшего пункта. */
export function hitIndex(
  axis: 'x' | 'y',
  box: Pick<DOMRect, 'left' | 'right' | 'top' | 'bottom'>,
  rects: readonly Pick<DOMRect, 'left' | 'right' | 'top' | 'bottom'>[],
  x: number,
  y: number,
): number {
  if (x < box.left || x > box.right || y < box.top || y > box.bottom) return -1;
  const v = axis === 'x' ? x : y;
  for (let i = 0; i < rects.length; i++) {
    const lo = axis === 'x' ? rects[i].left : rects[i].top;
    const hi = axis === 'x' ? rects[i].right : rects[i].bottom;
    if (v >= lo && v <= hi) return i;
  }
  if (axis === 'x') {
    // Зазор между вкладками: ближайшая по горизонтали.
    let best = -1;
    let distance = Infinity;
    rects.forEach((r, i) => {
      const d = Math.abs((r.left + r.right) / 2 - v);
      if (d < distance) {
        distance = d;
        best = i;
      }
    });
    return best;
  }
  return -1;
}

export function dragSelect(node: HTMLElement, initial: DragSelectOptions) {
  let options = initial;
  let drag: DragState | null = null;

  const indexOf = (target: EventTarget | null): number => {
    if (!(target instanceof Node)) return -1;
    return options.items().findIndex((el) => el.contains(target));
  };

  function finish(event: Event) {
    if (!drag) return;
    const id = (event as PointerEvent).pointerId ?? drag.pointerId;
    try {
      node.releasePointerCapture?.(id);
    } catch {
      /* захват уже снят */
    }
    drag = null;
    document.body.classList.remove('dragging');
  }

  function onDown(event: PointerEvent) {
    if (event.button !== 0) return;
    const index = indexOf(event.target);
    if (index < 0) return;
    // Без выделения текста и без встроенного перетаскивания.
    event.preventDefault();
    const items = options.items();
    drag = {
      pointerId: event.pointerId,
      last: index,
      box: node.getBoundingClientRect(),
      rects: items.map((el) => el.getBoundingClientRect()),
    };
    try {
      node.setPointerCapture?.(event.pointerId);
    } catch {
      /* без захвата ряд всё равно получает события пока курсор над ним */
    }
    document.body.classList.add('dragging');
    options.onSelect(index, 'press');
  }

  function onMove(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointerId) return;
    if (!(event.buttons & 1)) {
      finish(event);
      return;
    }
    const index = hitIndex(options.axis, drag.box, drag.rects, event.clientX, event.clientY);
    if (index < 0 || index === drag.last) return;
    drag.last = index;
    options.onSelect(index, 'drag');
  }

  function onClick(event: MouseEvent) {
    if (event.detail !== 0) return;
    const index = indexOf(event.target);
    if (index >= 0) options.onSelect(index, 'key');
  }

  const noNativeDrag = (event: Event) => event.preventDefault();

  node.addEventListener('pointerdown', onDown);
  node.addEventListener('pointermove', onMove);
  node.addEventListener('pointerup', finish);
  node.addEventListener('pointercancel', finish);
  node.addEventListener('lostpointercapture', finish);
  node.addEventListener('click', onClick);
  node.addEventListener('dragstart', noNativeDrag);

  return {
    update(next: DragSelectOptions) {
      options = next;
    },
    destroy() {
      node.removeEventListener('pointerdown', onDown);
      node.removeEventListener('pointermove', onMove);
      node.removeEventListener('pointerup', finish);
      node.removeEventListener('pointercancel', finish);
      node.removeEventListener('lostpointercapture', finish);
      node.removeEventListener('click', onClick);
      node.removeEventListener('dragstart', noNativeDrag);
      if (drag) document.body.classList.remove('dragging');
      drag = null;
    },
  };
}
