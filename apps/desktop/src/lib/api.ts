import { invoke } from '@tauri-apps/api/core';
import type { CommandError, SystemOverview } from './types';

/** Интервал опроса раздела «Главная», мс. */
export const POLL_INTERVAL_MS = 2000;

/** Единственная точка вызова системных данных из интерфейса. */
export function getSystemOverview(): Promise<SystemOverview> {
  return invoke<SystemOverview>('get_system_overview');
}

function isCommandError(err: unknown): err is CommandError {
  return (
    typeof err === 'object' &&
    err !== null &&
    'kind' in err &&
    'message' in err &&
    (err.kind === 'unsupported' || err.kind === 'probe') &&
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
