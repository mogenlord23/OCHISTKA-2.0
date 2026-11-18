/**
 * Настройки самого приложения (не Windows) хранятся в localStorage окна. Это удобство одного
 * пользователя; обращения обёрнуты в try/catch, потому что хранилище может быть недоступно.
 * Выбор обоев, курсора и цвета здесь не хранится: он системный и читается из Windows.
 */
interface KeyValueStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

function browserStorage(): KeyValueStorage | null {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

export const SETTINGS_STORAGE_KEY = 'ochistka.settings';

export interface AppSettings {
  reduceMotion: boolean;
}

export function loadSettings(storage: KeyValueStorage | null = browserStorage()): AppSettings {
  try {
    const raw = storage?.getItem(SETTINGS_STORAGE_KEY);
    if (!raw) return { reduceMotion: false };
    const parsed: unknown = JSON.parse(raw);
    const value = (parsed as Record<string, unknown> | null)?.reduceMotion;
    return { reduceMotion: value === true };
  } catch {
    return { reduceMotion: false };
  }
}

export function saveSettings(
  settings: AppSettings,
  storage: KeyValueStorage | null = browserStorage(),
): boolean {
  try {
    storage?.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
    return storage !== null;
  } catch {
    return false;
  }
}

/** Ставит атрибут, по которому CSS выключает анимации. */
export function applyReduceMotion(on: boolean): void {
  try {
    if (on) document.documentElement.dataset.reduceMotion = 'true';
    else delete document.documentElement.dataset.reduceMotion;
  } catch {
    /* нет документа: ничего не делаем */
  }
}
