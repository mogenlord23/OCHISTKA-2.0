import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import VirusesPage from './VirusesPage.svelte';
import { assessFile, clamavStatus, defaultScanTarget, scanStart } from '../lib/api';
import { scanCancel } from '../lib/scanControl';
import { resetScanState } from '../lib/scanState.svelte';
import type { ScanReport } from '../lib/types';

vi.mock('../lib/scanControl', () => ({ scanCancel: vi.fn() }));

vi.mock('../lib/api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/api')>()),
  clamavStatus: vi.fn(),
  defaultScanTarget: vi.fn(),
  scanStart: vi.fn(),
  assessFile: vi.fn(),
}));

const DOWNLOADS = 'C:\\Users\\Slava\\Downloads';
const START = 'Начать проверку';

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.resetAllMocks();
  // Состояние проверки теперь общее на модуль: сбрасываем между тестами.
  resetScanState();
});

function engineOk(target: string = DOWNLOADS): void {
  vi.mocked(clamavStatus).mockResolvedValue({ available: true, reason: '' });
  vi.mocked(defaultScanTarget).mockResolvedValue(target);
}

function report(findings: ScanReport['findings'] = []): ScanReport {
  return {
    scannedFiles: 5,
    infectedFiles: findings.length,
    errors: 0,
    findings,
  };
}

/** Рендерит страницу и ждёт, пока кнопка запуска станет доступной. */
async function renderReady(subsection = 'scan') {
  const view = render(VirusesPage, { props: { subsection } });
  await waitFor(() =>
    expect((screen.getByRole('button', { name: START }) as HTMLButtonElement).disabled).toBe(false),
  );
  return view;
}

describe('VirusesPage scan', () => {
  it('FND-0182: blocks scanning and shows the engine reason and install hint', async () => {
    vi.mocked(clamavStatus).mockResolvedValue({
      available: false,
      reason: 'C:\\app\\clamav\\clamscan.exe не найден',
    });
    vi.mocked(defaultScanTarget).mockResolvedValue(DOWNLOADS);
    render(VirusesPage, { props: { subsection: 'scan' } });

    expect(await screen.findByText('Движок ClamAV не найден')).toBeTruthy();
    expect(screen.getByText('C:\\app\\clamav\\clamscan.exe не найден')).toBeTruthy();
    expect(screen.getByText('Положите ClamAV в папку «clamav» рядом с приложением.')).toBeTruthy();
    const start = screen.getByRole('button', { name: START }) as HTMLButtonElement;
    expect(start.disabled).toBe(true);
  });

  it('shows the sweep while scanning, then the result and suspicion score', async () => {
    engineOk();
    let finish: (report: ScanReport) => void = () => {};
    vi.mocked(scanStart).mockReturnValue(new Promise((resolve) => (finish = resolve)));
    vi.mocked(assessFile).mockResolvedValue({
      score: 70,
      reasons: ['двойное расширение: документ, замаскированный под программу'],
    });

    const view = await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    expect(screen.getByTestId('scan-sweep')).toBeTruthy();

    finish({
      scannedFiles: 5,
      infectedFiles: 1,
      errors: 0,
      findings: [{ path: 'C:\\Users\\Slava\\Downloads\\invoice.pdf.exe', signature: 'Win.Test.X' }],
    });
    expect(await screen.findByText('Проверено файлов: 5. Находок: 1.')).toBeTruthy();
    expect(screen.queryByTestId('scan-sweep')).toBeNull();
    await waitFor(() =>
      expect(assessFile).toHaveBeenCalledWith('C:\\Users\\Slava\\Downloads\\invoice.pdf.exe'),
    );

    await view.rerender({ subsection: 'findings' });
    expect(screen.getByText('Win.Test.X')).toBeTruthy();
    expect(await screen.findByText(/Подсказка: 70\/100/)).toBeTruthy();
    const remove = screen.getByRole('button', { name: 'Удалить' }) as HTMLButtonElement;
    expect(remove.disabled).toBe(true);
  });

  it('FND-0181: shows an alert when clamavStatus rejects', async () => {
    vi.mocked(clamavStatus).mockRejectedValue({ kind: 'probe', message: 'ClamAV не отвечает' });
    vi.mocked(defaultScanTarget).mockResolvedValue(DOWNLOADS);
    render(VirusesPage, { props: { subsection: 'scan' } });

    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('ClamAV не отвечает');
  });

  it('FND-0094: scanStart rejecting shows the error message, not silence', async () => {
    engineOk();
    vi.mocked(scanStart).mockRejectedValue({
      kind: 'invalidTarget',
      message: 'clamscan завершился с ошибкой',
    });
    await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));

    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('clamscan завершился с ошибкой');
    expect(screen.queryByTestId('scan-sweep')).toBeNull();
    // Кнопка снова доступна после ошибки.
    await waitFor(() =>
      expect((screen.getByRole('button', { name: START }) as HTMLButtonElement).disabled).toBe(
        false,
      ),
    );
  });

  it('FND-0181/FND-0094: assessFile failing for one finding shows a notice and the others still score', async () => {
    engineOk();
    const bad = 'C:\\Users\\Slava\\Downloads\\bad.exe';
    const good = 'C:\\Users\\Slava\\Downloads\\good.pdf.exe';
    vi.mocked(scanStart).mockResolvedValue(
      report([
        { path: bad, signature: 'Win.Bad.A' },
        { path: good, signature: 'Win.Good.B' },
      ]),
    );
    vi.mocked(assessFile).mockImplementation(async (path: string) => {
      if (path === bad) {
        throw { kind: 'probe', message: 'файл недоступен' };
      }
      return { score: 40, reasons: ['исполняемый файл в загрузках'] };
    });

    const view = await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    expect(await screen.findByText('Проверено файлов: 5. Находок: 2.')).toBeTruthy();

    await view.rerender({ subsection: 'findings' });
    const notice = await screen.findByText(/Не удалось оценить файлов/);
    expect(notice.textContent).toContain('Не удалось оценить файлов: 1');
    expect(notice.textContent).toContain('файл недоступен');
    // Сканирование завершилось, второй файл оценён.
    expect(screen.getByText('Win.Bad.A')).toBeTruthy();
    expect(screen.getByText('Win.Good.B')).toBeTruthy();
    expect(await screen.findByText(/Подсказка: 40\/100/)).toBeTruthy();
    expect(screen.getAllByRole('meter')).toHaveLength(1);
  });

  it('FND-0183: while scanning the button and input are disabled and scanStart is called once', async () => {
    engineOk();
    let finish: (report: ScanReport) => void = () => {};
    vi.mocked(scanStart).mockReturnValue(new Promise((resolve) => (finish = resolve)));
    await renderReady();

    const button = screen.getByRole('button', { name: START }) as HTMLButtonElement;
    const input = screen.getByLabelText('Папка для проверки') as HTMLInputElement;
    await fireEvent.click(button);

    expect(await screen.findByTestId('scan-sweep')).toBeTruthy();
    expect(button.disabled).toBe(true);
    expect(input.disabled).toBe(true);
    await fireEvent.click(button);
    expect(scanStart).toHaveBeenCalledTimes(1);
    expect(scanStart).toHaveBeenCalledWith(DOWNLOADS);

    finish(report());
    expect(await screen.findByText('Проверено файлов: 5. Находок: 0.')).toBeTruthy();
    expect(button.disabled).toBe(false);
    expect(input.disabled).toBe(false);
  });

  it('FND-0183: empty or whitespace target does not call scanStart', async () => {
    engineOk('');
    render(VirusesPage, { props: { subsection: 'scan' } });
    const button = (await screen.findByRole('button', { name: START })) as HTMLButtonElement;
    await screen.findByText('Укажите папку для проверки');
    await fireEvent.click(button);
    expect(scanStart).not.toHaveBeenCalled();

    const input = screen.getByLabelText('Папка для проверки') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: '   ' } });
    expect(button.disabled).toBe(true);
    await fireEvent.click(button);
    expect(scanStart).not.toHaveBeenCalled();
  });

  it('FND-0097: shows the visible reason when the engine is unavailable', async () => {
    vi.mocked(clamavStatus).mockResolvedValue({ available: false, reason: 'нет clamscan' });
    vi.mocked(defaultScanTarget).mockResolvedValue(DOWNLOADS);
    render(VirusesPage, { props: { subsection: 'scan' } });

    expect(await screen.findByText('Движок ClamAV не найден')).toBeTruthy();
    expect(screen.getByText('Сначала проверьте движок')).toBeTruthy();
    expect(screen.queryByText('Укажите папку для проверки')).toBeNull();
  });

  it('FND-0097: shows the visible reason when the target is empty, and hides it when valid', async () => {
    engineOk('');
    render(VirusesPage, { props: { subsection: 'scan' } });

    expect(await screen.findByText('Укажите папку для проверки')).toBeTruthy();
    expect(screen.queryByText('Сначала проверьте движок')).toBeNull();

    const input = screen.getByLabelText('Папка для проверки') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: DOWNLOADS } });
    await waitFor(() => expect(screen.queryByText('Укажите папку для проверки')).toBeNull());
  });
});

describe('VirusesPage findings', () => {
  it('FND-0186: shows the no-report state before any scan', async () => {
    engineOk();
    render(VirusesPage, { props: { subsection: 'findings' } });
    expect(await screen.findByText('Здесь появятся находки после проверки.')).toBeTruthy();
    expect(screen.queryByText('Угроз не найдено.')).toBeNull();
  });

  it('FND-0186: shows "Угроз не найдено" for an empty findings list', async () => {
    engineOk();
    vi.mocked(scanStart).mockResolvedValue(report([]));
    const view = await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    await screen.findByText('Проверено файлов: 5. Находок: 0.');

    await view.rerender({ subsection: 'findings' });
    expect(screen.getByText('Угроз не найдено.')).toBeTruthy();
    expect(screen.queryByText('Здесь появятся находки после проверки.')).toBeNull();
    expect(screen.queryAllByRole('meter')).toHaveLength(0);
  });

  it('FND-0186: meter aria-valuenow and the reason text under the score', async () => {
    engineOk();
    vi.mocked(scanStart).mockResolvedValue(
      report([{ path: `${DOWNLOADS}\\invoice.pdf.exe`, signature: 'Win.Test.X' }]),
    );
    vi.mocked(assessFile).mockResolvedValue({
      score: 70,
      reasons: ['двойное расширение', 'исполняемый файл в загрузках'],
    });
    const view = await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    await screen.findByText('Проверено файлов: 5. Находок: 1.');

    await view.rerender({ subsection: 'findings' });
    const meter = await screen.findByRole('meter');
    expect(meter.getAttribute('aria-valuenow')).toBe('70');
    expect(meter.getAttribute('aria-valuemin')).toBe('0');
    expect(meter.getAttribute('aria-valuemax')).toBe('100');
    expect(
      screen.getByText('Подсказка: 70/100 — двойное расширение; исполняемый файл в загрузках'),
    ).toBeTruthy();
  });

  it('FND-0092: two findings with the same path both render (no each_key_duplicate)', async () => {
    engineOk();
    const path = `${DOWNLOADS}\\dup.exe`;
    vi.mocked(scanStart).mockResolvedValue(
      report([
        { path, signature: 'Win.Dup.A' },
        { path, signature: 'Win.Dup.B' },
      ]),
    );
    vi.mocked(assessFile).mockResolvedValue({ score: 5, reasons: [] });
    const errors = vi.spyOn(console, 'error').mockImplementation(() => {});

    const view = await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    await screen.findByText('Проверено файлов: 5. Находок: 2.');

    await view.rerender({ subsection: 'findings' });
    expect(await screen.findByText('Win.Dup.A')).toBeTruthy();
    expect(screen.getByText('Win.Dup.B')).toBeTruthy();
    expect(screen.getAllByText(path)).toHaveLength(2);
    expect(errors).not.toHaveBeenCalled();
    errors.mockRestore();
  });

  it('FND-0186: shows a fallback when the score has no reasons', async () => {
    engineOk();
    vi.mocked(scanStart).mockResolvedValue(
      report([{ path: `${DOWNLOADS}\\a.exe`, signature: 'Win.Test.Y' }]),
    );
    vi.mocked(assessFile).mockResolvedValue({ score: 0, reasons: [] });
    const view = await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    await screen.findByText('Проверено файлов: 5. Находок: 1.');

    await view.rerender({ subsection: 'findings' });
    expect(await screen.findByText('Подсказка: 0/100 — признаков не найдено')).toBeTruthy();
    expect((await screen.findByRole('meter')).getAttribute('aria-valuenow')).toBe('0');
  });
});

describe('VirusesPage journal', () => {
  it('FND-0180: shows the empty state before any scan', async () => {
    engineOk();
    render(VirusesPage, { props: { subsection: 'log' } });
    expect(await screen.findByText('Проверок в этой сессии пока не было.')).toBeTruthy();
    expect(screen.queryByRole('list')).toBeNull();
  });

  it('FND-0180: a completed scan appears in the journal list', async () => {
    engineOk();
    vi.mocked(scanStart).mockResolvedValue(
      report([{ path: `${DOWNLOADS}\\a.exe`, signature: 'Win.Test.Z' }]),
    );
    vi.mocked(assessFile).mockResolvedValue({ score: 10, reasons: [] });
    const view = await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    await screen.findByText('Проверено файлов: 5. Находок: 1.');

    await view.rerender({ subsection: 'log' });
    expect(screen.queryByText('Проверок в этой сессии пока не было.')).toBeNull();
    const items = screen.getAllByRole('listitem');
    expect(items).toHaveLength(1);
    expect(items[0].textContent).toContain(DOWNLOADS);
    expect(items[0].textContent).toContain('проверено 5, находок 1');
  });

  it('FND-0093: two runs of the same target in the same second both render (no each_key_duplicate)', async () => {
    // Фиксируем время, чтобы startedAt совпал у обоих запусков.
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date('2026-11-18T10:00:00Z'));
    engineOk();
    vi.mocked(scanStart).mockResolvedValue(report());
    const errors = vi.spyOn(console, 'error').mockImplementation(() => {});

    const view = await renderReady();
    const button = screen.getByRole('button', { name: START }) as HTMLButtonElement;
    await fireEvent.click(button);
    await screen.findByText('Проверено файлов: 5. Находок: 0.');
    await waitFor(() => expect(button.disabled).toBe(false));
    await fireEvent.click(button);
    await waitFor(() => expect(scanStart).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(button.disabled).toBe(false));

    await view.rerender({ subsection: 'log' });
    const items = screen.getAllByRole('listitem');
    expect(items).toHaveLength(2);
    expect(items[0].textContent).toBe(items[1].textContent);
    expect(errors).not.toHaveBeenCalled();
    errors.mockRestore();
  });
});

describe('VirusesPage shared scan state (FND-0018, FND-0098, FND-0026)', () => {
  it('FND-0018: a running scan and its results survive unmounting the page', async () => {
    engineOk();
    let finish: (report: ScanReport) => void = () => {};
    vi.mocked(scanStart).mockReturnValue(new Promise((resolve) => (finish = resolve)));
    await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    expect(await screen.findByTestId('scan-sweep')).toBeTruthy();

    cleanup();
    render(VirusesPage, { props: { subsection: 'scan' } });
    expect(await screen.findByTestId('scan-sweep')).toBeTruthy();
    expect((screen.getByRole('button', { name: START }) as HTMLButtonElement).disabled).toBe(true);

    finish(report());
    expect(await screen.findByText('Проверено файлов: 5. Находок: 0.')).toBeTruthy();
    cleanup();
    render(VirusesPage, { props: { subsection: 'log' } });
    expect(screen.getAllByRole('listitem')).toHaveLength(1);
    expect(scanStart).toHaveBeenCalledTimes(1);
  });

  it('FND-0098: assessing phase has its own text and no scan sweep', async () => {
    engineOk();
    let finishAssess: (value: { score: number; reasons: string[] }) => void = () => {};
    vi.mocked(scanStart).mockResolvedValue(
      report([{ path: `${DOWNLOADS}\\a.exe`, signature: 'S' }]),
    );
    vi.mocked(assessFile).mockReturnValue(new Promise((resolve) => (finishAssess = resolve)));
    await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));

    expect(await screen.findByText('Оцениваем находки…')).toBeTruthy();
    expect(screen.queryByText('Проверяем файлы…')).toBeNull();
    expect(screen.queryByTestId('scan-sweep')).toBeNull();
    expect((screen.getByRole('button', { name: START }) as HTMLButtonElement).disabled).toBe(true);

    finishAssess({ score: 1, reasons: [] });
    expect(await screen.findByText('Проверено файлов: 5. Находок: 1.')).toBeTruthy();
  });

  it('FND-0026: the stop button cancels the scan and shows a cancelled notice', async () => {
    engineOk();
    let fail: (err: unknown) => void = () => {};
    vi.mocked(scanStart).mockReturnValue(new Promise((_, reject) => (fail = reject)));
    vi.mocked(scanCancel).mockResolvedValue(true);
    await renderReady();
    expect(screen.queryByRole('button', { name: 'Остановить' })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: START }));

    await fireEvent.click(await screen.findByRole('button', { name: 'Остановить' }));
    expect(scanCancel).toHaveBeenCalledTimes(1);
    fail({ kind: 'probe', message: 'ошибка чтения системных данных: проверка отменена' });

    expect(await screen.findByText('Проверка отменена')).toBeTruthy();
    expect(screen.queryByRole('alert')).toBeNull();
    await waitFor(() =>
      expect((screen.getByRole('button', { name: START }) as HTMLButtonElement).disabled).toBe(
        false,
      ),
    );
  });
});
