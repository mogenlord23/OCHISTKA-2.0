import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import Home from './Home.svelte';
import { getSystemOverview, POLL_INTERVAL_MS } from '../lib/api';
import type { SystemOverview } from '../lib/types';

vi.mock('../lib/api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/api')>()),
  getSystemOverview: vi.fn(),
}));
const getMock = vi.mocked(getSystemOverview);

const GB = 1024 ** 3;
const sample: SystemOverview = {
  cpu: { usagePercent: 12.4, logicalCores: 8 },
  memory: { totalBytes: 8 * GB, usedBytes: 4 * GB, usagePercent: 50 },
  disks: [
    {
      displayName: 'Локальный диск (C:)',
      mountPoint: 'C:\\',
      totalBytes: 512 * GB,
      usedBytes: 128 * GB,
      usagePercent: 25,
    },
    { displayName: 'E:\\', mountPoint: 'E:\\', totalBytes: GB, usedBytes: 0, usagePercent: 0 },
  ],
};

/** Даёт промисам и эффектам Svelte выполниться без таймеров. */
async function flush(): Promise<void> {
  await tick();
  for (let i = 0; i < 5; i++) await Promise.resolve();
  await tick();
}

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  getMock.mockReset();
});

describe('Home', () => {
  it('shows loading until data arrives', () => {
    getMock.mockReturnValue(new Promise(() => {}));
    render(Home);
    expect(screen.getByText('Загрузка…')).toBeTruthy();
  });

  it('renders CPU, memory and disks', async () => {
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    expect(screen.getByText('Загрузка: 12 %')).toBeTruthy();
    expect(screen.getByText('Логических ядер: 8')).toBeTruthy();
    expect(screen.getByText('4.0 ГБ из 8.0 ГБ')).toBeTruthy();
    expect(screen.getByText('50 %')).toBeTruthy();
    expect(screen.getByText('Локальный диск (C:)')).toBeTruthy();
    expect(screen.getByText('C:\\')).toBeTruthy();
  });

  it('shows the disk mount point when the disk has no other name', async () => {
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    expect(screen.getByText('E:\\')).toBeTruthy();
  });

  it('shows the unsupported reason in an alert', async () => {
    getMock.mockRejectedValue({
      kind: 'unsupported',
      message: 'Сведения о системе недоступно на macOS',
    });
    render(Home);
    await flush();
    const alert = screen.getByRole('alert');
    expect(alert.textContent).toContain('Недоступно на этой ОС');
    expect(alert.textContent).toContain('Сведения о системе недоступно на macOS');
  });

  it('stops polling after unmount', async () => {
    vi.useFakeTimers();
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    expect(getMock).toHaveBeenCalledTimes(1);

    cleanup();
    vi.advanceTimersByTime(POLL_INTERVAL_MS * 3);
    expect(getMock).toHaveBeenCalledTimes(1);
  });

  it('ignores a response that arrives after unmount', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    let resolveLate: (value: SystemOverview) => void = () => {};
    getMock.mockReturnValue(new Promise((resolve) => (resolveLate = resolve)));
    render(Home);
    cleanup();

    resolveLate(sample);
    await flush();

    expect(document.body.textContent).not.toContain('Загрузка: 12 %');
    expect(consoleError).not.toHaveBeenCalled();
    consoleError.mockRestore();
  });
});
