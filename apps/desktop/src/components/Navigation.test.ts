import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen, within } from '@testing-library/svelte';
import Navigation from './Navigation.svelte';
import source from './Navigation.svelte?raw';
import tauriConf from '../../src-tauri/tauri.conf.json';
import { SECTIONS } from '../lib/sections';

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
});
