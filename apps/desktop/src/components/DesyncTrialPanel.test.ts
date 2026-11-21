import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import DesyncTrialPanel from './DesyncTrialPanel.svelte';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

const IDLE = {
  active: false,
  remainingSecs: 0,
  message: 'Выключено',
  unavailable: null,
  notes: [],
};
const RUNNING = { ...IDLE, active: true, remainingSecs: 65, message: 'Пробный запуск' };

beforeEach(() => invokeMock.mockReset());
afterEach(cleanup);

describe('DesyncTrialPanel', () => {
  it('does not start without consent and sends consent=true only after the checkbox', async () => {
    invokeMock.mockImplementation(async (cmd: string) =>
      cmd === 'desync_trial_start' ? RUNNING : IDLE,
    );
    render(DesyncTrialPanel);
    const button = await screen.findByRole('button', { name: 'Запустить пробную проверку' });
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('desync_trial_status'));
    expect((button as HTMLButtonElement).disabled).toBe(true);
    expect(invokeMock).not.toHaveBeenCalledWith('desync_trial_start', expect.anything());
    await fireEvent.click(screen.getByRole('checkbox'));
    await waitFor(() => expect((button as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(button);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('desync_trial_start', { consent: true }),
    );
    expect(await screen.findByText(/осталось 1:05/)).toBeTruthy();
  });

  it('shows the reason and keeps start disabled when the trial is unavailable', async () => {
    invokeMock.mockResolvedValue({ ...IDLE, unavailable: 'Нужны права администратора' });
    render(DesyncTrialPanel);
    expect(await screen.findByText('Нужны права администратора')).toBeTruthy();
    await fireEvent.click(screen.getByRole('checkbox'));
    expect(
      (screen.getByRole('button', { name: 'Запустить пробную проверку' }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
  });

  it('lets the user stop and mark observations, repeated click resets to unchecked', async () => {
    let notes: { service: string; verdict: string }[] = [];
    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, string>) => {
      if (cmd === 'desync_trial_note') {
        notes = [{ service: args!.service, verdict: args!.verdict }];
        return { ...RUNNING, notes };
      }
      if (cmd === 'desync_trial_stop') return IDLE;
      return { ...RUNNING, notes };
    });
    render(DesyncTrialPanel);
    const works = (await screen.findAllByRole('button', { name: 'Работает' }))[0];
    await fireEvent.click(works);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('desync_trial_note', {
        service: 'youtube',
        verdict: 'works',
      }),
    );
    await waitFor(() => expect(works.getAttribute('aria-pressed')).toBe('true'));
    await fireEvent.click(works);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('desync_trial_note', {
        service: 'youtube',
        verdict: 'unchecked',
      }),
    );
    await fireEvent.click(screen.getByRole('button', { name: 'Остановить' }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('desync_trial_stop'));
  });
});
