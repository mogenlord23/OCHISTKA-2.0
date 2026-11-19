import { invoke } from '@tauri-apps/api/core';
import { prefetchIcons } from './iconLoader';

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
  /** Стабильный ключ: по нему бэкенд находит программу при удалении. */
  id: string;
  /** Файл со значком (из DisplayIcon); `null` — рисуем букву. */
  iconPath: string | null;
  canUninstall: boolean;
  /** Почему удалить нельзя; `null`, если можно. */
  uninstallReason: string | null;
}

export interface StartupEntry {
  name: string;
  command: string | null;
  location: string;
  /** Источник записи и имя значения: по ним бэкенд меняет флаг автозагрузки. */
  source: string;
  key: string;
  enabled: boolean;
  /** Состояние до отключения из OCHISTKA; `null`, если не менялось. */
  previousEnabled: boolean | null;
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
  | { type: 'stopped' }
  | { type: 'icon'; dataUrl: string | null }
  | { type: 'iconBatch'; items: Array<{ key: string; dataUrl: string | null }> }
  | { type: 'uninstallStarted' }
  | { type: 'startupChanged'; previousEnabled: boolean; enabled: boolean };

type Request =
  | { action: 'listInstalled' }
  | { action: 'listStartup' }
  | { action: 'measureFolder'; path: string }
  | { action: 'stopMeasure' }
  | { action: 'icon'; path: string }
  | { action: 'programIcons'; ids: string[] }
  | { action: 'pathIcons'; paths: string[] }
  | { action: 'uninstall'; id: string; confirmed: boolean }
  | {
      action: 'setStartupEnabled';
      source: string;
      key: string;
      enabled: boolean;
      confirmed: boolean;
    };

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

/** Значок файла как адрес PNG; `null`, если взять не удалось (или ОС не поддерживает). */
export async function getIcon(path: string): Promise<string | null> {
  return expect(await request({ action: 'icon', path }), 'icon').dataUrl;
}

/** Ключ значка программы в общем кэше. */
export function programIconKey(id: string): string {
  return `program:${id}`;
}

function batchMap(
  items: Array<{ key: string; dataUrl: string | null }>,
): Map<string, string | null> {
  return new Map(items.map((item) => [item.key, item.dataUrl]));
}

/** Значки программ пачкой по `id`: DisplayIcon, логотип Store, основной exe. */
export async function getProgramIcons(ids: string[]): Promise<Map<string, string | null>> {
  return batchMap(expect(await request({ action: 'programIcons', ids }), 'iconBatch').items);
}

/** Значки файлов пачкой по путям (exe процессов и автозагрузки). */
export async function getPathIcons(paths: string[]): Promise<Map<string, string | null>> {
  return batchMap(expect(await request({ action: 'pathIcons', paths }), 'iconBatch').items);
}

/** Значок одной программы (запасной путь, если пачка не удалась). */
export async function getProgramIcon(id: string): Promise<string | null> {
  return (await getProgramIcons([id])).get(id) ?? null;
}

/** Сразу после загрузки списка просит у бэкенда значки всех программ фоновыми пачками. */
export function prefetchProgramIcons(programs: ReadonlyArray<{ id: string }>): void {
  prefetchIcons(
    programs.map((program) => programIconKey(program.id)),
    async (keys) => {
      const ids = keys.map((key) => key.slice('program:'.length));
      const found = await getProgramIcons(ids);
      return new Map(keys.map((key, i) => [key, found.get(ids[i]) ?? null]));
    },
  );
}

/** То же для файлов: ключ значка совпадает с путём. */
export function prefetchPathIcons(paths: readonly string[]): void {
  prefetchIcons(paths, (keys) => getPathIcons(keys));
}

/** Запускает штатное удаление. `confirmed` — только после диалога подтверждения. */
export async function uninstallProgram(id: string, confirmed: boolean): Promise<void> {
  expect(await request({ action: 'uninstall', id, confirmed }), 'uninstallStarted');
}

export async function setStartupEnabled(
  entry: Pick<StartupEntry, 'source' | 'key'>,
  enabled: boolean,
  confirmed: boolean,
): Promise<{ previousEnabled: boolean; enabled: boolean }> {
  const { previousEnabled, enabled: now } = expect(
    await request({
      action: 'setStartupEnabled',
      source: entry.source,
      key: entry.key,
      enabled,
      confirmed,
    }),
    'startupChanged',
  );
  return { previousEnabled, enabled: now };
}
