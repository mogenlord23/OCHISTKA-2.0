import { invoke } from '@tauri-apps/api/core';

/**
 * Остановка идущей проверки. Обёртка вынесена из api.ts, чтобы не пересекаться с другими правками;
 * при следующей чистке её можно перенести в api.ts.
 */
export function scanCancel(): Promise<boolean> {
  return invoke<boolean>('scan_cancel');
}
