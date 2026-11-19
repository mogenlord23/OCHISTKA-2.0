// DTO бэкенда генерируются из Rust (ts-rs) в ./generated. Вручную не править:
// `OCHISTKA_BLESS=1 cargo test -p ochistka-core --test generated_types`.
export type { CpuStatus } from './generated/CpuStatus';
export type { DiskKind } from './generated/DiskKind';
export type { DiskStatus } from './generated/DiskStatus';
export type { EngineStatus } from './generated/EngineStatus';
export type { GpuInfo } from './generated/GpuInfo';
export type { HardwareOverview } from './generated/HardwareOverview';
export type { MotherboardInfo } from './generated/MotherboardInfo';
export type { Finding } from './generated/Finding';
export type { MemoryStatus } from './generated/MemoryStatus';
export type { ScanReport } from './generated/ScanReport';
export type { Suspicion } from './generated/Suspicion';
export type { BoostState } from './generated/BoostState';
export type { BoostStep } from './generated/BoostStep';
export type { TweakState } from './generated/TweakState';
export type { TweaksOverview } from './generated/TweaksOverview';
export type { TweakAvailability } from './generated/TweakAvailability';
export type { ActivationInfo } from './generated/ActivationInfo';
export type { ActivationStatus } from './generated/ActivationStatus';
export type { SystemOverview } from './generated/SystemOverview';

// Типы только фронтенда.
export type ErrorKind =
  | 'unsupported'
  | 'probe'
  | 'engineMissing'
  | 'invalidTarget'
  | 'cancelled'
  | 'notApplicable'
  | 'needsAdmin';

/** Ошибка команды. `message` уже на русском. */
export interface CommandError {
  kind: ErrorKind;
  message: string;
}
