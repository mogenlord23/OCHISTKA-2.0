import { readSoundEnabled, writeSoundEnabled } from './sound';

/** Один общий переключатель звука: правая панель и «Оформление → Настройки приложения». */
export const soundState = $state<{ on: boolean }>({ on: readSoundEnabled() });

/** Перечитывает сохранённое значение (при старте окна и в тестах). */
export function syncSoundState(): void {
  soundState.on = readSoundEnabled();
}

export function setSoundOn(on: boolean): void {
  soundState.on = on;
  writeSoundEnabled(on);
}
