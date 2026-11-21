import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import VariableProximity from './VariableProximity.svelte';

let raf: FrameRequestCallback[] = [];
let clock = 0;

function stubFrames() {
  raf = [];
  clock = 0;
  vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => raf.push(cb));
  vi.stubGlobal('cancelAnimationFrame', () => {});
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (
    this: HTMLElement,
  ) {
    const i = Array.from(this.parentElement?.children ?? []).indexOf(this);
    return {
      left: i * 20,
      right: i * 20 + 20,
      top: 0,
      bottom: 20,
      width: 20,
      height: 20,
      x: i * 20,
      y: 0,
    } as DOMRect;
  });
}

/** Прокручивает кадры анимации; `ms` модельного времени. */
function run(ms: number) {
  for (let t = 0; t < ms; t += 16) {
    clock += 16;
    const batch = raf;
    raf = [];
    batch.forEach((cb) => cb(clock));
  }
}

const weight = (el: HTMLElement) =>
  Number(/["']wght["'] ([\d.]+)/.exec(el.style.fontVariationSettings)?.[1] ?? NaN);

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('VariableProximity (port of React Bits)', () => {
  it('starts every letter at the "from" settings', () => {
    stubFrames();
    const { container } = render(VariableProximity, {
      label: 'OCHISTKA',
      fromFontVariationSettings: "'wght' 400",
      toFontVariationSettings: "'wght' 1000",
    });
    const letters = [...container.querySelectorAll<HTMLElement>('[aria-hidden="true"]')];
    expect(letters).toHaveLength(8);
    expect(letters.every((l) => weight(l) === 400)).toBe(true);
  });

  it('makes letters near the pointer heavier by distance, smoothly, and returns on leave', () => {
    stubFrames();
    const { container } = render(VariableProximity, {
      label: 'OCHISTKA',
      fromFontVariationSettings: "'wght' 400",
      toFontVariationSettings: "'wght' 1000",
      radius: 50,
      falloff: 'linear',
      smoothing: 0.2,
    });
    const letters = [...container.querySelectorAll<HTMLElement>('[aria-hidden="true"]')];
    // Курсор в центре первой буквы (x = 10, y = 10).
    window.dispatchEvent(new MouseEvent('mousemove', { clientX: 10, clientY: 10 }));
    run(48);
    const early = weight(letters[0]);
    expect(early).toBeGreaterThan(400);
    expect(early).toBeLessThan(1000);
    run(1500);
    expect(weight(letters[0])).toBeGreaterThan(990);
    // Дальние буквы (расстояние больше радиуса) остаются в исходном состоянии.
    expect(weight(letters[7])).toBe(400);
    // Вторая буква на расстоянии 20 из 50: сила 0.6 → вес 760.
    expect(weight(letters[1])).toBeGreaterThan(700);
    expect(weight(letters[1])).toBeLessThan(800);
    document.documentElement.dispatchEvent(new MouseEvent('mouseleave'));
    run(2500);
    expect(letters.every((l) => weight(l) === 400)).toBe(true);
  });
});
