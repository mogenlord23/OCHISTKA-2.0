// DTO бэкенда генерируются из Rust (ts-rs) в ./generated. Вручную не править:
// `OCHISTKA_BLESS=1 cargo test -p ochistka-core --test generated_types`.
export type { CpuStatus } from './generated/CpuStatus';
export type { DiskKind } from './generated/DiskKind';
export type { DiskStatus } from './generated/DiskStatus';
export type { EngineStatus } from './generated/EngineStatus';
export type { Finding } from './generated/Finding';
export type { MemoryStatus } from './generated/MemoryStatus';
export type { ScanReport } from './generated/ScanReport';
export type { Suspicion } from './generated/Suspicion';
export type { SystemOverview } from './generated/SystemOverview';

// Типы только фронтенда.
export type ErrorKind = 'unsupported' | 'probe' | 'engineMissing' | 'invalidTarget' | 'cancelled';

/** Ошибка команды. `message` уже на русском. */
export interface CommandError {
  kind: ErrorKind;
  message: string;
}
