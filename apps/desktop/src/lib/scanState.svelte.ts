import { assessFile, scanStart, toCommandError, type ScanProgress } from './api';
import { quarantineFindings } from './disksApi';
import { scanCancel } from './scanControl';
import type { CommandError, Finding, ScanReport, Suspicion } from './types';

/** Одна попытка проверки: успешная, отменённая или с ошибкой (журнал показывает все). */
export interface Run {
  id: number;
  startedAt: string;
  target: string;
  status: 'done' | 'cancelled' | 'failed';
  scanned: number;
  infected: number;
  skipped: number;
  /** Причина для отменённой или неудачной попытки. */
  message: string;
}

export type ScanPhase = 'idle' | 'scanning' | 'assessing';

/**
 * Состояние проверки живёт вне компонента: переключение разделов не теряет
 * идущую проверку, её результаты и журнал.
 */
export const scan = $state({
  phase: 'idle' as ScanPhase,
  progress: null as ScanProgress | null,
  target: '',
  report: null as ScanReport | null,
  error: null as CommandError | null,
  notice: '',
  suspicions: {} as Record<string, Suspicion>,
  runs: [] as Run[],
  assessFailures: { count: 0, firstMessage: '' },
});

let nextRunId = 0;
let cancelRequested = false;

/** Сбрасывает всё состояние (для тестов). */
export function resetScanState(): void {
  scan.phase = 'idle';
  scan.progress = null;
  scan.target = '';
  scan.report = null;
  scan.error = null;
  scan.notice = '';
  scan.suspicions = {};
  scan.runs = [];
  scan.assessFailures = { count: 0, firstMessage: '' };
  nextRunId = 0;
  cancelRequested = false;
}

function now(): string {
  return new Date().toLocaleTimeString('ru-RU');
}

function addRun(run: Omit<Run, 'id' | 'startedAt'>): void {
  scan.runs = [{ id: nextRunId++, startedAt: now(), ...run }, ...scan.runs];
}

export async function startScan(path: string | null): Promise<void> {
  if (scan.phase !== 'idle' || path === '') {
    return;
  }
  scan.phase = 'scanning';
  const target = path ?? 'Компьютер';
  scan.progress = null;
  cancelRequested = false;
  scan.error = null;
  scan.notice = '';
  scan.report = null;
  scan.suspicions = {};
  scan.assessFailures = { count: 0, firstMessage: '' };
  let result: ScanReport;
  let acceptingProgress = true;
  try {
    result = await scanStart(path, (progress) => {
      if (acceptingProgress && scan.phase === 'scanning') scan.progress = progress;
    });
  } catch (err) {
    acceptingProgress = false;
    const command = toCommandError(err);
    if (cancelRequested || command.kind === 'cancelled') {
      scan.notice = 'Проверка отменена';
      addRun({
        target,
        status: 'cancelled',
        scanned: 0,
        infected: 0,
        skipped: 0,
        message: 'отменена пользователем',
      });
    } else {
      scan.error = command;
      addRun({
        target,
        status: 'failed',
        scanned: 0,
        infected: 0,
        skipped: 0,
        message: command.message,
      });
    }
    scan.phase = 'idle';
    return;
  }
  acceptingProgress = false;
  scan.progress = null;
  scan.report = result;
  addRun({
    target,
    status: 'done',
    scanned: result.scannedFiles,
    infected: result.infectedFiles,
    skipped: result.skippedFiles,
    message: '',
  });
  // Сама проверка закончилась; дальше только оценка находок.
  scan.phase = 'assessing';
  try {
    let failureCount = 0;
    let firstFailure = '';
    const next: Record<string, Suspicion> = {};
    for (const finding of result.findings) {
      try {
        next[finding.path] = await assessFile(finding.path);
      } catch (err) {
        failureCount += 1;
        if (firstFailure === '') firstFailure = toCommandError(err).message;
      }
    }
    scan.assessFailures = { count: failureCount, firstMessage: firstFailure };
    scan.suspicions = next;
  } finally {
    scan.phase = 'idle';
  }
}

/** Просит бэкенд остановить проверку; сама проверка завершится ошибкой, которую мы покажем как отмену. */
export async function cancelScan(): Promise<void> {
  if (scan.phase !== 'scanning') {
    return;
  }
  cancelRequested = true;
  try {
    await scanCancel();
  } catch (err) {
    cancelRequested = false;
    scan.error = toCommandError(err);
  }
}

/**
 * Переносит выбранные находки в карантин. Удаления нет: файлы попадают в карантин по ADR 0005.
 * Из отчёта убираются только те находки, которые реально перенесены.
 */
export async function quarantineSelected(selected: Finding[]): Promise<number> {
  if (selected.length === 0) {
    return 0;
  }
  const items = selected.map((finding) => {
    const reasons = scan.suspicions[finding.path]?.reasons ?? [];
    const reason = [`сигнатура ${finding.signature}`, ...reasons].join('; ');
    return { path: finding.path, reason };
  });
  const outcome = await quarantineFindings(items);
  const failed = new Set(outcome.failures.map((failure) => failure.path));
  const moved = new Set(selected.filter((finding) => !failed.has(finding.path)));
  if (scan.report) {
    scan.report = {
      ...scan.report,
      findings: scan.report.findings.filter((finding) => !moved.has(finding)),
    };
  }
  scan.notice =
    outcome.failed > 0
      ? `В карантин: ${outcome.moved}. Не перенесено: ${outcome.failed}. Причина: ${outcome.failures[0]?.message ?? ''}`
      : `В карантин: ${outcome.moved}.`;
  return outcome.moved;
}
