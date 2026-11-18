import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import Home from './Home.svelte';
import { getHardware, getSystemOverview, POLL_INTERVAL_MS, toCommandError } from '../lib/api';
import type { HardwareOverview, SystemOverview } from '../lib/types';

vi.mock('../lib/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/api')>();
  return {
    ...actual,
    getSystemOverview: vi.fn(),
    getHardware: vi.fn(),
    toCommandError: vi.fn(actual.toCommandError),
  };
});
const getMock = vi.mocked(getSystemOverview);
const toErrorMock = vi.mocked(toCommandError);
const hardwareMock = vi.mocked(getHardware);

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
      kind: 'ssd',
    },
    {
      displayName: 'E:\\',
      mountPoint: 'E:\\',
      totalBytes: GB,
      usedBytes: 0,
      usagePercent: 0,
      kind: 'unknown',
    },
  ],
  osName: 'Windows 11',
  osVersion: '24H2',
};

const hardwareSample: HardwareOverview = {
  gpus: [
    {
      vendor: 'NVIDIA',
      model: 'GeForce RTX 4070',
      usagePercent: 37,
      totalMemoryBytes: 12 * GB,
      usedMemoryBytes: 3 * GB,
    },
  ],
  motherboard: { vendor: 'ASUS', name: 'ROG STRIX B650', version: 'Rev 1.xx' },
};

/** Даёт эффектам Svelte и продолжениям промисов выполниться (требует фейковых таймеров). */
async function flush(): Promise<void> {
  await tick();
  await vi.advanceTimersByTimeAsync(0);
  await tick();
}

const consoleError = vi.spyOn(console, 'error');

beforeEach(() => {
  vi.useFakeTimers();
  consoleError.mockImplementation(() => {});
  hardwareMock.mockResolvedValue(hardwareSample);
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  getMock.mockReset();
  hardwareMock.mockReset();
  toErrorMock.mockClear();
  consoleError.mockReset();
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

  it('shows GPU and motherboard tiles from the hardware port', async () => {
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    expect(screen.getByText('GeForce RTX 4070')).toBeTruthy();
    expect(screen.getByText('Загрузка: 37 %')).toBeTruthy();
    expect(screen.getByText('ROG STRIX B650')).toBeTruthy();
  });

  it('shows the hardware reason in the tiles when the platform cannot read it', async () => {
    getMock.mockResolvedValue(sample);
    hardwareMock.mockRejectedValue({
      kind: 'unsupported',
      message: 'Чтение комплектующих недоступно на Linux',
    });
    render(Home);
    await flush();
    expect(screen.getAllByText('Чтение комплектующих недоступно на Linux').length).toBe(2);
    expect(screen.getByText('Локальный диск (C:)')).toBeTruthy();
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
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    expect(getMock).toHaveBeenCalledTimes(1);

    cleanup();
    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS * 3);
    expect(getMock).toHaveBeenCalledTimes(1);
  });

  it('does not start a request while the previous one is still pending', async () => {
    getMock.mockReturnValue(new Promise(() => {}));
    render(Home);
    await flush();
    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS * 3);
    expect(getMock).toHaveBeenCalledTimes(1);
  });

  it('FND-0170: polls again on every interval while responses resolve', async () => {
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    expect(getMock).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    expect(getMock).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    expect(getMock).toHaveBeenCalledTimes(3);
    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    expect(getMock).toHaveBeenCalledTimes(4);
  });

  it('FND-0170: does not poll before the interval has elapsed', async () => {
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS - 1);
    expect(getMock).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(getMock).toHaveBeenCalledTimes(2);
  });

  it('FND-0170: pending clears after a failed request and polling recovers', async () => {
    getMock.mockRejectedValueOnce({ kind: 'probe', message: 'временный сбой' });
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    expect(screen.getByRole('alert').textContent).toContain('временный сбой');

    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    expect(getMock).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.getByText('Загрузка: 12 %')).toBeTruthy();
  });

  it('FND-0170: resumes polling after a slow request settles', async () => {
    let resolveSlow: (value: SystemOverview) => void = () => {};
    getMock.mockReturnValueOnce(new Promise((resolve) => (resolveSlow = resolve)));
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS * 2);
    expect(getMock).toHaveBeenCalledTimes(1);

    resolveSlow(sample);
    await flush();
    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    expect(getMock).toHaveBeenCalledTimes(2);
  });

  it('FND-0102: a request that never settles yields a timeout error after 10s and polling resumes', async () => {
    let resolveSecond: (value: SystemOverview) => void = () => {};
    getMock.mockReturnValueOnce(new Promise(() => {}));
    // Опрос на отметке 10 с совпадает с таймаутом: второй запрос стартует сразу после него.
    getMock.mockReturnValueOnce(new Promise((resolve) => (resolveSecond = resolve)));
    render(Home);
    await flush();
    expect(getMock).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(9_999);
    await flush();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(getMock).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1);
    await flush();
    const alert = screen.getByRole('alert');
    expect(alert.textContent).toContain('Не удалось прочитать системные данные');
    expect(alert.textContent).toContain('Превышено время ожидания');
    // pending сброшен: опрос возобновился.
    expect(getMock).toHaveBeenCalledTimes(2);

    resolveSecond(sample);
    await flush();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.getByText('Загрузка: 12 %')).toBeTruthy();
  });

  it('ignores a response that arrives after unmount', async () => {
    let resolveLate: (value: SystemOverview) => void = () => {};
    getMock.mockReturnValue(new Promise((resolve) => (resolveLate = resolve)));
    render(Home);
    cleanup();

    resolveLate(sample);
    await flush();

    expect(document.body.textContent).not.toContain('Загрузка: 12 %');
    expect(consoleError).not.toHaveBeenCalled();
  });

  it('FND-0171: an error that arrives after unmount is not processed', async () => {
    let rejectLate: (reason: unknown) => void = () => {};
    getMock.mockReturnValue(new Promise((_, reject) => (rejectLate = reject)));
    render(Home);
    cleanup();

    rejectLate({ kind: 'probe', message: 'поздняя ошибка' });
    await flush();

    // Без защиты `if (active)` компонент вызвал бы toCommandError и записал состояние.
    expect(toErrorMock).not.toHaveBeenCalled();
    expect(consoleError).not.toHaveBeenCalled();
  });

  it('FND-0171: an error from a mounted component is processed (guard control)', async () => {
    getMock.mockRejectedValue({ kind: 'probe', message: 'ошибка' });
    render(Home);
    await flush();
    expect(toErrorMock).toHaveBeenCalledTimes(1);
  });
});

describe('Home tiles and graphs', () => {
  it('shows a tile for each component', async () => {
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    expect(screen.getByRole('button', { name: /Процессор/ })).toBeTruthy();
    expect(screen.getByRole('button', { name: /Видеокарта/ })).toBeTruthy();
    expect(screen.getByRole('button', { name: /Оперативная память/ })).toBeTruthy();
    expect(screen.getByRole('button', { name: /Материнская плата/ })).toBeTruthy();
    expect(screen.getByRole('button', { name: /SSD \/ HDD/ })).toBeTruthy();
    expect(screen.getByRole('button', { name: /Система/ })).toBeTruthy();
  });

  it('shows the OS name and version on the system tile', async () => {
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    expect(screen.getByText('Windows 11')).toBeTruthy();
    expect(screen.getByText('Версия: 24H2')).toBeTruthy();
  });

  it('shows the disk kind next to each disk', async () => {
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    expect(screen.getByText('SSD')).toBeTruthy();
    expect(screen.getByText('тип не определён')).toBeTruthy();
  });

  it('opens the RAM details and graph when its tile is clicked', async () => {
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    expect(screen.queryByRole('img', { name: /память/ })).toBeNull();

    const ramTile = screen.getByRole('button', { name: /Оперативная память/ });
    ramTile.click();
    await flush();
    expect(ramTile.getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByRole('img', { name: /использования памяти/ })).toBeTruthy();
    // Плитка и панель подробностей показывают один объём памяти.
    expect(screen.getAllByText('4.0 ГБ из 8.0 ГБ')).toHaveLength(2);
  });

  it('draws the CPU graph with the 100% and 90 seconds labels', async () => {
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    expect(screen.getByText('100%')).toBeTruthy();
    expect(screen.getByText('90 секунд')).toBeTruthy();
  });

  it('keeps at most 90 CPU samples in the graph history', async () => {
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    // Первый замер уже есть; ещё 99 опросов с шагом POLL_INTERVAL_MS.
    for (let i = 0; i < 99; i += 1) {
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    }
    await flush();
    const line = document.querySelector('polyline');
    expect(line).not.toBeNull();
    const points = (line?.getAttribute('points') ?? '').trim().split(/\s+/);
    expect(points).toHaveLength(90);
  });
});

describe('Home details card', () => {
  it('is closed by default, opens on a tile click and closes on a second click', async () => {
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    const reveal = document.querySelector('.reveal') as HTMLElement;
    expect(reveal.classList.contains('open')).toBe(false);
    const cpuTile = screen.getByRole('button', { name: /Процессор/ });
    cpuTile.click();
    await flush();
    expect(reveal.classList.contains('open')).toBe(true);
    cpuTile.click();
    await flush();
    expect(reveal.classList.contains('open')).toBe(false);
  });

  it('closes with the close button', async () => {
    getMock.mockResolvedValue(sample);
    render(Home);
    await flush();
    screen.getByRole('button', { name: /Оперативная память/ }).click();
    await flush();
    const reveal = document.querySelector('.reveal') as HTMLElement;
    expect(reveal.classList.contains('open')).toBe(true);
    (document.querySelector('.close') as HTMLButtonElement).click();
    await flush();
    expect(reveal.classList.contains('open')).toBe(false);
  });
});
