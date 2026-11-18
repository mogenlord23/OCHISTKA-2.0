import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import DriversPage from './DriversPage.svelte';
import { getHardware } from '../lib/api';

vi.mock('../lib/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/api')>();
  return { ...actual, getHardware: vi.fn(() => new Promise(() => {})) };
});

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

  it('shows the hardware row and the ready catalog with official links as text', () => {
    render(DriversPage);
    expect(screen.getByRole('heading', { name: 'Готовый каталог' })).toBeTruthy();
    expect(screen.getByText('GeForce Game Ready / Studio')).toBeTruthy();
    expect(screen.getByText('https://www.nvidia.com/en-us/drivers/')).toBeTruthy();
    expect(screen.getAllByRole('button', { name: 'Копировать ссылку' }).length).toBeGreaterThan(5);
    expect(document.querySelector('a[href]')).toBeNull();
  });

  it('builds the hardware row from the detected GPU', async () => {
    vi.mocked(getHardware).mockResolvedValueOnce({
      gpus: [
        {
          vendor: 'AMD',
          model: 'Radeon RX 6600',
          usagePercent: null,
          totalMemoryBytes: null,
          usedMemoryBytes: null,
        },
      ],
      motherboard: { vendor: null, name: null, version: null },
      cpuBrand: 'AMD Ryzen 5 5600',
      driveModels: [],
    });
    render(DriversPage);
    await vi.waitFor(() => expect(screen.queryByText('Видеокарта NVIDIA')).toBeNull());
    expect(screen.getByText('AMD · видео / чипсет')).toBeTruthy();
  });
});
