/** Состояния кнопки установки драйвера и их подписи. */

import type { DriverAction } from './driversCatalog';

export type InstallStage = 'downloading' | 'verifying' | 'launching' | 'done' | 'error';

/** Событие хода установки от бэкенда. */
export interface InstallProgress {
  id: string;
  stage: InstallStage;
  percent: number | null;
  message: string | null;
}

export type ButtonState =
  | { kind: 'idle' }
  | { kind: 'downloading'; percent: number | null }
  | { kind: 'verifying' }
  | { kind: 'launching' }
  | { kind: 'done' }
  | { kind: 'error'; reason: string };

export const IDLE: ButtonState = { kind: 'idle' };

export function stateFromProgress(event: InstallProgress): ButtonState {
  switch (event.stage) {
    case 'downloading':
      return { kind: 'downloading', percent: event.percent };
    case 'verifying':
      return { kind: 'verifying' };
    case 'launching':
      return { kind: 'launching' };
    case 'done':
      return { kind: 'done' };
    case 'error':
      return { kind: 'error', reason: event.message ?? 'Не удалось установить драйвер.' };
  }
}

/** Пока идёт загрузка, проверка или запуск, кнопка нажимается повторно только после конца. */
export function isBusy(state: ButtonState): boolean {
  return state.kind === 'downloading' || state.kind === 'verifying' || state.kind === 'launching';
}

export function buttonLabel(action: DriverAction, state: ButtonState): string {
  switch (state.kind) {
    case 'idle':
      return action === 'install' ? 'Установить' : 'Открыть сайт';
    case 'downloading':
      return state.percent === null ? 'Загрузка…' : `Загрузка ${state.percent}%`;
    case 'verifying':
      return 'Проверка подписи…';
    case 'launching':
      return 'Запуск установщика…';
    case 'done':
      return action === 'install' ? 'Установщик запущен' : 'Страница открыта';
    case 'error':
      return 'Повторить';
  }
}

/** Доля заполнения кнопки при загрузке, 0..100. */
export function fillPercent(state: ButtonState): number {
  return state.kind === 'downloading' && state.percent !== null
    ? Math.max(0, Math.min(100, state.percent))
    : 0;
}
