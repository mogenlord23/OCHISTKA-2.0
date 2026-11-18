import { getCurrentWindow } from '@tauri-apps/api/window';

/** Следит за состоянием «развёрнуто». Вне Tauri (тесты, браузер) тихо ничего не делает. */
export function watchMaximized(onChange: (maximized: boolean) => void): () => void {
  let stopped = false;
  let unlisten: (() => void) | undefined;
  try {
    const win = getCurrentWindow();
    const read = async () => {
      try {
        const value = await win.isMaximized();
        if (!stopped) onChange(value);
      } catch {
        /* окно недоступно: остаёмся в обычном виде */
      }
    };
    void read();
    void Promise.resolve(win.onResized(() => void read()))
      .then((fn) => {
        if (stopped) fn();
        else unlisten = fn;
      })
      .catch(() => {});
  } catch {
    /* не Tauri */
  }
  return () => {
    stopped = true;
    unlisten?.();
  };
}
