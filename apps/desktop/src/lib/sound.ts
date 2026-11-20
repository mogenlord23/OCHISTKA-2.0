import heeheeUrl from '../assets/heehee.wav';
import clickUrl from '../assets/sounds/click.wav';
import switchUrl from '../assets/sounds/switch.wav';

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

type SoundName = 'click' | 'switch' | 'heehee';

const SOURCES: Record<SoundName, { url: string; gain: number }> = {
  click: { url: clickUrl, gain: 1 },
  switch: { url: switchUrl, gain: 1 },
  heehee: { url: heeheeUrl, gain: 1 },
};

/*
 * Звуки играются через Web Audio: файлы читаются и декодируются заранее (preloadSounds),
 * потом каждое воспроизведение это один start() готового буфера, без задержки на загрузку.
 */
let context: AudioContext | null = null;
let preloading: Promise<void> | null = null;
const buffers = new Map<SoundName, AudioBuffer>();

function audioContext(): AudioContext | null {
  if (context) return context;
  try {
    const Ctor =
      globalThis.AudioContext ??
      (globalThis as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!Ctor) return null;
    context = new Ctor({ latencyHint: 'interactive' });
    return context;
  } catch {
    return null;
  }
}

/** Заранее читает и декодирует все звуки. Безопасно вызывать много раз; ошибки глушатся. */
export function preloadSounds(): Promise<void> {
  if (preloading) return preloading;
  const ctx = audioContext();
  if (!ctx || typeof fetch !== 'function') return (preloading = Promise.resolve());
  preloading = Promise.all(
    (Object.keys(SOURCES) as SoundName[]).map(async (name) => {
      try {
        const response = await fetch(SOURCES[name].url);
        const data = await response.arrayBuffer();
        buffers.set(name, await ctx.decodeAudioData(data));
      } catch {
        /* звук не загрузился: для heehee есть запасной проигрыватель, остальные молчат */
      }
    }),
  ).then(() => undefined);
  return preloading;
}

/** Браузер запускает звук только после действия пользователя: будим контекст при первом нажатии. */
export function unlockAudio(): void {
  try {
    if (context && context.state !== 'running') void context.resume().catch(() => {});
  } catch {
    /* без звука */
  }
}

function playBuffer(name: SoundName): boolean {
  const ctx = context;
  const buffer = buffers.get(name);
  if (!ctx || !buffer) return false;
  try {
    if (ctx.state !== 'running') void ctx.resume().catch(() => {});
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    const gain = ctx.createGain();
    gain.gain.value = SOURCES[name].gain;
    source.connect(gain).connect(ctx.destination);
    source.start(0);
    return true;
  } catch {
    return false;
  }
}

/** Проигрывает звук пасхалки, если звук включён. Ошибки воспроизведения глушатся. */
export function playHeehee(): void {
  if (!readSoundEnabled()) return;
  if (playBuffer('heehee')) return;
  try {
    const audio = new Audio(heeheeUrl);
    void audio.play()?.catch(() => {});
  } catch {
    /* окружение без звука */
  }
}

/** Тихий щелчок интерфейса, если звук включён. Пока звук не готов, молчит (без запасного плеера). */
export function playUiSound(kind: 'click' | 'switch'): void {
  if (!readSoundEnabled()) return;
  playBuffer(kind);
}

/**
 * Какой звук у элемента: «switch» для разделов и вкладок, «click» для остальных кнопок и флажков,
 * null если звука нет (не кнопка, недоступна, постер с собственным звуком, отметка data-no-ui-sound).
 */
export function soundKindFor(target: EventTarget | null): 'click' | 'switch' | null {
  if (!(target instanceof Element)) return null;
  const el = target.closest(
    'button, [role="button"], [role="tab"], [role="checkbox"], [role="switch"], input[type="checkbox"], input[type="radio"]',
  );
  if (!el) return null;
  if ((el as HTMLButtonElement).disabled || el.getAttribute('aria-disabled') === 'true')
    return null;
  if (el.closest('[data-no-ui-sound], .poster, .pill.sound')) return null;
  if (el.closest('.rail, .subtabs, [role="tablist"]')) return 'switch';
  return 'click';
}

/** Окно, в котором второй звук от того же действия считается дублем. */
const DEDUPE_MS = 60;

/** Время последнего звука действия: общее для нажатий и переключений перетаскиванием. */
let lastActionSound = -Infinity;

/**
 * Звук одного действия с защитой от дубля (60 мс). Нажатие и перетаскивание по вкладкам
 * и разделам играют через неё, поэтому одно действие не звучит дважды.
 */
export function playActionSound(kind: 'click' | 'switch'): void {
  const now = performance.now();
  if (now - lastActionSound < DEDUPE_MS) return;
  lastActionSound = now;
  playUiSound(kind);
}

/**
 * Подключает щелчки к окну: мышь и касание играют при нажатии (без ожидания отпускания),
 * клавиатура (Enter, пробел) при активации. Возвращает функцию отключения.
 */
export function attachUiSounds(target: Window | HTMLElement = window): () => void {
  // Нажатие и следующий за ним клик (в том числе вызванный программно) это одно действие:
  // второй звук в пределах окна глушится общей защитой playActionSound.
  lastActionSound = -Infinity;
  const play = playActionSound;
  const onDown = (event: Event) => {
    const e = event as PointerEvent;
    unlockAudio();
    if (e.button !== undefined && e.button !== 0) return;
    const kind = soundKindFor(e.target);
    if (kind) play(kind);
  };
  const onClick = (event: Event) => {
    // detail === 0: клик от клавиатуры, мышиный уже сыграл при нажатии.
    if ((event as MouseEvent).detail !== 0) return;
    const kind = soundKindFor(event.target);
    if (kind) play(kind);
  };
  target.addEventListener('pointerdown', onDown, true);
  target.addEventListener('click', onClick, true);
  return () => {
    target.removeEventListener('pointerdown', onDown, true);
    target.removeEventListener('click', onClick, true);
  };
}

/** Только для тестов: сброс внутреннего состояния. */
export function resetSoundForTests(): void {
  context = null;
  lastActionSound = -Infinity;
  preloading = null;
  buffers.clear();
}
