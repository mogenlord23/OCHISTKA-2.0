import type { InstalledProgram, ProgramKind } from './programsApi';

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
