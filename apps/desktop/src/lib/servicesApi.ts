import { invoke } from '@tauri-apps/api/core';

export type ServiceState =
  | 'running'
  | 'stopped'
  | 'startPending'
  | 'stopPending'
  | 'continuePending'
  | 'pausePending'
  | 'paused'
  | 'unknown';

export type StartType =
  'boot' | 'system' | 'auto' | 'autoDelayed' | 'manual' | 'disabled' | 'unknown';

/** Насколько осторожно: свободно, с подтверждением последствий, заблокировано. */
export type ServiceGuard = 'free' | 'careful' | 'locked';

export interface ServiceInfo {
  name: string;
  displayName: string;
  state: ServiceState;
  startType: StartType;
  previousStartType: StartType | null;
  guard: ServiceGuard;
  /** Причина блокировки или последствия остановки. */
  note: string | null;
}

export interface ServicesSnapshot {
  /** Есть ли права администратора для действий. */
  elevated: boolean;
  services: ServiceInfo[];
}

export type RequestedStart = 'auto' | 'manual' | 'disabled';

export interface ServicesErrorInfo {
  kind: string;
  message: string;
}

type Response =
  { type: 'list'; snapshot: ServicesSnapshot } | { type: 'changed'; service: ServiceInfo };

type Request =
  | { action: 'list' }
  | { action: 'start'; name: string }
  | { action: 'stop'; name: string; confirmed: boolean }
  | { action: 'setStart'; name: string; start: RequestedStart; confirmed: boolean }
  | { action: 'restoreStart'; name: string };

async function request(payload: Request): Promise<Response> {
  return invoke<Response>('services_request', { request: payload });
}

export function toServicesError(err: unknown): ServicesErrorInfo {
  if (typeof err === 'object' && err !== null && 'kind' in err && 'message' in err) {
    const value = err as { kind: unknown; message: unknown };
    return { kind: String(value.kind), message: String(value.message) };
  }
  return { kind: 'failed', message: 'Не получилось выполнить действие со службой.' };
}

export async function listServices(): Promise<ServicesSnapshot> {
  const response = await request({ action: 'list' });
  if (response.type !== 'list') throw { kind: 'probe', message: 'Неожиданный ответ системы.' };
  return response.snapshot;
}

async function changed(payload: Request): Promise<ServiceInfo> {
  const response = await request(payload);
  if (response.type !== 'changed') throw { kind: 'probe', message: 'Неожиданный ответ системы.' };
  return response.service;
}

/** Запускает службу. Возвращает фактическое состояние после действия. */
export const startService = (name: string) => changed({ action: 'start', name });

/** Останавливает службу. Для служб с последствиями нужен `confirmed: true`. */
export const stopService = (name: string, confirmed: boolean) =>
  changed({ action: 'stop', name, confirmed });

export const setServiceStart = (name: string, start: RequestedStart, confirmed: boolean) =>
  changed({ action: 'setStart', name, start, confirmed });

/** Возвращает тип запуска, который был до первого изменения из OCHISTKA. */
export const restoreServiceStart = (name: string) => changed({ action: 'restoreStart', name });
