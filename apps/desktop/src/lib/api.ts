import { invoke } from '@tauri-apps/api/core';
import type {
  CommandError,
  DiskStatus,
  EngineStatus,
  ErrorKind,
  Finding,
  ScanReport,
  Suspicion,
  SystemOverview,
} from './types';

/** Интервал опроса раздела «Компьютер», мс. */
export const POLL_INTERVAL_MS = 2000;

type Obj = Record<string, unknown>;

function isObj(value: unknown): value is Obj {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isNum(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

function isStr(value: unknown): value is string {
  return typeof value === 'string';
}

function isDisk(value: unknown): value is DiskStatus {
  return (
    isObj(value) &&
    isStr(value.displayName) &&
    isStr(value.mountPoint) &&
    isNum(value.totalBytes) &&
    isNum(value.usedBytes) &&
    isNum(value.usagePercent)
  );
}

function isFinding(value: unknown): value is Finding {
  return isObj(value) && isStr(value.path) && isStr(value.signature);
}

/** Проверка формы ответа `get_system_overview` во время выполнения. */
export function isSystemOverview(value: unknown): value is SystemOverview {
  return (
    isObj(value) &&
    isObj(value.cpu) &&
    isNum(value.cpu.usagePercent) &&
    isNum(value.cpu.logicalCores) &&
    isObj(value.memory) &&
    isNum(value.memory.totalBytes) &&
    isNum(value.memory.usedBytes) &&
    isNum(value.memory.usagePercent) &&
    Array.isArray(value.disks) &&
    value.disks.every(isDisk)
  );
}

/** Проверка формы ответа `scan_start` во время выполнения. */
export function isScanReport(value: unknown): value is ScanReport {
  return (
    isObj(value) &&
    isNum(value.scannedFiles) &&
    isNum(value.infectedFiles) &&
    isNum(value.errors) &&
    Array.isArray(value.findings) &&
    value.findings.every(isFinding)
  );
}

/** Проверка формы ответа `clamav_status` во время выполнения. */
export function isEngineStatus(value: unknown): value is EngineStatus {
  return isObj(value) && typeof value.available === 'boolean' && isStr(value.reason);
}

/** `invoke<T>` только приводит тип; здесь форма ответа проверяется на самом деле. */
async function invokeChecked<T>(
  guard: (value: unknown) => value is T,
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const result: unknown = await (args === undefined ? invoke(command) : invoke(command, args));
  if (!guard(result)) {
    const error: CommandError = {
      kind: 'probe',
      message: `Ответ команды ${command} имеет неожиданную форму`,
    };
    throw error;
  }
  return result;
}

/** Единственная точка вызова бэкенда из интерфейса. */
export function getSystemOverview(): Promise<SystemOverview> {
  return invokeChecked(isSystemOverview, 'get_system_overview');
}

export function scanStart(target: string): Promise<ScanReport> {
  return invokeChecked(isScanReport, 'scan_start', { target });
}

export function clamavStatus(): Promise<EngineStatus> {
  return invokeChecked(isEngineStatus, 'clamav_status');
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
