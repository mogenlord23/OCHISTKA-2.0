/** Пробный запуск полного NetZapret. Только вызовы Rust: параметров движка здесь нет. */
import { invoke } from '@tauri-apps/api/core';

export interface TrialStatus {
  active: boolean;
  message: string;
  /** Причина, по которой запуск сейчас невозможен; null — можно запускать. */
  unavailable: string | null;
  /** Короткая причина, если обход не выключился до конца (драйвер не остановлен). */
  error?: string | null;
}

export const desyncTrialStatus = () => invoke<TrialStatus>('desync_trial_status');
/** `consent: true` передаётся только при настоящем нажатии на переключатель. Rust проверяет его ещё раз. */
export const desyncTrialStart = (consent: boolean) =>
  invoke<TrialStatus>('desync_trial_start', { consent });
export const desyncTrialStop = () => invoke<TrialStatus>('desync_trial_stop');

export function trialErrorText(err: unknown): string {
  if (err && typeof err === 'object' && 'message' in err) {
    return String((err as { message: unknown }).message);
  }
  return 'Не удалось выполнить действие';
}

/** Что показывает переключатель «Обход блокировок». «Включено» подтверждено только статусом из Rust. */
export type BypassView = {
  status: 'off' | 'on' | 'pending' | 'error';
  /** Положение переключателя: меняется в тот же кадр по нажатию (оптимистично). */
  checked: boolean;
  /** Идёт включение или выключение в фоне (анимация состояния). */
  busy: boolean;
  /** Можно ли нажимать (статус получен и нет причины запрета). */
  available: boolean;
  text: string;
  /** Короткая причина или ошибка рядом со статусом. */
  hint: string | null;
};

/** Желаемое положение, пока команда Rust ещё выполняется: интерфейс не ждёт ответа. */
export type Want = 'on' | 'off' | null;

export function bypassView(
  trial: TrialStatus | null,
  want: Want,
  error: string | null,
): BypassView {
  if (want === 'on') {
    return {
      status: 'on',
      checked: true,
      busy: true,
      available: true,
      text: 'Включаем…',
      hint: null,
    };
  }
  if (want === 'off') {
    return {
      status: 'off',
      checked: false,
      busy: true,
      available: true,
      text: 'Выключаем…',
      hint: null,
    };
  }
  // Пока драйвер не остановлен, «Выключено» не показываем: ошибка, повторное нажатие повторяет остановку.
  if (trial?.error) {
    return {
      status: 'error',
      checked: false,
      busy: false,
      available: true,
      text: 'Ошибка',
      hint: trial.error,
    };
  }
  if (trial?.active) {
    return {
      status: 'on',
      checked: true,
      busy: false,
      available: true,
      text: 'Включено',
      hint: null,
    };
  }
  if (error) {
    return {
      status: 'error',
      checked: false,
      busy: false,
      available: trial !== null,
      text: 'Ошибка',
      hint: error,
    };
  }
  if (!trial) {
    return {
      status: 'off',
      checked: false,
      busy: false,
      available: false,
      text: 'Выключено',
      hint: null,
    };
  }
  return {
    status: 'off',
    checked: false,
    busy: false,
    available: trial.unavailable === null,
    text: 'Выключено',
    hint: trial.unavailable,
  };
}
