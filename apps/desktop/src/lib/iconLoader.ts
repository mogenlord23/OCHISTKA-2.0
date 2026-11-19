/**
 * Загрузка значков для строк таблиц: общий кэш, одна загрузка на ключ и не больше
 * нескольких запросов одновременно, чтобы длинный список не заваливал бэкенд.
 * `null` — значка нет (рисуется цветная буква).
 */
const MAX_PARALLEL = 4;
const MAX_CACHED = 3000;
const PREFETCH_CHUNK = 8;
const PREFETCH_PARALLEL = 3;

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
  })().then((value) => remember(id, value));
  inflight.set(id, job);
  return job;
}

function remember(id: string, value: string | null, keep = true): string | null {
  if (keep) {
    if (cache.size >= MAX_CACHED) {
      const oldest = cache.keys().next().value;
      if (oldest !== undefined) cache.delete(oldest);
    }
    cache.set(id, value);
  }
  inflight.delete(id);
  return value;
}

let prefetching = 0;
const prefetchWaiting: Array<() => void> = [];

async function prefetchSlot<T>(job: () => Promise<T>): Promise<T> {
  if (prefetching >= PREFETCH_PARALLEL) await new Promise<void>((go) => prefetchWaiting.push(go));
  else prefetching++;
  try {
    return await job();
  } finally {
    const next = prefetchWaiting.shift();
    if (next) next();
    else prefetching--;
  }
}

/**
 * Фоновая загрузка значков пачками сразу после загрузки списка. Каждому ключу заранее
 * регистрируется «в пути» запись, поэтому строка, попавшая на экран позже, просто ждёт эту же
 * пачку и не делает свой запрос. Уже известные и уже загружаемые ключи пропускаются.
 * Целая пачка упала: ключи не запоминаются как «без значка», строка попробует сама.
 */
export function prefetchIcons(
  keys: readonly string[],
  fetchBatch: (keys: string[]) => Promise<Map<string, string | null>>,
  chunkSize = PREFETCH_CHUNK,
): void {
  const fresh: string[] = [];
  const seen = new Set<string>();
  for (const key of keys) {
    const id = key.toLowerCase();
    if (seen.has(id) || cache.has(id) || inflight.has(id)) continue;
    seen.add(id);
    fresh.push(key);
  }
  for (let from = 0; from < fresh.length; from += chunkSize) {
    const chunk = fresh.slice(from, from + chunkSize);
    const batch = prefetchSlot(async () => {
      try {
        return await fetchBatch(chunk);
      } catch {
        return null;
      }
    });
    for (const key of chunk) {
      const id = key.toLowerCase();
      inflight.set(
        id,
        batch.then((found) =>
          found === null ? remember(id, null, false) : remember(id, found.get(key) ?? null),
        ),
      );
    }
  }
}

/** Только для тестов. */
export function resetIconCache() {
  cache.clear();
  inflight.clear();
}
