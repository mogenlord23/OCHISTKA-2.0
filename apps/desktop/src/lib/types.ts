// Зеркало DTO бэкенда (camelCase). Пишется вручную.

export interface CpuStatus {
  usagePercent: number;
  logicalCores: number;
}

export interface MemoryStatus {
  totalBytes: number;
  usedBytes: number;
  usagePercent: number;
}

export interface DiskStatus {
  displayName: string;
  mountPoint: string;
  totalBytes: number;
  usedBytes: number;
  usagePercent: number;
}

export interface SystemOverview {
  cpu: CpuStatus;
  memory: MemoryStatus;
  disks: DiskStatus[];
}

export type ErrorKind = 'unsupported' | 'probe' | 'engineMissing' | 'invalidTarget';

/** Ошибка команды. `message` уже на русском. */
export interface CommandError {
  kind: ErrorKind;
  message: string;
}

export interface Finding {
  path: string;
  signature: string;
}

export interface ScanReport {
  scannedFiles: number;
  infectedFiles: number;
  errors: number;
  findings: Finding[];
}

export interface EngineStatus {
  available: boolean;
  reason: string;
}

/** Эвристическая подсказка по файлу. Не вердикт. */
export interface Suspicion {
  score: number;
  reasons: string[];
}
