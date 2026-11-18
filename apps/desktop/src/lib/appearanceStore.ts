import {
  APPEARANCE_KINDS,
  CURSOR_DEFS,
  cursorCss,
  defaultTileId,
  tilesFor,
  type AppearanceKind,
} from './appearance';

/**
 * Выбор оформления хранится в localStorage этого окна. Это удобство одного пользователя:
 * он не системная настройка и на Windows ничего не меняет. Все обращения к хранилищу
 * обёрнуты в try/catch, потому что оно может быть недоступно (приватный режим, заблокированные данные).
 */

export const APPEARANCE_STORAGE_KEY = 'ochistka.appearance';

export type AppearanceChoice = Record<AppearanceKind, string>;

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

export function defaultChoice(): AppearanceChoice {
  return {
    wallpaper: defaultTileId('wallpaper'),
    cursor: defaultTileId('cursor'),
    color: defaultTileId('color'),
  };
}

function isKnown(kind: AppearanceKind, id: unknown): id is string {
  return typeof id === 'string' && tilesFor(kind).some((tile) => tile.id === id);
}

/** Читает выбор; при любой ошибке или неизвестном id возвращает значения по умолчанию. */
export function loadChoice(storage: KeyValueStorage | null = browserStorage()): AppearanceChoice {
  const choice = defaultChoice();
  try {
    const raw = storage?.getItem(APPEARANCE_STORAGE_KEY);
    if (!raw) return choice;
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== 'object' || parsed === null) return choice;
    for (const kind of APPEARANCE_KINDS) {
      const value = (parsed as Record<string, unknown>)[kind];
      if (isKnown(kind, value)) choice[kind] = value;
    }
  } catch {
    return defaultChoice();
  }
  return choice;
}

/** Сохраняет выбор одного вида, не трогая остальные. Возвращает false, если записать не удалось. */
export function saveChoice(
  kind: AppearanceKind,
  id: string,
  storage: KeyValueStorage | null = browserStorage(),
): boolean {
  if (!isKnown(kind, id)) return false;
  try {
    const next = { ...loadChoice(storage), [kind]: id };
    storage?.setItem(APPEARANCE_STORAGE_KEY, JSON.stringify(next));
    return storage !== null;
  } catch {
    return false;
  }
}

/** Настройки самого приложения (не Windows). Тоже только localStorage. */
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

const CURSOR_STYLE_ID = 'ochistka-app-cursor';

/** Id курсора, который пользователь выбрал сам; `null` — выбора не было, остаётся системный. */
export function storedCursorId(storage: KeyValueStorage | null = browserStorage()): string | null {
  try {
    const raw = storage?.getItem(APPEARANCE_STORAGE_KEY);
    if (!raw) return null;
    const value = (JSON.parse(raw) as Record<string, unknown> | null)?.cursor;
    return isKnown('cursor', value) ? value : null;
  } catch {
    return null;
  }
}

/**
 * Делает схему курсора указателем самого приложения (Windows не меняется).
 * `null` возвращает системный указатель.
 */
export function applyAppCursor(id: string | null): void {
  try {
    document.getElementById(CURSOR_STYLE_ID)?.remove();
    const def = CURSOR_DEFS.find((item) => item.id === id);
    if (!def) return;
    const style = document.createElement('style');
    style.id = CURSOR_STYLE_ID;
    style.textContent = `html, html *:not(input):not(textarea) { cursor: ${cursorCss(def.cursor, 'app')} !important; }`;
    document.head.appendChild(style);
  } catch {
    /* нет документа: ничего не делаем */
  }
}
