/**
 * Загрузка значков для строк таблиц: общий кэш, одна загрузка на ключ и не больше
 * нескольких запросов одновременно, чтобы длинный список не заваливал бэкенд.
 * `null` — значка нет (рисуется цветная буква).
 */
const MAX_PARALLEL = 4;
const MAX_CACHED = 600;

const cache = new Map<string, string | null>();
const inflight = new Map<string, Promise<string | null>>();
const waiting: Array<() => void> = [];
let running = 0;

function acquire(): Promise<void> {
  if (running < MAX_PARALLEL) {
    running++;
    return Promise.resolve();
  }
  return new Promise((resolve) => waiting.push(() => resolve()));
}

function release() {
  const next = waiting.shift();
  if (next) next();
  else running--;
}

/** Значок из кэша, если он уже загружен. `undefined` — ещё не просили. */
export function cachedIcon(key: string): string | null | undefined {
  return cache.get(key.toLowerCase());
}

/** Загружает значок через `resolve` один раз на ключ. Ошибка загрузки — это `null`, не исключение. */
export async function loadIcon(
  key: string,
  resolve: () => Promise<string | null>,
): Promise<string | null> {
  const id = key.toLowerCase();
  if (cache.has(id)) return cache.get(id) ?? null;
  const existing = inflight.get(id);
  if (existing) return existing;
  const job = (async () => {
    await acquire();
    try {
      return await resolve();
    } catch {
      return null;
    } finally {
      release();
    }
  })().then((value) => {
    if (cache.size >= MAX_CACHED) {
      const oldest = cache.keys().next().value;
      if (oldest !== undefined) cache.delete(oldest);
    }
    cache.set(id, value);
    inflight.delete(id);
    return value;
  });
  inflight.set(id, job);
  return job;
}

/** Только для тестов. */
export function resetIconCache() {
  cache.clear();
  inflight.clear();
}
