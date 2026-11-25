import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import { tick } from 'svelte';
import ContextMenu from './components/ContextMenu.svelte';
import { allowsNativeMenu, attachChromeGuards } from './lib/chromeGuards';
import { contrastRatio, SELECTION } from './lib/tokens';

const APP = readFileSync(join(__dirname, 'App.svelte'), 'utf8');

afterEach(() => cleanup());

describe('native context menu guard (item 15)', () => {
  it('blocks the WebView menu everywhere except inputs and .selectable', () => {
    document.body.innerHTML =
      '<div id="a">текст</div><input id="b"><textarea id="c"></textarea><p class="selectable"><span id="d">путь</span></p>';
    const detach = attachChromeGuards(window);
    const fire = (id: string) => {
      const event = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
      document.getElementById(id)!.dispatchEvent(event);
      return event.defaultPrevented;
    };
    expect(fire('a')).toBe(true);
    expect(fire('b')).toBe(false);
    expect(fire('c')).toBe(false);
    expect(fire('d')).toBe(false);
    detach();
    expect(fire('a')).toBe(false);
    expect(allowsNativeMenu(null)).toBe(false);
  });
});

describe('text selection (item 5)', () => {
  it('selection colours are the brand pink with contrast of at least 4.5', () => {
    expect(contrastRatio(SELECTION.text, SELECTION.background)).toBeGreaterThanOrEqual(4.5);
  });

  it('the chrome is unselectable, inputs and .selectable are selectable, one ::selection rule', () => {
    expect(APP).toMatch(/\.shell\s*\{[^}]*user-select:\s*none/);
    expect(APP).toMatch(/:global\(\.selectable\)/);
    expect(APP).toMatch(/user-select:\s*text/);
    expect(APP.match(/::selection/g)?.length).toBe(1);
  });
});

it('changes section colour without animating the whole background', () => {
  expect(APP).toMatch(/\.stage\s*\{[^}]*background-color:\s*var\(--accent\)/);
  expect(APP).not.toMatch(/\.stage\s*\{[^}]*transition:/);
});

it('clips the decorative backdrop without making the stage a scroll container', () => {
  expect(APP).toMatch(/\.stage\s*\{[^}]*overflow:\s*clip/);
});

describe('dock panel (item 1)', () => {
  it('has no border, a black card, one size and no media-query resize', () => {
    const block = APP.slice(APP.indexOf('.bottom-wrap {'), APP.indexOf('.handle {'));
    expect(block).not.toMatch(/border:\s*1px/);
    expect(block).not.toMatch(/border-color/);
    expect(block).toMatch(/background-color:\s*var\(--black\)/);
    expect(block).not.toMatch(/@media/);
    expect(block).toMatch(/height:\s*var\(--dock-panel-height\)/);
  });

  it('centres the handle in the free space below the last navigation tile', () => {
    expect(APP).toMatch(/\.dock\s*\{[^}]*flex:\s*1/);
    expect(APP).toMatch(/\.dock\s*\{[^}]*justify-content:\s*center/);
  });
});

describe('context menu position (item 9.3)', () => {
  it('opens at the click point and is clamped to the window', async () => {
    const { container } = render(ContextMenu, {
      items: [{ id: 'a', label: 'A' }],
      x: 50,
      y: 60,
      label: 'menu',
      onselect: () => {},
      onclose: () => {},
    });
    await tick();
    await tick();
    const menu = container.querySelector('.menu') as HTMLElement;
    expect(menu.style.left).toBe('50px');
    expect(menu.style.top).toBe('60px');
    cleanup();
    const far = render(ContextMenu, {
      items: [{ id: 'a', label: 'A' }],
      x: 99999,
      y: 99999,
      label: 'menu',
      onselect: () => {},
      onclose: () => {},
    });
    await tick();
    await tick();
    const clamped = far.container.querySelector('.menu') as HTMLElement;
    expect(parseInt(clamped.style.left)).toBeLessThanOrEqual(window.innerWidth - 8);
    expect(parseInt(clamped.style.top)).toBeLessThanOrEqual(window.innerHeight - 8);
  });
});
