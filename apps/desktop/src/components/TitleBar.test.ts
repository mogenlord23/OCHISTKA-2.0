import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import TitleBar from './TitleBar.svelte';

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
  it('shows the app name and author credit', () => {
    render(TitleBar);
    expect(screen.getByText('OCHISTKA')).toBeTruthy();
    expect(screen.getByText('Create: Slava Lavs')).toBeTruthy();
  });

  it('minimises, maximises and closes the window', async () => {
    render(TitleBar);
    await fireEvent.click(screen.getByRole('button', { name: 'Свернуть' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Развернуть' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Закрыть' }));
    expect(win.minimize).toHaveBeenCalledTimes(1);
    expect(win.toggleMaximize).toHaveBeenCalledTimes(1);
    expect(win.close).toHaveBeenCalledTimes(1);
  });
});
