import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import DriversPage from './DriversPage.svelte';
import { getHardware } from '../lib/api';
import {
  getDriversCatalog,
  installDriver,
  openDriverPage,
  openWindowsUpdate,
} from '../lib/driversApi';
import type { DriverItem } from '../lib/driversCatalog';

vi.mock('../lib/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/api')>();
  return { ...actual, getHardware: vi.fn(() => new Promise(() => {})) };
});

vi.mock('../lib/driversApi', () => ({
  getDriversCatalog: vi.fn(),
  installDriver: vi.fn(),
  openDriverPage: vi.fn(),
  openWindowsUpdate: vi.fn(),
}));

const ITEMS: DriverItem[] = [
  {
    id: 'nvidia-app',
    vendor: 'nvidia',
    title: 'NVIDIA App',
    description: 'Приложение NVIDIA',
    action: 'install',
    windowsUpdate: false,
  },
  {
    id: 'nvidia-geforce',
    vendor: 'nvidia',
    title: 'GeForce Game Ready',
    description: 'Драйвер GeForce',
    action: 'openSite',
    windowsUpdate: true,
  },
  {
    id: 'amd-software',
    vendor: 'amd',
    title: 'AMD Adrenalin',
    description: 'Драйвер Radeon',
    action: 'openSite',
    windowsUpdate: true,
  },
];

beforeEach(() => {
  vi.mocked(getDriversCatalog).mockResolvedValue(ITEMS);
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('DriversPage', () => {
  it('has no OS switcher', async () => {
    render(DriversPage);
    await screen.findByText('NVIDIA App');
    expect(screen.queryByRole('group', { name: 'Операционная система' })).toBeNull();
    for (const name of ['Windows', 'macOS', 'Linux']) {
      expect(screen.queryByRole('button', { name })).toBeNull();
    }
  });

  it('filters the catalog by the clicked vendor tile', async () => {
    render(DriversPage);
    await screen.findByText('NVIDIA App');
    expect(screen.getByText('AMD Adrenalin')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'NVIDIA' }));
    expect(screen.getByText('GeForce Game Ready')).toBeTruthy();
    expect(screen.queryByText('AMD Adrenalin')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'AMD' }));
    expect(screen.queryByText('NVIDIA App')).toBeNull();
    expect(screen.getByText('AMD Adrenalin')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Все' }));
    expect(screen.getByText('NVIDIA App')).toBeTruthy();
  });

  it('shows no URL text and no copy-link button', async () => {
    render(DriversPage);
    await screen.findByText('NVIDIA App');
    expect(screen.queryByText(/https?:\/\//)).toBeNull();
    expect(screen.queryByRole('button', { name: 'Копировать ссылку' })).toBeNull();
  });

  it('uses Установить for direct installers and Открыть сайт for web pages', async () => {
    render(DriversPage);
    await screen.findByText('NVIDIA App');
    expect(screen.getAllByRole('button', { name: 'Установить' })).toHaveLength(1);
    expect(screen.getAllByRole('button', { name: 'Открыть сайт' })).toHaveLength(2);
    expect(screen.getAllByRole('button', { name: 'Установить через Windows Update' })).toHaveLength(
      2,
    );
  });

  it('runs the install, shows download percent and then the done state', async () => {
    let finish: () => void = () => {};
    vi.mocked(installDriver).mockImplementation(
      (id, onProgress) =>
        new Promise<void>((resolve) => {
          onProgress({ id, stage: 'downloading', percent: 42, message: null });
          finish = resolve;
        }),
    );
    render(DriversPage);
    await fireEvent.click(await screen.findByRole('button', { name: 'Установить' }));
    const busy = await screen.findByRole('button', { name: 'Загрузка 42%' });
    expect((busy as HTMLButtonElement).disabled).toBe(true);
    finish();
    expect(await screen.findByRole('button', { name: 'Установщик запущен' })).toBeTruthy();
    expect(installDriver).toHaveBeenCalledWith('nvidia-app', expect.any(Function));
  });

  it('shows the reason and Повторить when the install fails', async () => {
    vi.mocked(installDriver).mockRejectedValue({
      kind: 'driver',
      message: 'Цифровая подпись не подтверждена: статус HashMismatch',
    });
    render(DriversPage);
    await fireEvent.click(await screen.findByRole('button', { name: 'Установить' }));
    expect(await screen.findByRole('button', { name: 'Повторить' })).toBeTruthy();
    expect((await screen.findByRole('alert')).textContent).toContain('HashMismatch');
  });

  it('opens the official page and the Windows Update page through the backend', async () => {
    vi.mocked(openDriverPage).mockResolvedValue();
    vi.mocked(openWindowsUpdate).mockResolvedValue();
    render(DriversPage);
    await fireEvent.click((await screen.findAllByRole('button', { name: 'Открыть сайт' }))[0]);
    expect(openDriverPage).toHaveBeenCalledWith('nvidia-geforce');
    await fireEvent.click(
      screen.getAllByRole('button', { name: 'Установить через Windows Update' })[0],
    );
    expect(openWindowsUpdate).toHaveBeenCalled();
  });

  it('marks the vendor found in this computer and lists it first', async () => {
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
      cpuBrand: null,
      driveModels: [],
    });
    render(DriversPage);
    await vi.waitFor(() => expect(screen.getByText('ваше устройство')).toBeTruthy());
    const tiles = screen
      .getAllByRole('button', { pressed: false })
      .map((button) => button.textContent ?? '');
    expect(tiles[0]).toContain('AMD');
    expect(tiles[0]).toContain('ваше устройство');
  });

  it('shows the error when the catalog cannot be loaded', async () => {
    vi.mocked(getDriversCatalog).mockRejectedValue({ kind: 'probe', message: 'Нет связи' });
    render(DriversPage);
    expect((await screen.findByRole('alert')).textContent).toContain('Нет связи');
  });
});
