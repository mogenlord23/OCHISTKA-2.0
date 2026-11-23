/**
 * Возможности раздела «Вирусы» на стороне интерфейса.
 *
 * Перенос находок в карантин пока выключен: бэкенд отказывает в переносе, потому что
 * безопасная проверка файла перед переносом не реализована (apps/desktop/src-tauri/src/disks.rs,
 * ветка QuarantineFindings). Включать флаг можно только вместе с доказанной на Windows
 * безопасностью переноса — тогда кнопка «Убрать выбранные» оживёт сама.
 */
export const FINDINGS_TRANSFER_READY = false;

/** Кнопка «Убрать выбранные»: перенос готов, выбрана хотя бы одна находка, проверка не идёт. */
export function removeAllowed(
  transferReady: boolean,
  selectedCount: number,
  busy = false,
): boolean {
  return transferReady && selectedCount > 0 && !busy;
}
