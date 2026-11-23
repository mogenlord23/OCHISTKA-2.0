import { Channel, invoke } from '@tauri-apps/api/core';
import type {
  CommandError,
  DiskKind,
  DiskStatus,
  EngineStatus,
  ErrorKind,
  Finding,
  GpuInfo,
  HardwareOverview,
  MotherboardInfo,
  ScanReport,
  SignaturesInfo,
  Suspicion,
  SystemOverview,
} from './types';

/** Интервал опроса раздела «Компьютер», мс. */
export const POLL_INTERVAL_MS = 1000;

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

const DISK_KINDS: readonly DiskKind[] = ['ssd', 'hdd', 'unknown'];

function isDisk(value: unknown): value is DiskStatus {
  return (
    isObj(value) &&
    isStr(value.displayName) &&
    isStr(value.mountPoint) &&
    isNum(value.totalBytes) &&
    isNum(value.usedBytes) &&
    isNum(value.usagePercent) &&
    isStr(value.kind) &&
    (DISK_KINDS as readonly string[]).includes(value.kind)
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
    value.disks.every(isDisk) &&
    isStr(value.osName) &&
    isStr(value.osVersion)
  );
}

function isGpu(value: unknown): value is GpuInfo {
  return (
    isObj(value) &&
    (value.vendor === null || isStr(value.vendor)) &&
    (value.model === null || isStr(value.model)) &&
    (value.usagePercent === null || isNum(value.usagePercent)) &&
    (value.totalMemoryBytes === null || isNum(value.totalMemoryBytes)) &&
    (value.usedMemoryBytes === null || isNum(value.usedMemoryBytes))
  );
}

function isBoard(value: unknown): value is MotherboardInfo {
  return (
    isObj(value) &&
    (value.vendor === null || isStr(value.vendor)) &&
    (value.name === null || isStr(value.name)) &&
    (value.version === null || isStr(value.version))
  );
}

/** Проверка формы ответа `get_hardware` во время выполнения. */
export function isHardwareOverview(value: unknown): value is HardwareOverview {
  return (
    isObj(value) &&
    Array.isArray(value.gpus) &&
    value.gpus.every(isGpu) &&
    isBoard(value.motherboard) &&
    (value.cpuBrand === null || isStr(value.cpuBrand)) &&
    Array.isArray(value.driveModels) &&
    value.driveModels.every(isStr)
  );
}

/** Проверка формы ответа `scan_start` во время выполнения. */
export function isScanReport(value: unknown): value is ScanReport {
  return (
    isObj(value) &&
    isNum(value.scannedFiles) &&
    isNum(value.infectedFiles) &&
    isNum(value.errors) &&
    isNum(value.skippedFiles) &&
    typeof value.incomplete === 'boolean' &&
    Array.isArray(value.findings) &&
    value.findings.every(isFinding)
  );
}

/** Проверка формы ответа `clamav_status` во время выполнения. */
export function isEngineStatus(value: unknown): value is EngineStatus {
  return isObj(value) && typeof value.available === 'boolean' && isStr(value.reason);
}

/** Проверка формы ответа `clamav_signatures`: папка баз, число файлов, сигнатуры. */
export function isSignaturesInfo(value: unknown): value is SignaturesInfo {
  return (
    isObj(value) &&
    isStr(value.dir) &&
    isNum(value.files) &&
    typeof value.enginePresent === 'boolean' &&
    (value.signatures === null || isNum(value.signatures))
  );
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

/** Видеокарты и материнская плата (только чтение). Вне Windows бэкенд отвечает `unsupported`. */
export function getHardware(): Promise<HardwareOverview> {
  return invokeChecked(isHardwareOverview, 'get_hardware');
}

export interface ScanProgress {
  totalFiles: number;
  processedFiles: number;
  scannedFiles: number;
  errors: number;
  skippedFiles: number;
}

export function scanStart(
  target: string,
  onProgress?: (progress: ScanProgress) => void,
): Promise<ScanReport> {
  if (!onProgress) return invokeChecked(isScanReport, 'scan_start', { target });
  const channel = new Channel<ScanProgress>();
  channel.onmessage = onProgress;
  return invokeChecked(isScanReport, 'scan_start_with_progress', { target, onProgress: channel });
}

export function clamavStatus(): Promise<EngineStatus> {
  return invokeChecked(isEngineStatus, 'clamav_status');
}

/** Папка баз и число загруженных сигнатур: нужно экрану проверки. */
export function clamavSignatures(): Promise<SignaturesInfo> {
  return invokeChecked(isSignaturesInfo, 'clamav_signatures');
}

/** Просит движок загрузить базы (по умолчанию — из папки приложения). */
export function clamavLoadSignatures(path?: string): Promise<EngineStatus> {
  return invokeChecked(isEngineStatus, 'clamav_load_signatures', { path: path ?? null });
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
  'cancelled',
  // Без этих двух причина отказа терялась и интерфейс показывал общую фразу.
  'notApplicable',
  'needsAdmin',
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
