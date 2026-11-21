/** Пробный запуск полного NetZapret. Только вызовы Rust: параметров движка здесь нет. */
import { invoke } from '@tauri-apps/api/core';

export interface TrialStatus {
  active: boolean;
  remainingSecs: number;
  message: string;
  /** Причина, по которой запуск сейчас невозможен; null — можно запускать. */
  unavailable: string | null;
  /** Короткая причина, если обход не выключился до конца (драйвер не остановлен). */
  error?: string | null;
  /** Идёт замер скорости: «Проверяем…». */
  phase?: 'idle' | 'measuringBaseline' | 'measuringAfter';
  /** Итог замера скорости: откат или «быстрее или так же; это не доказывает работу у провайдера». */
  speedNote?: string | null;
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
  if (busy) {
    const checking = trial?.phase === 'measuringBaseline';
    return {
      status: 'pending',
      available: false,
      text: checking ? 'Проверяем…' : 'Подготовка…',
      hint: null,
    };
  }
  // Пока драйвер не остановлен, «Выключено» не показываем: ошибка, повторное нажатие повторяет остановку.
  if (trial?.error) return { status: 'error', available: true, text: 'Ошибка', hint: trial.error };
  if (trial?.active) {
    const checking = trial.phase === 'measuringAfter';
    return {
      status: 'on',
      available: true,
      text: `${checking ? 'Проверяем… · ' : ''}Включено · осталось ${formatRemaining(trial.remainingSecs)}`,
      hint: trial.speedNote ?? null,
    };
  }
  if (error) return { status: 'error', available: trial !== null, text: 'Ошибка', hint: error };
  if (!trial) return { status: 'off', available: false, text: 'Выключено', hint: null };
  return {
    status: 'off',
    available: trial.unavailable === null,
    text: 'Выключено',
    hint: trial.speedNote ?? trial.unavailable,
  };
}
