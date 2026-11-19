import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import SpeedTest from './SpeedTest.svelte';
import VpnNetworkTab from './VpnNetworkTab.svelte';
import type { SpeedEvent } from '../lib/speedtestApi';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
  Channel: class {
    onmessage: (event: unknown) => void = () => {};
  },
}));

type Channelish = { onmessage: (event: SpeedEvent) => void };

/** Подставной Rust: на `speedtest_start` отдаёт заданные события и завершается. */
function backend(events: SpeedEvent[], opts: { reject?: boolean; hold?: boolean } = {}) {
  let release: () => void = () => {};
  invokeMock.mockImplementation(async (cmd: string, args?: { onEvent?: Channelish }) => {
    if (cmd === 'speedtest_start') {
      if (opts.reject) throw { kind: 'probe', message: 'x' };
      for (const e of events) args?.onEvent?.onmessage(e);
      if (opts.hold) await new Promise<void>((r) => (release = r));
      return undefined;
    }
    if (cmd === 'get_network_adapters') return [];
    return undefined;
  });
  return () => release();
}

const RESULT = { downloadMbps: 120, uploadMbps: 40, pingMs: 12, jitterMs: 2, loadedPingMs: 40 };
const FULL: SpeedEvent[] = [
  { type: 'phase', phase: 'ping' },
  { type: 'ping', ms: 12 },
  { type: 'phase', phase: 'download' },
  { type: 'progress', phase: 'download', mbps: 100, fraction: 1 },
  { type: 'phase', phase: 'upload' },
  { type: 'done', result: RESULT },
];

beforeEach(() => localStorage.clear());
afterEach(() => {
  cleanup();
  invokeMock.mockReset();
});

describe('SpeedTest', () => {
  it('does nothing until the button is pressed and shows the traffic note once', async () => {
    backend(FULL);
    render(SpeedTest);
    expect(invokeMock).not.toHaveBeenCalled();
    expect(screen.getByText(/расходует трафик и обращается к внешнему серверу/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Проверить скорость' }));
    await screen.findByText('Отличное соединение.');
    expect(screen.queryByText(/расходует трафик/)).toBeNull();
    // При следующем показе компонента предупреждения уже нет.
    cleanup();
    render(SpeedTest);
    expect(screen.queryByText(/расходует трафик/)).toBeNull();
  });

  it('shows result cards and the summary after a run', async () => {
    backend(FULL);
    render(SpeedTest);
    await fireEvent.click(screen.getByRole('button', { name: 'Проверить скорость' }));
    await screen.findByText('Отличное соединение.');
    expect(screen.getByTestId('speed-down').textContent).toContain('120');
    expect(screen.getByTestId('speed-up').textContent).toContain('40.0');
    expect(screen.getByTestId('speed-ping').textContent).toContain('12');
    expect(screen.getByTestId('speed-loaded').textContent).toContain('40');
    expect(screen.getByTestId('speed-jitter').textContent).toContain('2.0');
    expect(screen.getByRole('button', { name: 'Повторить' })).toBeTruthy();
    expect(calls('speedtest_start')).toHaveLength(1);
  });

  it('shows a plain no-internet message without codes and allows retry', async () => {
    backend([
      { type: 'phase', phase: 'ping' },
      { type: 'failed', kind: 'offline' },
    ]);
    render(SpeedTest);
    await fireEvent.click(screen.getByRole('button', { name: 'Проверить скорость' }));
    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toBe('Нет подключения к интернету. Проверьте сеть и повторите');
    await fireEvent.click(screen.getByRole('button', { name: 'Повторить' }));
    await waitFor(() => expect(calls('speedtest_start')).toHaveLength(2));
  });

  it('a rejected command becomes a plain error, not a stuck button', async () => {
    backend([], { reject: true });
    render(SpeedTest);
    await fireEvent.click(screen.getByRole('button', { name: 'Проверить скорость' }));
    expect((await screen.findByRole('alert')).textContent).toMatch(/Сервер измерения/);
    expect(screen.getByRole('button', { name: 'Повторить' })).toBeTruthy();
  });

  it('can be cancelled while running and the UI stays responsive', async () => {
    const finish = backend(
      [
        { type: 'phase', phase: 'download' },
        { type: 'progress', phase: 'download', mbps: 30, fraction: 0.3 },
      ],
      { hold: true },
    );
    render(SpeedTest);
    await fireEvent.click(screen.getByRole('button', { name: 'Проверить скорость' }));
    const stop = await screen.findByRole('button', { name: 'Остановить' });
    expect(screen.getByRole('progressbar')).toBeTruthy();
    await fireEvent.click(stop);
    expect(calls('speedtest_cancel')).toHaveLength(1);
    finish();
  });
});

describe('Network tab', () => {
  it('has the speed test above the adapter list', async () => {
    backend(FULL);
    render(VpnNetworkTab);
    expect(screen.getByRole('heading', { name: 'Твой интернет' })).toBeTruthy();
    expect(await screen.findByText('Сетевых адаптеров не найдено.')).toBeTruthy();
  });
});

function calls(cmd: string) {
  return invokeMock.mock.calls.filter((c) => c[0] === cmd);
}
