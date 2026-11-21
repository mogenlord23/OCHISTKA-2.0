/** Пробный запуск полного NetZapret. Только вызовы Rust: параметров движка здесь нет. */
import { invoke } from '@tauri-apps/api/core';

export interface TrialStatus {
  active: boolean;
  remainingSecs: number;
  message: string;
  /** Причина, по которой запуск сейчас невозможен; null — можно запускать. */
  unavailable: string | null;
}

export const desyncTrialStatus = () => invoke<TrialStatus>('desync_trial_status');
/** `consent: true` передаётся только при настоящем нажатии на переключатель. Rust проверяет его ещё раз. */
export const desyncTrialStart = (consent: boolean) =>
  invoke<TrialStatus>('desync_trial_start', { consent });
export const desyncTrialStop = () => invoke<TrialStatus>('desync_trial_stop');

/** Оставшееся время как «1:05». */
export function formatRemaining(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

export function trialErrorText(err: unknown): string {
  if (err && typeof err === 'object' && 'message' in err) {
    return String((err as { message: unknown }).message);
  }
  return 'Не удалось выполнить действие';
}

/** Что показывает переключатель «Обход блокировок». «Включено» только при подтверждённом запуске. */
export type BypassView = {
  status: 'off' | 'on' | 'pending' | 'error';
  /** Можно ли нажимать (статус получен и нет причины запрета). */
  available: boolean;
  text: string;
  /** Короткая причина или ошибка рядом со статусом. */
  hint: string | null;
};

export function bypassView(
  trial: TrialStatus | null,
  busy: boolean,
  error: string | null,
): BypassView {
  if (busy) return { status: 'pending', available: false, text: 'Подготовка…', hint: null };
  if (trial?.active) {
    return {
      status: 'on',
      available: true,
      text: `Включено · осталось ${formatRemaining(trial.remainingSecs)}`,
      hint: null,
    };
  }
  if (error) return { status: 'error', available: trial !== null, text: 'Ошибка', hint: error };
  if (!trial) return { status: 'off', available: false, text: 'Выключено', hint: null };
  return {
    status: 'off',
    available: trial.unavailable === null,
    text: 'Выключено',
    hint: trial.unavailable,
  };
}
