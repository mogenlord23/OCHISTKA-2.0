import { assessFile, scanStart, toCommandError } from './api';
import { scanCancel } from './scanControl';
import type { CommandError, ScanReport, Suspicion } from './types';

export interface Run {
  id: number;
  startedAt: string;
  target: string;
  scanned: number;
  infected: number;
}

export type ScanPhase = 'idle' | 'scanning' | 'assessing';

/**
 * Состояние проверки живёт вне компонента: переключение разделов не теряет
 * идущую проверку, её результаты и журнал.
 */
export const scan = $state({
  phase: 'idle' as ScanPhase,
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

export async function startScan(path: string): Promise<void> {
  if (scan.phase !== 'idle' || path === '') {
    return;
  }
  scan.phase = 'scanning';
  cancelRequested = false;
  scan.error = null;
  scan.notice = '';
  scan.report = null;
  scan.suspicions = {};
  scan.assessFailures = { count: 0, firstMessage: '' };
  let result: ScanReport;
  try {
    result = await scanStart(path);
  } catch (err) {
    if (cancelRequested) {
      scan.notice = 'Проверка отменена';
    } else {
      scan.error = toCommandError(err);
    }
    scan.phase = 'idle';
    return;
  }
  scan.report = result;
  scan.runs = [
    {
      id: nextRunId++,
      startedAt: new Date().toLocaleTimeString('ru-RU'),
      target: path,
      scanned: result.scannedFiles,
      infected: result.infectedFiles,
    },
    ...scan.runs,
  ];
  // Сама проверка закончилась; дальше только оценка находок.
  scan.phase = 'assessing';
  try {
    let failureCount = 0;
    let firstFailure = '';
    const scores = await Promise.all(
      result.findings.map(async (finding) => {
        try {
          return [finding.path, await assessFile(finding.path)] as const;
        } catch (err) {
          failureCount += 1;
          if (firstFailure === '') {
            firstFailure = toCommandError(err).message;
          }
          return [finding.path, null] as const;
        }
      }),
    );
    scan.assessFailures = { count: failureCount, firstMessage: firstFailure };
    const next: Record<string, Suspicion> = {};
    for (const [findingPath, score] of scores) {
      if (score) {
        next[findingPath] = score;
      }
    }
    scan.suspicions = next;
  } finally {
    scan.phase = 'idle';
  }
}

/** Просит бэкенд остановить clamscan; сама проверка завершится ошибкой, которую мы покажем как отмену. */
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
