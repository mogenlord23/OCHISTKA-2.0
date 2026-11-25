import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import TrayPanel from './TrayPanel.svelte';
import { trayAction, trayStatus } from '../lib/tray';
import { SECTIONS } from '../lib/sections';
import { isTauri } from '@tauri-apps/api/core';
import { tick } from 'svelte';
import { getHardware, getSystemOverview, POLL_INTERVAL_MS } from '../lib/api';
import type { HardwareOverview, SystemOverview } from '../lib/types';

vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@tauri-apps/api/core')>()),
  isTauri: vi.fn(),
}));

vi.mock('../lib/api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/api')>()),
  getHardware: vi.fn(),
  getSystemOverview: vi.fn(),
}));

const trayEvents = new Map<string, () => void>();
const hardware: HardwareOverview = {
  gpus: [{ vendor: null, model: null, totalMemoryBytes: null, usedMemoryBytes: null, usagePercent: 42 }],
  motherboard: { vendor: null, name: null, version: null },
  cpuBrand: null,
  driveModels: [],
};
const overview: SystemOverview = {
  cpu: { logicalCores: 8, usagePercent: 13 },
  memory: { totalBytes: 100, usedBytes: 61, usagePercent: 61 },
  disks: [],
  osName: 'Windows',
  osVersion: '11',
};

vi.mock('../lib/tray', () => ({
  listenDesktop: (event: string, handler: () => void) => {
    trayEvents.set(event, handler);
    return () => trayEvents.delete(event);
  },
  trayAction: vi.fn(),
  trayHover: vi.fn(),
  trayStatus: vi.fn(),
}));

beforeEach(() => {
  vi.mocked(isTauri).mockReturnValue(false);
  vi.mocked(getHardware).mockResolvedValue(hardware);
  vi.mocked(getSystemOverview).mockResolvedValue(overview);
  vi.mocked(trayStatus).mockResolvedValue({ vpnConnected: false });
  vi.mocked(trayAction).mockResolvedValue(undefined);
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.resetAllMocks();
});

it('shows the seven section tiles and opens the chosen section', async () => {
  render(TrayPanel);
  const tiles = screen.getAllByRole('button').slice(1);
  expect(tiles.every((tile) => !tile.textContent?.trim())).toBe(true);
  expect(tiles.map((tile) => tile.getAttribute('aria-label')))
    .toEqual(SECTIONS.map((section) => section.title));
  expect(screen.queryByRole('button', { name: /Проверить/ })).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Вирусы' }));
  expect(trayAction).toHaveBeenCalledWith('viruses');
});

it('updates the VPN button after the existing connection command returns', async () => {
  render(TrayPanel);
  const button = await screen.findByRole<HTMLButtonElement>('button', { name: 'Включить VPN' });
  await vi.waitFor(() => expect(button.disabled).toBe(false));
  vi.mocked(trayStatus).mockResolvedValue({ vpnConnected: true });
  await fireEvent.click(button);
  expect(trayAction).toHaveBeenCalledWith('toggleVpn');
  expect(await screen.findByRole('button', { name: 'Выключить VPN' })).toBeTruthy();
  expect(document.querySelector('.tray .connect-pulses')).toBeTruthy();
});

it('warms the CPU, GPU and RAM values once, then only polls while the native tray is open', async () => {
  vi.mocked(isTauri).mockReturnValue(true);
  vi.useFakeTimers();
  render(TrayPanel);
  await vi.advanceTimersByTimeAsync(3 * POLL_INTERVAL_MS);
  expect(getHardware).toHaveBeenCalledTimes(1);
  expect(getSystemOverview).toHaveBeenCalledTimes(1);
  let release!: (value: HardwareOverview) => void;
  vi.mocked(getHardware).mockReturnValueOnce(new Promise((resolve) => (release = resolve)));

  trayEvents.get('tray-open')?.();
  await tick();
  await vi.advanceTimersByTimeAsync(0);
  expect(screen.getByText('42%')).toBeTruthy();
  expect(screen.getByText('13%')).toBeTruthy();
  expect(screen.getAllByRole('term').map(el => el.textContent)).toEqual(['CPU', 'GPU', 'RAM']);
  release(hardware);
  await vi.advanceTimersByTimeAsync(0);
  expect(screen.getByText('61%')).toBeTruthy();

  vi.mocked(getHardware).mockResolvedValue({
    ...hardware,
    gpus: [{ ...hardware.gpus[0], usagePercent: null }],
  });
  vi.mocked(getSystemOverview).mockResolvedValue({
    ...overview,
    memory: { ...overview.memory, usagePercent: 0 },
  });
  await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
  expect(screen.getByText('—')).toBeTruthy();
  expect(screen.getByText('0%')).toBeTruthy();

  trayEvents.get('tray-closed')?.();
  await tick();
  await vi.advanceTimersByTimeAsync(3 * POLL_INTERVAL_MS);
  expect(getHardware).toHaveBeenCalledTimes(3);
  expect(getSystemOverview).toHaveBeenCalledTimes(3);
});
