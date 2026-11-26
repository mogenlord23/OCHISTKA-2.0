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
    // Событие размера приходит десятки раз в секунду при перетаскивании края окна.
    // Каждый вызов isMaximized это обмен с ядром, поэтому читаем один раз, когда движение стихло.
    let timer: ReturnType<typeof setTimeout> | undefined;
    const schedule = () => {
      clearTimeout(timer);
      timer = setTimeout(() => void read(), 120);
    };
    void Promise.resolve(win.onResized(schedule))
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

/** Some WebView2 windows stay document-visible after the native window is minimized. */
export function watchWindowVisibility(change: (visible: boolean) => void): () => void {
  let stopped = false;
  let revision = 0;
  let timer: ReturnType<typeof setTimeout>;
  const stops: Array<() => void> = [];
  const win = getCurrentWindow();
  async function read() {
    const current = ++revision;
    try {
      const [visible, minimized] = await Promise.all([win.isVisible(), win.isMinimized()]);
      if (!stopped && current === revision && typeof visible === 'boolean') change(visible && !minimized);
    } catch { /* In a browser document visibility is sufficient. */ }
  }
  function schedule() {
    clearTimeout(timer);
    timer = setTimeout(read, 100);
  }
  for (const subscription of [win.onResized(schedule), win.onFocusChanged(schedule), win.listen('main-window-visibility', schedule)]) {
    void subscription.then(stop => stopped ? stop() : stops.push(stop)).catch(() => {});
  }
  void read();
  return () => { stopped = true; clearTimeout(timer); for (const stop of stops) stop(); };
}
