import { SECTIONS, type SectionId } from './sections';

const KEY = 'ochistka.subsections';

export type SubMemory = Partial<Record<SectionId, string>>;

/** Подраздел по умолчанию: первый в разделе. */
export function defaultSub(id: SectionId): string {
  return SECTIONS.find((item) => item.id === id)?.subsections[0]?.id ?? '';
}

/** Подраздел существует в этом разделе (чужие и устаревшие значения отбрасываем). */
function valid(id: SectionId, sub: unknown): sub is string {
  return (
    typeof sub === 'string' &&
    (SECTIONS.find((item) => item.id === id)?.subsections.some((s) => s.id === sub) ?? false)
  );
}

/** Читает запомненные подразделы; при любой порче данных возвращает пустое (все по умолчанию). */
export function loadSubsections(
  storage: Pick<Storage, 'getItem'> | null = safeStorage(),
): SubMemory {
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, unknown> | null;
    const memory: SubMemory = {};
    for (const section of SECTIONS) {
      const sub = parsed?.[section.id];
      if (valid(section.id, sub)) memory[section.id] = sub;
    }
    return memory;
  } catch {
    return {};
  }
}

export function saveSubsections(
  memory: SubMemory,
  storage: Pick<Storage, 'setItem'> | null = safeStorage(),
): void {
  try {
    storage?.setItem(KEY, JSON.stringify(memory));
  } catch {
    /* хранилище недоступно: подразделы помнятся до закрытия окна */
  }
}

/** Подраздел, который нужно открыть при входе в раздел. */
export function subFor(id: SectionId, memory: SubMemory): string {
  return memory[id] ?? defaultSub(id);
}

function safeStorage(): Storage | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
}
