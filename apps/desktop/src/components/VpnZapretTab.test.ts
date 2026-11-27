import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import VpnZapretTab from './VpnZapretTab.svelte';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke, isTauri: () => true }));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  invoke.mockReset();
});

it('offers a single action, animates on click and reflects the real result', async () => {
  vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue('Windows NT');
  let running = false;
  let finish: () => void = () => {};
  invoke.mockImplementation((command, args) => {
    if (command === 'zapret_status') return Promise.resolve({ configured: true, running });
    return new Promise((resolve) => {
      finish = () => {
        running = args.enabled;
        resolve({ configured: true, running });
      };
    });
  });
  const onchanged = vi.fn();
  render(VpnZapretTab, { active: true, vpnState: 'disconnected', onchanged });
  const card = screen.getByRole('region', { name: 'Обход блокировок' });
  const button = within(card).getByRole('button', { name: 'Обход блокировок' }) as HTMLButtonElement;
  expect(within(card).getAllByRole('button')).toHaveLength(1);
  expect(button.textContent?.trim()).toBe('OFF');
  await waitFor(() => expect(button.disabled).toBe(false));

  await fireEvent.click(button);
  expect(button.classList.contains('filled')).toBe(true);
  expect(button.textContent?.trim()).toBe('ON');
  expect(button.getAttribute('aria-busy')).toBe('true');
  expect(invoke).toHaveBeenCalledWith('zapret_set', { enabled: true });
  finish();
  await waitFor(() => expect(button.getAttribute('aria-pressed')).toBe('true'));

  await fireEvent.click(button);
  expect(button.classList.contains('filled')).toBe(false);
  expect(button.textContent?.trim()).toBe('OFF');
  expect(invoke).toHaveBeenCalledWith('zapret_set', { enabled: false });
  finish();
  await waitFor(() => expect(button.getAttribute('aria-pressed')).toBe('false'));
  expect(onchanged).toHaveBeenCalledTimes(2);
});
