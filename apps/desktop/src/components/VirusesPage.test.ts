import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import VirusesPage from './VirusesPage.svelte';
import { assessFile, clamavStatus, defaultScanTarget, scanStart } from '../lib/api';
import type { ScanReport } from '../lib/types';

vi.mock('../lib/api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/api')>()),
  clamavStatus: vi.fn(),
  defaultScanTarget: vi.fn(),
  scanStart: vi.fn(),
  assessFile: vi.fn(),
}));

/** Даёт промисам и эффектам Svelte выполниться без таймеров. */
async function flush(): Promise<void> {
  await tick();
  for (let i = 0; i < 5; i++) await Promise.resolve();
  await tick();
}

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

describe('VirusesPage scan', () => {
  it('blocks scanning and explains when the ClamAV engine is missing', async () => {
    vi.mocked(clamavStatus).mockResolvedValue({
      available: false,
      reason: 'C:\\app\\clamav\\clamscan.exe не найден',
    });
    vi.mocked(defaultScanTarget).mockResolvedValue('C:\\Users\\Slava\\Downloads');
    render(VirusesPage, { props: { subsection: 'scan' } });
    await flush();

    expect(screen.getByText('Движок ClamAV не найден')).toBeTruthy();
    const start = screen.getByRole('button', { name: 'Начать проверку' }) as HTMLButtonElement;
    expect(start.disabled).toBe(true);
  });

  it('shows the sweep while scanning, then the result and suspicion score', async () => {
    vi.mocked(clamavStatus).mockResolvedValue({ available: true, reason: '' });
    vi.mocked(defaultScanTarget).mockResolvedValue('C:\\Users\\Slava\\Downloads');
    let finish: (report: ScanReport) => void = () => {};
    vi.mocked(scanStart).mockReturnValue(new Promise((resolve) => (finish = resolve)));
    vi.mocked(assessFile).mockResolvedValue({
      score: 70,
      reasons: ['двойное расширение: документ, замаскированный под программу'],
    });

    const view = render(VirusesPage, { props: { subsection: 'scan' } });
    await flush();
    await fireEvent.click(screen.getByRole('button', { name: 'Начать проверку' }));
    expect(screen.getByTestId('scan-sweep')).toBeTruthy();

    finish({
      scannedFiles: 5,
      infectedFiles: 1,
      errors: 0,
      findings: [{ path: 'C:\\Users\\Slava\\Downloads\\invoice.pdf.exe', signature: 'Win.Test.X' }],
    });
    await flush();
    expect(screen.queryByTestId('scan-sweep')).toBeNull();
    expect(screen.getByText('Проверено файлов: 5. Находок: 1.')).toBeTruthy();
    expect(assessFile).toHaveBeenCalledWith('C:\\Users\\Slava\\Downloads\\invoice.pdf.exe');

    await view.rerender({ subsection: 'findings' });
    expect(screen.getByText('Win.Test.X')).toBeTruthy();
    expect(screen.getByText(/Подсказка: 70\/100/)).toBeTruthy();
    const remove = screen.getByRole('button', { name: 'Удалить' }) as HTMLButtonElement;
    expect(remove.disabled).toBe(true);
  });
});
