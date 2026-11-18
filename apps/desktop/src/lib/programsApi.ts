import { invoke } from '@tauri-apps/api/core';

/** Ошибка раздела «Программы». `message` уже на русском. */
export interface ProgramsErrorInfo {
  kind: string;
  message: string;
}

/** Тип программы: `win32` — классическая, `store` — Microsoft Store, `other` — прочее. */
export type ProgramKind = 'win32' | 'store' | 'other';

export interface InstalledProgram {
  name: string;
  publisher: string | null;
  version: string | null;
  /** Оценка из реестра в байтах; `null`, если в реестре нет. */
  sizeBytes: number | null;
  installLocation: string | null;
  /** Буква диска вида `C:`; `null`, если папка установки не указана. */
  diskLetter: string | null;
  kind: ProgramKind;
}

export interface StartupEntry {
  name: string;
  command: string | null;
  location: string;
}

export interface FolderMeasure {
  path: string;
  bytes: number;
  files: number;
  /** `false`: подсчёт остановлен лимитом записей, цифры — нижняя оценка. */
  complete: boolean;
}

/** Ответ бэкенда: тип в поле `type` (см. `ProgramsResponse` в programs.rs). */
export type ProgramsResponse =
  | { type: 'installed'; programs: InstalledProgram[] }
  | { type: 'startup'; entries: StartupEntry[] }
  | { type: 'folder'; report: FolderMeasure }
  | { type: 'stopped' };

type Request =
  | { action: 'listInstalled' }
  | { action: 'listStartup' }
  | { action: 'measureFolder'; path: string }
  | { action: 'stopMeasure' };

async function request(payload: Request): Promise<ProgramsResponse> {
  return invoke<ProgramsResponse>('programs_request', { request: payload });
}

/** Берёт ответ нужного типа. Другой тип — ошибка контракта, а не молчаливый пустой список. */
function expect<T extends ProgramsResponse['type']>(
  response: ProgramsResponse,
  type: T,
): Extract<ProgramsResponse, { type: T }> {
  if (response.type !== type) {
    throw {
      kind: 'io',
      message: `неожиданный ответ: ${response.type}`,
    } satisfies ProgramsErrorInfo;
  }
  return response as Extract<ProgramsResponse, { type: T }>;
}

/** Приводит любую ошибку вызова к виду для интерфейса. */
export function toProgramsError(err: unknown): ProgramsErrorInfo {
  if (typeof err === 'object' && err !== null && 'kind' in err && 'message' in err) {
    const value = err as { kind: unknown; message: unknown };
    return { kind: String(value.kind), message: String(value.message) };
  }
  return { kind: 'io', message: String(err) };
}

export async function listInstalled(): Promise<InstalledProgram[]> {
  return expect(await request({ action: 'listInstalled' }), 'installed').programs;
}

export async function listStartup(): Promise<StartupEntry[]> {
  return expect(await request({ action: 'listStartup' }), 'startup').entries;
}

export async function measureFolder(path: string): Promise<FolderMeasure> {
  return expect(await request({ action: 'measureFolder', path }), 'folder').report;
}

export async function stopMeasure(): Promise<void> {
  expect(await request({ action: 'stopMeasure' }), 'stopped');
}
