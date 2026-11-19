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

let ctx: AudioContext | null = null;
let heeheeBuffer: AudioBuffer | null = null;

/** Один AudioContext на приложение; звук пасхалки декодируется заранее, чтобы играть мгновенно. */
function audio(): AudioContext | null {
  if (ctx) return ctx;
  try {
    const Ctor = globalThis.AudioContext;
    if (!Ctor) return null;
    ctx = new Ctor();
    void fetch(heeheeUrl)
      .then((r) => r.arrayBuffer())
      .then((data) => ctx!.decodeAudioData(data))
      .then((buf) => (heeheeBuffer = buf))
      .catch(() => {});
  } catch {
    ctx = null;
  }
  return ctx;
}

/** Подготовить звук заранее (вызывается при запуске). */
export function preloadSounds(): void {
  audio();
}

/** Проигрывает звук пасхалки, если звук включён. Ошибки воспроизведения глушатся. */
export function playHeehee(): void {
  if (!readSoundEnabled()) return;
  try {
    const c = audio();
    if (c && heeheeBuffer) {
      void c.resume();
      const src = c.createBufferSource();
      src.buffer = heeheeBuffer;
      src.connect(c.destination);
      src.start();
      return;
    }
    void new Audio(heeheeUrl).play()?.catch(() => {});
  } catch {
    /* окружение без звука */
  }
}

/** Короткий мягкий щелчок при переходе между разделами. */
export function playClick(): void {
  if (!readSoundEnabled()) return;
  try {
    const c = audio();
    if (!c) return;
    void c.resume();
    const t = c.currentTime;
    const osc = c.createOscillator();
    const gain = c.createGain();
    osc.type = 'triangle';
    osc.frequency.setValueAtTime(1400, t);
    osc.frequency.exponentialRampToValueAtTime(700, t + 0.04);
    gain.gain.setValueAtTime(0.12, t);
    gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.06);
    osc.connect(gain).connect(c.destination);
    osc.start(t);
    osc.stop(t + 0.07);
  } catch {
    /* окружение без звука */
  }
}
