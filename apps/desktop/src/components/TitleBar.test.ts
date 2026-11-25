import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import TitleBar from './TitleBar.svelte';
import source from './TitleBar.svelte?raw';

const geometry = readFileSync(join(__dirname, '..', 'geometry.css'), 'utf8');

const win = vi.hoisted(() => ({
  minimize: vi.fn(),
  toggleMaximize: vi.fn(),
  close: vi.fn(),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => win,
}));

afterEach(() => {
  cleanup();
  win.minimize.mockClear();
  win.toggleMaximize.mockClear();
  win.close.mockClear();
});

describe('TitleBar', () => {
  it('uses the common 44px control height and keeps window buttons square in every section', () => {
    expect(geometry).toMatch(/--control-height:\s*44px/);
    expect(source).toContain('--subtabs-height: var(--control-height)');
    expect(source).toMatch(/width:\s*var\(--subtabs-height\);\s*height:\s*var\(--subtabs-height\)/);
    expect(source).not.toContain('cursor-grid');
  });

  it('keeps the window toolbar free of branding', () => {
    render(TitleBar);
    expect(screen.queryByLabelText('OCHISTKA')).toBeNull();
    expect(screen.queryByText(/Create/)).toBeNull();
    expect(screen.queryByText(/Slava/)).toBeNull();
  });

  it('minimises, maximises and closes the window', async () => {
    render(TitleBar);
    await fireEvent.click(screen.getByRole('button', { name: 'Свернуть' }));
    expect(win.minimize).toHaveBeenCalledTimes(1);
    expect(win.toggleMaximize).toHaveBeenCalledTimes(0);
    expect(win.close).toHaveBeenCalledTimes(0);

    await fireEvent.click(screen.getByRole('button', { name: 'Развернуть' }));
    expect(win.minimize).toHaveBeenCalledTimes(1);
    expect(win.toggleMaximize).toHaveBeenCalledTimes(1);
    expect(win.close).toHaveBeenCalledTimes(0);

    await fireEvent.click(screen.getByRole('button', { name: 'Закрыть' }));
    expect(win.minimize).toHaveBeenCalledTimes(1);
    expect(win.toggleMaximize).toHaveBeenCalledTimes(1);
    expect(win.close).toHaveBeenCalledTimes(1);
  });

  it('lets the frameless window be dragged by the empty bar, but not by the buttons', () => {
    const { container } = render(TitleBar);
    const header = container.querySelector('header');
    expect(header?.hasAttribute('data-tauri-drag-region')).toBe(true);
    expect(header?.querySelector('.tabs')?.hasAttribute('data-tauri-drag-region')).toBe(true);

    for (const label of ['Свернуть', 'Развернуть', 'Закрыть']) {
      const button = screen.getByRole('button', { name: label });
      expect(button.hasAttribute('data-tauri-drag-region')).toBe(false);
    }
  });
});
