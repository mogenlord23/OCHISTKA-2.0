/**
 * Последние списки разделов (процессы, программы, автозагрузка) в памяти приложения.
 * Раздел при открытии сразу показывает прошлый список, а свежий читает в фоне;
 * так страница не пустеет и не перезагружается целиком при каждом открытии.
 */
const store = new Map<string, unknown>();

export function cachedList<T>(key: string): T[] | null {
  const value = store.get(key);
  return Array.isArray(value) ? (value as T[]) : null;
}

export function rememberList<T>(key: string, items: readonly T[]): void {
  store.set(key, [...items]);
}

/** Для тестов. */
export function clearListCache(): void {
  store.clear();
}
