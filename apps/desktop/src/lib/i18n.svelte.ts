import ru from './i18n/ru.json';
import en from './i18n/en.json';

export type Lang = 'ru' | 'en';
export type Key = keyof typeof ru;

const DICTS: Record<Lang, Record<Key, string>> = { ru, en };

/** Текущий язык интерфейса. Меняется переключателем RU/ENG. */
export const i18n = $state<{ lang: Lang }>({ lang: 'ru' });

/** Строка на текущем языке; при отсутствии перевода — русская. */
export function t(key: Key): string {
  return DICTS[i18n.lang][key] ?? DICTS.ru[key];
}

export const KEYS = Object.keys(ru) as Key[];
export { DICTS };
