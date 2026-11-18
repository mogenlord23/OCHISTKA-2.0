import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import DriversPage from './DriversPage.svelte';

afterEach(() => {
  cleanup();
});

describe('DriversPage', () => {
  it('enables Windows and disables macOS and Linux with the reason', () => {
    render(DriversPage);
    const windows = screen.getByRole('button', { name: 'Windows' }) as HTMLButtonElement;
    const macos = screen.getByRole('button', { name: 'macOS' }) as HTMLButtonElement;
    const linux = screen.getByRole('button', { name: 'Linux' }) as HTMLButtonElement;
    expect(windows.disabled).toBe(false);
    expect(macos.disabled).toBe(true);
    expect(linux.disabled).toBe(true);
  });

  it('shows the reason next to each disabled OS option', () => {
    render(DriversPage);
    expect(screen.getByText('macOS: Нет поддержки этой ОС в текущей версии')).toBeTruthy();
    expect(screen.getByText('Linux: Нет поддержки этой ОС в текущей версии')).toBeTruthy();
    const macos = screen.getByRole('button', { name: 'macOS' });
    const reasonId = macos.getAttribute('aria-describedby');
    expect(reasonId).toBeTruthy();
    expect(document.getElementById(reasonId ?? '')?.textContent).toContain(
      'Нет поддержки этой ОС в текущей версии',
    );
  });

  it('shows the Windows driver placeholder with its reason', () => {
    render(DriversPage);
    expect(screen.getByText(/^Раздел в разработке: список драйверов Windows/)).toBeTruthy();
  });
});
