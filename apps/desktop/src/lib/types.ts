// Зеркало DTO ядра (crates/ochistka-core, camelCase). Пишется вручную.

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

/** Ошибка команды: "unsupported" — ОС без адаптера, "probe" — сбой чтения. */
export interface CommandError {
  kind: 'unsupported' | 'probe';
  message: string;
}
