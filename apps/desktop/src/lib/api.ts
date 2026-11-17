import { invoke } from '@tauri-apps/api/core';
import type {
  CommandError,
  EngineStatus,
  ErrorKind,
  ScanReport,
  Suspicion,
  SystemOverview,
} from './types';

/** Интервал опроса раздела «Компьютер», мс. */
export const POLL_INTERVAL_MS = 2000;

/** Единственная точка вызова бэкенда из интерфейса. */
export function getSystemOverview(): Promise<SystemOverview> {
  return invoke<SystemOverview>('get_system_overview');
}

export function scanStart(target: string): Promise<ScanReport> {
  return invoke<ScanReport>('scan_start', { target });
}

export function clamavStatus(): Promise<EngineStatus> {
  return invoke<EngineStatus>('clamav_status');
}

export function assessFile(path: string): Promise<Suspicion> {
  return invoke<Suspicion>('assess_file', { path });
}

export function defaultScanTarget(): Promise<string> {
  return invoke<string>('default_scan_target');
}

const ERROR_KINDS: readonly ErrorKind[] = [
  'unsupported',
  'probe',
  'engineMissing',
  'invalidTarget',
];

function isCommandError(err: unknown): err is CommandError {
  return (
    typeof err === 'object' &&
    err !== null &&
    'kind' in err &&
    'message' in err &&
    typeof err.kind === 'string' &&
    (ERROR_KINDS as readonly string[]).includes(err.kind) &&
    typeof err.message === 'string'
  );
}

/** Приводит любое отклонение к CommandError, чтобы интерфейс не получал сырые значения. */
export function toCommandError(err: unknown): CommandError {
  if (isCommandError(err)) {
    return err;
  }
  return { kind: 'probe', message: String(err) };
}
