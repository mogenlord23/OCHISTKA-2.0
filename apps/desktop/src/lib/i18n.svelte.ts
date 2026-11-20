import ru from './i18n/ru.json';
import en from './i18n/en.json';

export type Lang = 'ru' | 'en';
export type Key = keyof typeof ru;

const DICTS: Record<Lang, Record<Key, string>> = { ru, en };

const LANG_KEY = 'ochistka.lang';

function readLang(): Lang {
  try {
    return localStorage.getItem(LANG_KEY) === 'en' ? 'en' : 'ru';
  } catch {
    return 'ru';
  }
}

/** Текущий язык интерфейса. Меняется переключателем RU/ENG, выбор запоминается. */
export const i18n = $state<{ lang: Lang }>({ lang: readLang() });

export function setLang(lang: Lang): void {
  i18n.lang = lang;
  try {
    localStorage.setItem(LANG_KEY, lang);
  } catch {
    /* хранилище недоступно: язык действует до закрытия окна */
  }
}

/** Строка на текущем языке; при отсутствии перевода — русская. */
export function t(key: Key): string {
  return DICTS[i18n.lang][key] ?? DICTS.ru[key];
}

export const KEYS = Object.keys(ru) as Key[];
export { DICTS };
