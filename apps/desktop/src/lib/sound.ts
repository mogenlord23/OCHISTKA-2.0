import heeheeUrl from '../assets/heehee.wav';

const KEY = 'soundEnabled';

/** Звук включён по умолчанию; сохранённое значение читается с защитой от сбоев хранилища. */
export function readSoundEnabled(): boolean {
  try {
    return localStorage.getItem(KEY) !== '0';
  } catch {
    return true;
  }
}

export function writeSoundEnabled(on: boolean): void {
  try {
    localStorage.setItem(KEY, on ? '1' : '0');
  } catch {
    /* без сохранения */
  }
}

/** Проигрывает звук пасхалки, если звук включён. Ошибки воспроизведения глушатся. */
export function playHeehee(): void {
  if (!readSoundEnabled()) return;
  try {
    const audio = new Audio(heeheeUrl);
    void audio.play()?.catch(() => {});
  } catch {
    /* окружение без звука */
  }
}
