/** Чистая логика теста скорости: состояние по событиям, форматы, шкала, итоговый текст. */
import type { SpeedEvent, SpeedFailure, SpeedPhase, SpeedResult } from './speedtestApi';

export const SPEED_NOTE =
  'Тест расходует трафик и обращается к внешнему серверу измерения. Личные данные не отправляются.';

export const OFFLINE_MESSAGE = 'Нет подключения к интернету. Проверьте сеть и повторите';
export const SERVER_MESSAGE = 'Сервер измерения не ответил. Повторите чуть позже';
export const CANCELLED_MESSAGE = 'Тест остановлен';

export const PHASE_LABEL: Record<SpeedPhase, string> = {
  ping: 'Измеряем задержку…',
  download: 'Скачивание…',
  upload: 'Отдача…',
};

export interface SpeedUi {
  status: 'idle' | 'running' | 'done' | 'failed' | 'cancelled';
  phase: SpeedPhase | null;
  /** Мгновенная скорость текущего этапа, Мбит/с. */
  liveMbps: number;
  fraction: number;
  pingMs: number | null;
  downloadMbps: number | null;
  uploadMbps: number | null;
  jitterMs: number | null;
  loadedMs: number | null;
  failure: SpeedFailure | null;
}

export const IDLE: SpeedUi = {
  status: 'idle',
  phase: null,
  liveMbps: 0,
  fraction: 0,
  pingMs: null,
  downloadMbps: null,
  uploadMbps: null,
  jitterMs: null,
  loadedMs: null,
  failure: null,
};

export const START: SpeedUi = { ...IDLE, status: 'running' };

/** Новое состояние по событию Rust. Прежнее не меняется. */
export function reduce(state: SpeedUi, event: SpeedEvent): SpeedUi {
  switch (event.type) {
    case 'phase': {
      const next = { ...state, status: 'running' as const, phase: event.phase, fraction: 0 };
      if (event.phase === 'download') return { ...next, liveMbps: 0 };
      if (event.phase === 'upload') {
        // Скачивание закончено: пока нет итога, показываем его последнее значение.
        return { ...next, downloadMbps: state.liveMbps, liveMbps: 0 };
      }
      return next;
    }
    case 'ping':
      return { ...state, pingMs: event.ms };
    case 'progress':
      return { ...state, phase: event.phase, liveMbps: event.mbps, fraction: event.fraction };
    case 'done':
      return {
        ...state,
        status: 'done',
        phase: null,
        fraction: 1,
        liveMbps: event.result.downloadMbps,
        downloadMbps: event.result.downloadMbps,
        uploadMbps: event.result.uploadMbps,
        pingMs: event.result.pingMs,
        jitterMs: event.result.jitterMs,
        loadedMs: event.result.loadedPingMs,
      };
    case 'failed':
      return { ...state, status: 'failed', phase: null, failure: event.kind };
    case 'cancelled':
      return { ...state, status: 'cancelled', phase: null, liveMbps: 0 };
  }
}

export function failureMessage(kind: SpeedFailure | null): string {
  return kind === 'offline' ? OFFLINE_MESSAGE : SERVER_MESSAGE;
}

/** Верх шкалы индикатора, Мбит/с. Шкала логарифмическая: и 5, и 500 Мбит/с видны. */
export const GAUGE_MAX = 1000;

export function gaugeFraction(mbps: number): number {
  if (!Number.isFinite(mbps) || mbps <= 0) return 0;
  return Math.min(1, Math.log10(1 + mbps) / Math.log10(1 + GAUGE_MAX));
}

/** Шаг плавного приближения числа к цели; `dtMs` — время кадра. Почти у цели прилипает к ней. */
export function easeToward(current: number, target: number, dtMs: number, tauMs = 180): number {
  const k = 1 - Math.exp(-Math.max(0, dtMs) / tauMs);
  const next = current + (target - current) * k;
  return Math.abs(target - next) < 0.05 ? target : next;
}

/** Скорость для показа: 0.0 … 99.9 с десятыми, от 100 целым. */
export function formatMbps(value: number | null): string {
  if (value === null || !Number.isFinite(value)) return '—';
  return value >= 100 ? String(Math.round(value)) : value.toFixed(1);
}

export function formatMs(value: number | null): string {
  if (value === null || !Number.isFinite(value)) return '—';
  return value < 10 ? value.toFixed(1) : String(Math.round(value));
}

export interface Summary {
  title: string;
  text: string;
}

/** Итог словами: что подойдёт при такой скорости и что смущает. Без терминов. */
export function summarize(result: SpeedResult): Summary {
  const { downloadMbps: down, uploadMbps: up, pingMs, jitterMs, loadedPingMs } = result;
  let title: string;
  let text: string;
  if (down >= 100 && pingMs < 40) {
    title = 'Отличное соединение';
    text = 'Подойдёт для видео в 4K, онлайн-игр и больших загрузок.';
  } else if (down >= 25) {
    title = 'Хорошее соединение';
    text = 'Видео в высоком качестве, звонки и обычные загрузки работают без проблем.';
  } else if (down >= 8) {
    title = 'Скромная скорость';
    text = 'Сайты и видео открываются, но большие загрузки будут долгими.';
  } else {
    title = 'Медленное соединение';
    text = 'Видео может подгружаться, а звонки прерываться.';
  }
  const notes: string[] = [];
  if (up < 3) notes.push('Отдача низкая: отправка файлов и видеозвонки могут быть неровными.');
  if (loadedPingMs !== null && loadedPingMs - pingMs > 100) {
    notes.push('Пока канал занят, задержка сильно растёт: игры и звонки могут лагать.');
  }
  if (jitterMs > 30) notes.push('Задержка нестабильна: возможны рывки в играх и звонках.');
  return { title, text: [text, ...notes].join(' ') };
}

const NOTE_KEY = 'ochistka.speedtest.note-seen';

/** Предупреждение показывается один раз: пока тест ни разу не запускали. */
export function noteSeen(): boolean {
  try {
    return localStorage.getItem(NOTE_KEY) === '1';
  } catch {
    return false;
  }
}

export function markNoteSeen(): void {
  try {
    localStorage.setItem(NOTE_KEY, '1');
  } catch {
    // Без хранилища предупреждение просто покажется снова.
  }
}
