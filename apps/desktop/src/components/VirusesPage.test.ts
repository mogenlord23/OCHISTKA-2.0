import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import VirusesPage from './VirusesPage.svelte';
import { assessFile, clamavStatus, defaultScanTarget, scanStart } from '../lib/api';
import { revealInExplorer } from '../lib/programsApi';
import {
  listQuarantine,
  purgeItem,
  quarantineFindings,
  restoreItem,
  type QuarantineEntry,
} from '../lib/disksApi';
import { scanCancel } from '../lib/scanControl';
import { resetScanState } from '../lib/scanState.svelte';
import type { ScanReport } from '../lib/types';

vi.mock('../lib/scanControl', () => ({ scanCancel: vi.fn() }));

vi.mock('../lib/programsApi', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/programsApi')>()),
  revealInExplorer: vi.fn(),
}));

vi.mock('../lib/disksApi', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/disksApi')>()),
  listQuarantine: vi.fn(),
  quarantineFindings: vi.fn(),
  purgeItem: vi.fn(),
  restoreItem: vi.fn(),
}));

vi.mock('../lib/api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/api')>()),
  clamavStatus: vi.fn(),
  defaultScanTarget: vi.fn(),
  scanStart: vi.fn(),
  assessFile: vi.fn(),
}));

const DOWNLOADS = 'C:\\Users\\Slava\\Downloads';
const START = 'Начать проверку';

beforeEach(() => {
  vi.mocked(listQuarantine).mockResolvedValue([]);
});

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
    skippedFiles: 0,
    incomplete: false,
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
      reason:
        'движок не собран: libclamav не встроен в эту сборку, см. docs/status/clamav-build.md',
    });
    vi.mocked(defaultScanTarget).mockResolvedValue(DOWNLOADS);
    render(VirusesPage, { props: { subsection: 'scan' } });

    expect(await screen.findByText('Движок ClamAV не найден')).toBeTruthy();
    expect(
      screen.getByText(
        'движок не собран: libclamav не встроен в эту сборку, см. docs/status/clamav-build.md',
      ),
    ).toBeTruthy();
    expect(
      screen.getByText(
        'Сборка libclamav не выполнена: статус и причина в docs/status/clamav-build.md.',
      ),
    ).toBeTruthy();
    const start = screen.getByRole('button', { name: START }) as HTMLButtonElement;
    expect(start.disabled).toBe(true);
  });

  it('warns that the check is incomplete when the entry limit was reached', async () => {
    engineOk();
    vi.mocked(scanStart).mockResolvedValue({ ...report(), incomplete: true });

    await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    expect(
      await screen.findByText(
        'Проверка неполная: часть папок недоступна или достигнут лимит обхода.',
      ),
    ).toBeTruthy();
  });

  it('does not show the incomplete warning for a full check', async () => {
    engineOk();
    vi.mocked(scanStart).mockResolvedValue(report());

    await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    expect(await screen.findByText('Проверено файлов: 5. Находок: 0.')).toBeTruthy();
    expect(screen.queryByText(/Проверка неполная/)).toBeNull();
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
      skippedFiles: 0,
      incomplete: false,
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
      message: 'движок не проверил файл',
    });
    await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));

    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('движок не проверил файл');
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

  it('shows real progress received while scanning', async () => {
    engineOk();
    let finish: (report: ScanReport) => void = () => {};
    vi.mocked(scanStart).mockImplementation((_path, onProgress) => {
      onProgress?.({
        totalFiles: 10,
        processedFiles: 4,
        scannedFiles: 3,
        errors: 1,
        skippedFiles: 0,
      });
      return new Promise((resolve) => (finish = resolve));
    });
    await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    expect(await screen.findByText('Обработано файлов: 4 из 10. Проверено: 3.')).toBeTruthy();
    expect(screen.getByTestId('scan-sweep').style.width).toBe('40%');
    finish(report());
    await screen.findByText('Проверено файлов: 5. Находок: 0.');
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
    expect(scanStart).toHaveBeenCalledWith(DOWNLOADS, expect.any(Function));

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
    vi.mocked(clamavStatus).mockResolvedValue({ available: false, reason: 'нет движка' });
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

  it.each([
    { incomplete: true, errors: 0, skippedFiles: 0 },
    { incomplete: false, errors: 1, skippedFiles: 0 },
    { incomplete: false, errors: 0, skippedFiles: 1 },
  ])('does not call a partial scan clean', async (partial) => {
    engineOk();
    vi.mocked(scanStart).mockResolvedValue({ ...report([]), ...partial });
    const view = await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    await screen.findByText('Проверено файлов: 5. Находок: 0.');
    await view.rerender({ subsection: 'findings' });
    expect(screen.getByText('Находок нет. Часть файлов или папок не проверена.')).toBeTruthy();
    expect(screen.queryByText('Угроз не найдено.')).toBeNull();
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

describe('VirusesPage scan screen (round 28)', () => {
  async function twoFindings() {
    const a = `${DOWNLOADS}\\a.exe`;
    const b = `${DOWNLOADS}\\b.exe`;
    engineOk();
    vi.mocked(scanStart).mockResolvedValue(
      report([
        { path: a, signature: 'Win.A.Test' },
        { path: b, signature: 'Win.B.Test' },
      ]),
    );
    vi.mocked(assessFile).mockResolvedValue({
      score: 20,
      reasons: ['исполняемый файл в загрузках'],
    });
    const view = await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    await screen.findByText('Проверено файлов: 5. Находок: 2.');
    return { view, a, b };
  }

  it('shows the station, the findings panel on the right and the bin with the report count', async () => {
    const { a } = await twoFindings();

    expect(screen.getByTestId('scan-station')).toBeTruthy();
    expect(screen.getByTestId('findings-list')).toBeTruthy();
    expect(screen.getByTestId('virus-bin')).toBeTruthy();
    expect(screen.getByTestId('bin-count').textContent).toBe('2');
    // Находки видны в правой панели, а не только в подразделе «Находки».
    expect(screen.getByText('Win.A.Test')).toBeTruthy();
    expect(screen.getByText(a)).toBeTruthy();
  });

  it('counts selected findings and keeps the choice when switching subsections', async () => {
    const { view } = await twoFindings();

    expect(screen.getByText('Выбрано: 0')).toBeTruthy();
    await fireEvent.click(screen.getAllByRole('checkbox')[0]);
    expect(screen.getByText('Выбрано: 1')).toBeTruthy();

    await view.rerender({ subsection: 'findings' });
    expect(screen.getByText('Выбрано: 1')).toBeTruthy();
    const boxes = screen.getAllByRole('checkbox') as HTMLInputElement[];
    expect(boxes[0].checked).toBe(true);
    expect(boxes[1].checked).toBe(false);

    await view.rerender({ subsection: 'scan' });
    expect(screen.getByText('Выбрано: 1')).toBeTruthy();
  });

  it('unselects a finding when its checkbox is clicked again', async () => {
    await twoFindings();
    const box = screen.getAllByRole('checkbox')[0];
    await fireEvent.click(box);
    expect(screen.getByText('Выбрано: 1')).toBeTruthy();
    await fireEvent.click(box);
    expect(screen.getByText('Выбрано: 0')).toBeTruthy();
  });
});

describe('VirusesPage scan screen motion (round 28)', () => {
  const REDUCED = {
    matches: true,
    media: '(prefers-reduced-motion: reduce)',
    addEventListener: () => {},
    removeEventListener: () => {},
  };

  afterEach(() => {
    vi.unstubAllGlobals();
    delete document.documentElement.dataset.reduceMotion;
  });

  /** Находки после проверки: становится видно, что полёт закончился. */
  async function scanWithOneFinding(): Promise<void> {
    engineOk();
    vi.mocked(scanStart).mockResolvedValue(
      report([{ path: `${DOWNLOADS}\\evil.exe`, signature: 'Win.Evil.Test' }]),
    );
    vi.mocked(assessFile).mockResolvedValue({ score: 10, reasons: [] });
    await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    await screen.findByText('Проверено файлов: 5. Находок: 1.');
  }

  it('shows the beam only while scanning and marks the station state', async () => {
    engineOk();
    let finish: (value: ScanReport) => void = () => {};
    vi.mocked(scanStart).mockReturnValue(new Promise((resolve) => (finish = resolve)));

    await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));

    const station = screen.getByTestId('scan-station');
    expect(station.dataset.state).toBe('scanning');
    expect(screen.getByTestId('scan-beam')).toBeTruthy();

    finish(report());
    await screen.findByText('Проверено файлов: 5. Находок: 0.');
    expect(screen.queryByTestId('scan-beam')).toBeNull();
    expect(screen.getByTestId('scan-station').dataset.state).toBe('clean');
  });

  it('flies findings into the bin, then closes the lid and keeps the count', async () => {
    await scanWithOneFinding();

    expect(screen.getByTestId('bin-flight')).toBeTruthy();
    expect(screen.getByTestId('virus-bin').dataset.state).toBe('filling');
    expect(screen.getByTestId('bin-count').textContent).toBe('1');

    await waitFor(() => expect(screen.queryByTestId('bin-flight')).toBeNull(), { timeout: 2000 });
    expect(screen.getByTestId('virus-bin').dataset.state).toBe('closed');
    expect(screen.getByTestId('bin-count').textContent).toBe('1');
  });

  it('keeps everything still when the system asks for less motion', async () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn(() => REDUCED),
    );
    await scanWithOneFinding();

    expect(screen.queryByTestId('bin-flight')).toBeNull();
    expect(screen.getByTestId('virus-bin').dataset.state).toBe('closed');
    expect(screen.getByTestId('bin-count').textContent).toBe('1');
  });

  it('keeps everything still with the in-app reduce-motion switch', async () => {
    document.documentElement.dataset.reduceMotion = 'true';
    await scanWithOneFinding();

    expect(screen.queryByTestId('bin-flight')).toBeNull();
    expect(screen.getByTestId('virus-bin').dataset.state).toBe('closed');
  });
});

describe('VirusesPage findings context menu (round 28)', () => {
  const FINDING = `${DOWNLOADS}\\evil.exe`;

  async function scanOne(): Promise<void> {
    engineOk();
    vi.mocked(scanStart).mockResolvedValue(report([{ path: FINDING, signature: 'Win.Evil.Test' }]));
    vi.mocked(assessFile).mockResolvedValue({ score: 5, reasons: [] });
    const view = await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    await screen.findByText('Проверено файлов: 5. Находок: 1.');
    await view.rerender({ subsection: 'findings' });
    await screen.findByTestId('finding');
  }

  it('opens the menu with both items on right click and reveals the parent folder', async () => {
    await scanOne();
    await fireEvent.contextMenu(screen.getByTestId('finding'), { clientX: 120, clientY: 90 });

    const menu = await screen.findByRole('menu');
    expect(within(menu).getByRole('menuitem', { name: 'Открыть папку' })).toBeTruthy();
    expect(within(menu).getByRole('menuitem', { name: 'Скопировать путь' })).toBeTruthy();

    await fireEvent.click(within(menu).getByRole('menuitem', { name: 'Открыть папку' }));
    await waitFor(() => expect(revealInExplorer).toHaveBeenCalledWith(DOWNLOADS));
  });

  it('explains a failed reveal with a plain line, without windows or codes', async () => {
    vi.mocked(revealInExplorer).mockRejectedValue({ kind: 'io', message: 'Папка недоступна' });
    await scanOne();
    await fireEvent.contextMenu(screen.getByTestId('finding'), { clientX: 120, clientY: 90 });
    const menu = await screen.findByRole('menu');
    await fireEvent.click(within(menu).getByRole('menuitem', { name: 'Открыть папку' }));

    const line = await screen.findByTestId('finding-action');
    expect(line.textContent).toBe('Папка недоступна');
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('copies the full path', async () => {
    const writeText = vi.fn(async () => {});
    Object.assign(navigator, { clipboard: { writeText } });
    await scanOne();
    await fireEvent.contextMenu(screen.getByTestId('finding'), { clientX: 120, clientY: 90 });
    const menu = await screen.findByRole('menu');
    await fireEvent.click(within(menu).getByRole('menuitem', { name: 'Скопировать путь' }));

    await waitFor(() => expect(writeText).toHaveBeenCalledWith(FINDING));
    expect((await screen.findByTestId('finding-action')).textContent).toBe('Путь скопирован.');
  });

  it('opens the menu from the keyboard and returns focus on Escape', async () => {
    await scanOne();
    const box = screen.getAllByRole('checkbox')[0] as HTMLInputElement;
    box.focus();

    await fireEvent.keyDown(box, { key: 'F10', shiftKey: true });
    const menu = await screen.findByRole('menu');
    expect(within(menu).getByRole('menuitem', { name: 'Открыть папку' })).toBeTruthy();

    await fireEvent.keyDown(menu, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('menu')).toBeNull());
    expect(document.activeElement).toBe(box);
  });
});

describe('VirusesPage findings: selection and quarantine', () => {
  it('keeps quarantine unavailable even after selecting a finding', async () => {
    engineOk();
    const a = `${DOWNLOADS}\\a.exe`;
    const b = `${DOWNLOADS}\\b.exe`;
    vi.mocked(scanStart).mockResolvedValue(
      report([
        { path: a, signature: 'Win.A.Test' },
        { path: b, signature: 'Win.B.Test' },
      ]),
    );
    vi.mocked(assessFile).mockResolvedValue({
      score: 30,
      reasons: ['исполняемый файл в загрузках'],
    });
    vi.mocked(quarantineFindings).mockResolvedValue({
      moved: 1,
      bytes: 1,
      failed: 0,
      failures: [],
    });

    const view = await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    await screen.findByText('Проверено файлов: 5. Находок: 2.');
    await view.rerender({ subsection: 'findings' });
    await screen.findAllByText(/Подсказка: 30\/100/);

    const move = screen.getByRole('button', { name: 'Удалить' }) as HTMLButtonElement;
    expect(move.disabled).toBe(true);
    await fireEvent.click(screen.getAllByRole('checkbox')[0]);
    expect(move.disabled).toBe(true);
    await fireEvent.click(move);
    expect(quarantineFindings).not.toHaveBeenCalled();
    expect(screen.getByText('Перенос находок в карантин временно недоступен.')).toBeTruthy();
    expect(screen.getByText('Win.A.Test')).toBeTruthy();
    expect(screen.getByText('Win.B.Test')).toBeTruthy();
    expect(screen.getByText('Причина: сигнатура Win.B.Test')).toBeTruthy();
  });

  it('shows failed attempts in the journal', async () => {
    engineOk();
    vi.mocked(scanStart).mockRejectedValue({ kind: 'probe', message: 'файл недоступен' });
    await renderReady();
    await fireEvent.click(screen.getByRole('button', { name: START }));
    await screen.findByRole('alert');

    const view = render(VirusesPage, { props: { subsection: 'log' } });
    const items = await screen.findAllByRole('listitem');
    expect(items).toHaveLength(1);
    expect(items[0].textContent).toContain('ошибка: файл недоступен');
    view.unmount();
  });

  it('permanent deletion needs a second confirmation and then uses the entry token', async () => {
    const entry = {
      id: '0123456789abcdef',
      originalPath: `${DOWNLOADS}\\x.exe`,
      size: 1,
      sha256: '',
      movedAtUnix: 1,
      reason: 'проверка «Вирусы»: сигнатура Win.X',
      purgeToken: 'purge:0123456789abcdef',
    } as QuarantineEntry;
    vi.mocked(listQuarantine).mockResolvedValue([entry]);
    vi.mocked(purgeItem).mockResolvedValue(entry);
    vi.mocked(restoreItem).mockResolvedValue(entry);

    render(VirusesPage, { props: { subsection: 'log' } });
    await fireEvent.click(await screen.findByRole('button', { name: 'Удалить окончательно' }));
    expect(purgeItem).not.toHaveBeenCalled();

    await fireEvent.click(screen.getByRole('button', { name: 'Подтвердить удаление' }));
    await waitFor(() => expect(purgeItem).toHaveBeenCalledWith(entry.id, entry.purgeToken));
  });
});
