import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import TitleBar from './TitleBar.svelte';

const win = vi.hoisted(() => ({
  minimize: vi.fn().mockResolvedValue(undefined),
  toggleMaximize: vi.fn().mockResolvedValue(undefined),
  close: vi.fn().mockResolvedValue(undefined),
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
  it('shows the app name and no author credit line', () => {
    render(TitleBar);
    expect(screen.getByLabelText('OCHISTKA')).toBeTruthy();
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

  it('announces the action matching the window state', async () => {
    const { rerender } = render(TitleBar, { maximized: false });
    expect(screen.getByRole('button', { name: 'Развернуть' })).toBeTruthy();
    await rerender({ maximized: true });
    expect(screen.getByRole('button', { name: 'Восстановить окно' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Восстановить окно' }));
    expect(win.toggleMaximize).toHaveBeenCalledTimes(1);
  });

  it.each([
    ['Свернуть', 'minimize'],
    ['Развернуть', 'toggleMaximize'],
    ['Закрыть', 'close'],
  ] as const)('reports an error when %s fails', async (label, method) => {
    const alert = vi.spyOn(window, 'alert').mockImplementation(() => {});
    const errorLog = vi.spyOn(console, 'error').mockImplementation(() => {});
    win[method].mockRejectedValueOnce(new Error('native failure'));
    render(TitleBar);
    await fireEvent.click(screen.getByRole('button', { name: label }));
    expect(alert).toHaveBeenCalledWith('Не удалось изменить состояние окна. Попробуйте ещё раз.');
    expect(errorLog).toHaveBeenCalled();
    alert.mockRestore();
    errorLog.mockRestore();
  });

  it('lets the frameless window be dragged by the bar and brand, but not by the buttons', () => {
    const { container } = render(TitleBar);
    const header = container.querySelector('header');
    const name = screen.getByLabelText('OCHISTKA');
    const brand = name.parentElement;

    expect(header?.hasAttribute('data-tauri-drag-region')).toBe(true);
    expect(brand?.classList.contains('brand')).toBe(true);
    expect(brand?.hasAttribute('data-tauri-drag-region')).toBe(true);
    expect(name.hasAttribute('data-tauri-drag-region')).toBe(true);

    for (const label of ['Свернуть', 'Развернуть', 'Закрыть']) {
      const button = screen.getByRole('button', { name: label });
      expect(button.hasAttribute('data-tauri-drag-region')).toBe(false);
    }
  });
});
