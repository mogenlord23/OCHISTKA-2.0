import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import TrayPanel from './TrayPanel.svelte';
import { trayAction, trayStatus } from '../lib/tray';
import { SECTIONS } from '../lib/sections';

vi.mock('../lib/tray', () => ({
  listenDesktop: () => () => {},
  trayAction: vi.fn(),
  trayHover: vi.fn(),
  trayStatus: vi.fn(),
}));

beforeEach(() => {
  vi.mocked(trayStatus).mockResolvedValue({ vpnConnected: false });
  vi.mocked(trayAction).mockResolvedValue(undefined);
});

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

it('shows the seven section tiles and opens the chosen section', async () => {
  render(TrayPanel);
  expect(screen.getAllByRole('button').slice(1).map((button) => button.textContent?.trim()))
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
});
