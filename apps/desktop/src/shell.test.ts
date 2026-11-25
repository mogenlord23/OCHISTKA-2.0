import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import App from './App.svelte';
import TitleBar from './components/TitleBar.svelte';
import desktopConfig from '../src-tauri/tauri.conf.json';

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    minimize: vi.fn(),
    toggleMaximize: vi.fn(),
    close: vi.fn(),
    isMaximized: vi.fn(async () => false),
    onResized: vi.fn(async () => () => {}),
  }),
}));

afterEach(() => {
  cleanup();
  localStorage.clear();
});

describe('shell', () => {
  it('opens the main window at the minimum size that fits the full navigation and cursor grid', () => {
    const main = desktopConfig.app.windows.find((window) => window.label === 'main');
    expect(main).toMatchObject({ width: 900, minWidth: 900, height: 760, minHeight: 760 });
  });

  it('draws window buttons as inline SVG and swaps the maximise glyph when maximised', () => {
    const { container, rerender } = render(TitleBar, { props: { maximized: false } });
    expect(container.querySelectorAll('.controls button svg').length).toBe(3);
    const before = container.querySelector('.maximize svg')?.innerHTML;
    void rerender({ maximized: true });
    return Promise.resolve().then(() => {
      expect(container.querySelector('.maximize svg')?.innerHTML).not.toBe(before);
    });
  });

  it('marks the root as not maximised by default and shows no version text', () => {
    const { container } = render(App);
    expect(container.querySelector('.shell')?.getAttribute('data-maximized')).toBe('false');
    expect(container.textContent).not.toMatch(/\b0\.1\.0\b/);
  });

  it('collapses the bottom block into a chevron handle and expands it again', async () => {
    render(App);
    const handle = screen.getByRole('button', { name: 'Свернуть панель' });
    expect(handle.getAttribute('aria-expanded')).toBe('true');
    await fireEvent.click(handle);
    const open = screen.getByRole('button', { name: 'Показать панель' });
    expect(open.getAttribute('aria-expanded')).toBe('false');
    await fireEvent.click(open);
    expect(screen.getByRole('button', { name: 'Свернуть панель' })).toBeTruthy();
  });
});
