import { invoke } from '@tauri-apps/api/core';

/** Ошибка команды «Оформление». `message` уже на русском. */
export interface AppearanceErrorInfo {
  kind: string;
  message: string;
}

/** Состояние оформления Windows. `null` — в реестре значения нет. */
export interface AppearanceStatus {
  accent: string | null;
  /** Эффект прозрачности включён из OCHISTKA (по умолчанию выключено). */
  transparencyOn: boolean;
  /** Заливка панели задач и Пуска включена из OCHISTKA. */
  fillOn: boolean;
  hasBackup: boolean;
  hasPreviousWallpaper: boolean;
  /** Плитка обоев, поставленная из OCHISTKA; `custom` — свой файл. */
  wallpaperTile: string | null;
  /** Акцентный цвет, поставленный плиткой (`#RRGGBB`). */
  accentTile: string | null;
  /** Схема курсора, поставленная из OCHISTKA. */
  cursorScheme: string | null;
}

type Request =
  | { action: 'status' }
  | { action: 'setAccent'; color: string; confirmed: boolean }
  | { action: 'setTransparency'; enabled: boolean; confirmed: boolean }
  | { action: 'setFillTaskbar'; enabled: boolean; confirmed: boolean }
  | { action: 'restoreAppearance'; confirmed: boolean }
  | { action: 'pickAndApplyWallpaper'; confirmed: boolean }
  | { action: 'restoreWallpaper'; confirmed: boolean }
  | { action: 'applyWallpaperTile'; id: string; confirmed: boolean }
  | { action: 'restoreAccent'; confirmed: boolean }
  | {
      action: 'applyCursorScheme';
      id: string;
      color: string;
      secondary: string;
      pattern: string;
      confirmed: boolean;
    }
  | { action: 'restoreCursorScheme'; confirmed: boolean }
  | {
      action: 'prepareCursorSchemes';
      schemes: { id: string; color: string; secondary: string; pattern: string }[];
    };

/** Показываемый текст ошибки: ответ бэкенда или запасная фраза. */
export function appearanceErrorMessage(err: unknown): string {
  if (typeof err === 'object' && err !== null && 'message' in err) {
    const message = (err as { message: unknown }).message;
    if (typeof message === 'string' && message) return message;
  }
  return 'Не удалось связаться с Windows. Попробуйте ещё раз.';
}

export function isCancelled(err: unknown): boolean {
  return (
    typeof err === 'object' && err !== null && (err as { kind?: unknown }).kind === 'cancelled'
  );
}

async function call(request: Request): Promise<AppearanceStatus> {
  const response = await invoke<{ type: 'status'; status: AppearanceStatus }>(
    'appearance_request',
    { request },
  );
  return response.status;
}

export const appearanceStatus = () => call({ action: 'status' });
export const setAccent = (color: string) => call({ action: 'setAccent', color, confirmed: true });
export const setTransparency = (enabled: boolean) =>
  call({ action: 'setTransparency', enabled, confirmed: true });
export const setFillTaskbar = (enabled: boolean) =>
  call({ action: 'setFillTaskbar', enabled, confirmed: true });
export const restoreAppearance = () => call({ action: 'restoreAppearance', confirmed: true });
export const pickAndApplyWallpaper = () =>
  call({ action: 'pickAndApplyWallpaper', confirmed: true });
export const restoreWallpaper = () => call({ action: 'restoreWallpaper', confirmed: true });
export const applyWallpaperTile = (id: string) =>
  call({ action: 'applyWallpaperTile', id, confirmed: true });
export const restoreAccent = () => call({ action: 'restoreAccent', confirmed: true });
export const applyCursorScheme = (id: string, color: string, secondary: string, pattern: string) =>
  call({ action: 'applyCursorScheme', id, color, secondary, pattern, confirmed: true });
export const restoreCursorScheme = () => call({ action: 'restoreCursorScheme', confirmed: true });

let cursorsPrepared = false;
const hoverPrepared = new Set<string>();

/**
 * Наведение или фокус на плитке курсора: рисуем эту схему вне очереди (один раз за запуск), чтобы к
 * клику файлы уже лежали готовые. Повторные наведения ничего не отправляют.
 */
export function prepareCursorScheme(scheme: {
  id: string;
  color: string;
  secondary: string;
  pattern: string;
}): void {
  if (cursorsPrepared || hoverPrepared.has(scheme.id)) return;
  hoverPrepared.add(scheme.id);
  void call({ action: 'prepareCursorSchemes', schemes: [scheme] }).catch(() => {
    hoverPrepared.delete(scheme.id);
  });
}

/**
 * Просит бэкенд заранее нарисовать файлы всех схем курсоров в фоне (один раз за запуск).
 * Потом выбор схемы только пишет в реестр, без рисования. Ошибки не показываются.
 */
export function prepareCursorSchemes(
  schemes: { id: string; color: string; secondary: string; pattern: string }[],
): void {
  if (cursorsPrepared) return;
  cursorsPrepared = true;
  void call({ action: 'prepareCursorSchemes', schemes }).catch(() => {
    cursorsPrepared = false;
  });
}
