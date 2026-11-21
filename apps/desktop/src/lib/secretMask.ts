/**
 * Визуальное представление ссылки подписки для поля, когда оно не редактируется.
 * Видны только схема и адрес сервера (не секрет), а вместо пути с токеном, параметров, логина и пароля
 * рисуются случайные символы-приманки: они не выводятся из настоящих символов, зависит от секрета только
 * их число (с потолком). Это оформление, а не защита от снимка экрана. Настоящая ссылка не меняется.
 */
const URL_PARTS = /^([a-z][a-z0-9+.-]*:\/\/)([^/?#]*)(.*)$/is;
const DECOY_ALPHABET = 'abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789';
const MAX_SECRET = 28;
const MIN_SECRET = 8;

/** Безопасное начало (схема и сервер) и число символов, которые нужно спрятать. */
export function splitSecret(raw: string): { open: string; secretLength: number } {
  const text = raw.trim();
  if (text === '') return { open: '', secretLength: 0 };
  const match = URL_PARTS.exec(text);
  if (!match) return { open: '', secretLength: Math.min(Math.max(text.length, MIN_SECRET), 40) };
  const [, scheme, authority, rest] = match;
  const at = authority.lastIndexOf('@');
  const host = at >= 0 ? authority.slice(at + 1) : authority;
  const hidden = (at >= 0 ? at + 1 : 0) + rest.length;
  return {
    open: `${scheme}${host}${hidden > 0 ? '/' : ''}`,
    secretLength: hidden > 0 ? Math.min(Math.max(hidden, MIN_SECRET), MAX_SECRET) : 0,
  };
}

/** Случайные символы-приманки; `random` можно подменить в тестах. */
export function decoy(length: number, random: () => number = Math.random): string {
  let out = '';
  for (let i = 0; i < length; i++)
    out += DECOY_ALPHABET[Math.floor(random() * DECOY_ALPHABET.length)];
  return out;
}

/** Строка для показа: безопасное начало плюс приманка. Для пустой ссылки пусто. */
export function visualLine(raw: string, random: () => number = Math.random): string {
  const { open, secretLength } = splitSecret(raw);
  return open + decoy(secretLength, random);
}

/** Делит строку на `count` почти равных участков (для поочерёдной анимации). */
export function segments(text: string, count: number): string[] {
  const chars = Array.from(text);
  if (chars.length < count * 3) return chars.length ? [text] : [];
  const size = Math.ceil(chars.length / count);
  const out: string[] = [];
  for (let i = 0; i < chars.length; i += size) out.push(chars.slice(i, i + size).join(''));
  return out;
}
