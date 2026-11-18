import { invoke } from '@tauri-apps/api/core';

/** Процесс из списка. `cpuPercent` — доля всей машины (0..100). */
export interface ProcessInfo {
  pid: number;
  name: string;
  cpuPercent: number;
  memoryBytes: number;
  startTimeUnix: number;
}

/** Служба только для чтения. `startType` — режим запуска из реестра (подпись). */
export interface ServiceInfo {
  name: string;
  displayName: string;
  startType: string;
}

/** Ошибка раздела «Процессы». `message` уже на русском. */
export interface ProcessesErrorInfo {
  kind: string;
  message: string;
}

/** Ответ бэкенда: тип в поле `type` (см. `ProcessesResponse` в processes.rs). */
type ProcessesResponse =
  | { type: 'list'; processes: ProcessInfo[] }
  | { type: 'exePath'; path: string | null }
  | { type: 'ended'; pid: number }
  | { type: 'services'; services: ServiceInfo[] };

type Request =
  | { action: 'list' }
  | { action: 'exePath'; pid: number }
  | { action: 'endTask'; pid: number; startTimeUnix: number; confirmed: boolean }
  | { action: 'services' };

async function request(payload: Request): Promise<ProcessesResponse> {
  return invoke<ProcessesResponse>('processes_request', { request: payload });
}

/** Берёт ответ нужного типа. Другой тип — ошибка контракта. */
function expect<T extends ProcessesResponse['type']>(
  response: ProcessesResponse,
  type: T,
): Extract<ProcessesResponse, { type: T }> {
  if (response.type !== type) {
    throw {
      kind: 'probe',
      message: `неожиданный ответ: ${response.type}`,
    } satisfies ProcessesErrorInfo;
  }
  return response as Extract<ProcessesResponse, { type: T }>;
}

/** Приводит любую ошибку вызова к виду для интерфейса. */
export function toProcessesError(err: unknown): ProcessesErrorInfo {
  if (typeof err === 'object' && err !== null && 'kind' in err && 'message' in err) {
    const value = err as { kind: unknown; message: unknown };
    return { kind: String(value.kind), message: String(value.message) };
  }
  return { kind: 'probe', message: String(err) };
}

export async function listProcesses(): Promise<ProcessInfo[]> {
  return expect(await request({ action: 'list' }), 'list').processes;
}

export async function exePath(pid: number): Promise<string | null> {
  return expect(await request({ action: 'exePath', pid }), 'exePath').path;
}

/**
 * Завершает процесс. `confirmed` должен быть `true` только после подтверждения в диалоге;
 * без него бэкенд откажет. Время запуска сверяется, чтобы не задеть новый процесс с тем же PID.
 */
export async function endTask(
  target: Pick<ProcessInfo, 'pid' | 'startTimeUnix'>,
  confirmed: boolean,
): Promise<void> {
  expect(
    await request({
      action: 'endTask',
      pid: target.pid,
      startTimeUnix: target.startTimeUnix,
      confirmed,
    }),
    'ended',
  );
}

export async function listServices(): Promise<ServiceInfo[]> {
  return expect(await request({ action: 'services' }), 'services').services;
}
