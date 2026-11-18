import type { InstalledProgram, ProgramKind } from './programsApi';
import { sortList, type SortState } from './sortState';

/** Подпись типа для таблицы. Store и прочее — по-русски, как в остальном интерфейсе. */
export function kindLabel(kind: ProgramKind): string {
  switch (kind) {
    case 'win32':
      return 'Win32';
    case 'store':
      return 'Store';
    default:
      return 'Другое';
  }
}

/** Поиск без учёта регистра по названию и издателю. Пустой запрос возвращает весь список. */
export function filterPrograms(
  programs: readonly InstalledProgram[],
  query: string,
): InstalledProgram[] {
  const needle = query.trim().toLocaleLowerCase('ru-RU');
  if (needle === '') {
    return [...programs];
  }
  return programs.filter((program) => {
    const name = program.name.toLocaleLowerCase('ru-RU');
    const publisher = (program.publisher ?? '').toLocaleLowerCase('ru-RU');
    return name.includes(needle) || publisher.includes(needle);
  });
}

/** Размер папки можно посчитать, только если известна папка установки. Иначе — причина для интерфейса. */
export function measureBlockReason(program: InstalledProgram): string | null {
  return program.installLocation ? null : 'В реестре не указана папка установки';
}

export type ProgramSortKey = 'name' | 'size' | 'disk' | 'version' | 'kind' | 'publisher';

/** Столбцы, где первый клик даёт «больше сверху». Текстовые идут по алфавиту. */
export const NUMERIC_PROGRAM_KEYS: readonly ProgramSortKey[] = ['size'];

function programSortValue(program: InstalledProgram, key: ProgramSortKey): number | string | null {
  switch (key) {
    case 'name':
      return program.name;
    case 'size':
      return program.sizeBytes;
    case 'disk':
      return program.diskLetter;
    case 'version':
      return program.version;
    case 'kind':
      return kindLabel(program.kind);
    case 'publisher':
      return program.publisher;
  }
}

/** Порядок программ по столбцу. Неизвестные значения (размер, версия…) всегда в конце. */
export function sortPrograms(
  list: readonly InstalledProgram[],
  state: SortState<ProgramSortKey> | null,
): InstalledProgram[] {
  const { key, dir } = state ?? { key: 'name' as const, dir: 'asc' as const };
  return sortList(list, (program) => programSortValue(program, key), dir);
}

/** Путь к exe из команды автозагрузки (`"C:\\A\\a.exe" -x` или `C:\\A B\\a.exe /s`). `null` — не нашли. */
export function commandExePath(command: string | null): string | null {
  if (!command) return null;
  const text = command.trim();
  if (text.startsWith('"')) {
    const end = text.indexOf('"', 1);
    return end > 1 ? text.slice(1, end) : null;
  }
  const match = /^(.+?\.exe)(?:\s|$)/i.exec(text);
  return match ? match[1] : null;
}

/** Описание пункта «Отключить/Включить автозагрузку» для выбранной записи. */
export function startupActionLabel(enabled: boolean): string {
  return enabled ? 'Отключить автозагрузку' : 'Включить автозагрузку';
}
