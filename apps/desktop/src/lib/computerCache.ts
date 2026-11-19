/**
 * Последний снимок раздела «Компьютер»: в памяти (переходы между разделами мгновенные)
 * и в localStorage (первый показ после запуска не пустой). Любой сбой хранилища глушится.
 */
import type { HardwareOverview, SystemOverview } from './types';

export interface ComputerSnapshot {
  overview: SystemOverview | null;
  hardware: HardwareOverview | null;
  cpuHistory: number[];
  gpuHistory: number[];
  ramHistory: number[];
  /** Время замера, мс от эпохи. */
  updatedAt: number | null;
}

const KEY = 'computerSnapshot.v1';
/** В хранилище пишем не чаще, чем раз в столько мс: опрос идёт каждую секунду. */
const PERSIST_EVERY_MS = 5000;

let memory: ComputerSnapshot | null = null;
let lastPersist = 0;

function isSnapshot(value: unknown): value is ComputerSnapshot {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    Array.isArray(v.cpuHistory) &&
    Array.isArray(v.gpuHistory) &&
    Array.isArray(v.ramHistory) &&
    (v.overview === null || typeof v.overview === 'object') &&
    (v.hardware === null || typeof v.hardware === 'object')
  );
}

/** Последний снимок: из памяти, иначе из хранилища, иначе null. */
export function loadSnapshot(): ComputerSnapshot | null {
  if (memory) return memory;
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      const parsed: unknown = JSON.parse(raw);
      if (isSnapshot(parsed)) {
        memory = parsed;
        return memory;
      }
    }
  } catch {
    /* хранилище недоступно или повреждено: начинаем без кэша */
  }
  return null;
}

/** Запоминает снимок; в хранилище пишет не чаще `PERSIST_EVERY_MS` (или сразу при `force`). */
export function saveSnapshot(snapshot: ComputerSnapshot, now: number = Date.now(), force = false) {
  memory = snapshot;
  if (!force && now - lastPersist < PERSIST_EVERY_MS) return;
  lastPersist = now;
  try {
    localStorage.setItem(KEY, JSON.stringify(snapshot));
  } catch {
    /* без сохранения */
  }
}

/** Сбрасывает кэш (для тестов). */
export function clearSnapshot(): void {
  memory = null;
  lastPersist = 0;
  try {
    localStorage.removeItem(KEY);
  } catch {
    /* ничего */
  }
}
