/** «Обход сайтов»: только вызовы Rust. Параметров нет: список сайтов и прокси ведёт Rust. */
import { invoke } from '@tauri-apps/api/core';

export interface SiteBypassStatus {
  active: boolean;
  error: boolean;
  /** «Сайты открываются через обход» / «Выключено» / «Ошибка: причина». */
  message: string;
}

export const siteBypassStatus = () => invoke<SiteBypassStatus>('site_bypass_status');
export const siteBypassStart = () => invoke<SiteBypassStatus>('site_bypass_start');
export const siteBypassStop = () => invoke<SiteBypassStatus>('site_bypass_stop');

/** Желаемое положение, пока Rust работает в фоне (кнопка меняется в тот же кадр). */
export type Want = 'on' | 'off' | null;

export type BypassView = {
  /** Положение кнопки. */
  checked: boolean;
  /** Идёт включение или выключение. */
  busy: boolean;
  error: boolean;
  text: string;
};

export function bypassView(
  status: SiteBypassStatus | null,
  want: Want,
  error: string | null,
): BypassView {
  if (want === 'on') return { checked: true, busy: true, error: false, text: 'Включаем…' };
  if (want === 'off') return { checked: false, busy: true, error: false, text: 'Выключаем…' };
  if (error) return { checked: false, busy: false, error: true, text: `Ошибка: ${error}` };
  if (status?.error) return { checked: false, busy: false, error: true, text: status.message };
  if (status?.active) {
    return { checked: true, busy: false, error: false, text: 'Сайты открываются через обход' };
  }
  return { checked: false, busy: false, error: false, text: 'Выключено' };
}

export function errorText(err: unknown): string {
  if (err && typeof err === 'object' && 'message' in err) {
    return String((err as { message: unknown }).message);
  }
  return 'не удалось выполнить действие';
}
