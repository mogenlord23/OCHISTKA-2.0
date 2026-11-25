/** Редкое подмигивание постера: случайный интервал, пропуск когда нельзя, полная очистка таймера. */
export const WINK_MIN_MS = 20_000;
export const WINK_MAX_MS = 40_000;

export interface WinkOptions {
  /** Можно ли подмигнуть прямо сейчас (окно видно и в фокусе, нет нажатия, нет reduced-motion). */
  canWink: () => boolean;
  wink: () => void;
  random?: () => number;
}

/** Запускает расписание. Возвращает функцию остановки (вызывать при размонтировании). */
export function startWinking({ canWink, wink, random = Math.random }: WinkOptions): () => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  const schedule = () => {
    timer = setTimeout(
      () => {
        timer = null;
        if (canWink()) wink();
        schedule();
      },
      WINK_MIN_MS + random() * (WINK_MAX_MS - WINK_MIN_MS),
    );
  };
  schedule();
  return () => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
  };
}

/** Окно видно, в фокусе и пользователь не просил меньше движения. */
export function pageAllowsWink(): boolean {
  try {
    if (document.visibilityState !== 'visible' || !document.hasFocus()) return false;
    return !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  } catch {
    return false;
  }
}

/** Расположение глаза в координатах картинки 1024x1536 (правый глаз зрителя). */
export const EYE = { x: 612, y: 755, w: 123, h: 57, imgW: 1024, imgH: 1536, focusY: 0.43 };

/** Масштаб и сдвиг портрета внутри панели, включая видимые поля. */
export function portraitGeometry(boxW: number, boxH: number) {
  const scale = Math.min(boxW / EYE.imgW, boxH / EYE.imgH);
  return {
    scale,
    offsetX: (boxW - EYE.imgW * scale) * 0.5,
    offsetY: (boxH - EYE.imgH * scale) * EYE.focusY,
  };
}
