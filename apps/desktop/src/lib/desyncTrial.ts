/** Пробный запуск полного NetZapret. Только вызовы Rust: параметров движка здесь нет. */
import { invoke } from '@tauri-apps/api/core';

export type TrialService = 'sites' | 'youtube' | 'discord' | 'telegram' | 'games';
export type TrialVerdict = 'works' | 'fails' | 'unchecked';

export interface TrialNote {
  service: TrialService;
  verdict: TrialVerdict;
}

export interface TrialStatus {
  active: boolean;
  remainingSecs: number;
  message: string;
  /** Причина, по которой запуск сейчас невозможен; null — можно запускать. */
  unavailable: string | null;
  /** Только наблюдения пользователя, не подтверждение работы сервиса. */
  notes: TrialNote[];
}

export const TRIAL_SERVICES: { id: TrialService; label: string }[] = [
  { id: 'youtube', label: 'YouTube' },
  { id: 'discord', label: 'Discord' },
  { id: 'telegram', label: 'Telegram' },
  { id: 'sites', label: 'Сайты' },
  { id: 'games', label: 'Игры' },
];

export const TRIAL_VERDICTS: { id: TrialVerdict; label: string }[] = [
  { id: 'works', label: 'Работает' },
  { id: 'fails', label: 'Не работает' },
  { id: 'unchecked', label: 'Не проверено' },
];

export const desyncTrialStatus = () => invoke<TrialStatus>('desync_trial_status');
/** `consent` — явное согласие; Rust проверяет его ещё раз. */
export const desyncTrialStart = (consent: boolean) =>
  invoke<TrialStatus>('desync_trial_start', { consent });
export const desyncTrialStop = () => invoke<TrialStatus>('desync_trial_stop');
export const desyncTrialNote = (service: TrialService, verdict: TrialVerdict) =>
  invoke<TrialStatus>('desync_trial_note', { service, verdict });

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
