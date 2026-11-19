/** Защита оболочки: стандартное меню WebView не показывается, кроме полей ввода и
    выделяемого текста (`.selectable`), где нужны «Копировать / Вставить». Собственные меню
    приложения ставят свой обработчик и работают как раньше. */
export const TEXT_SURFACE = 'input, textarea, [contenteditable="true"], .selectable';

/** Можно ли показать стандартное меню для этого элемента. */
export function allowsNativeMenu(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest(TEXT_SURFACE) !== null;
}

/** Вешает запрет на переданный объект (окно или документ); возвращает отписку. */
export function attachChromeGuards(target: Window | Document): () => void {
  const onContextMenu = (event: Event) => {
    if (!allowsNativeMenu(event.target)) event.preventDefault();
  };
  target.addEventListener('contextmenu', onContextMenu);
  return () => target.removeEventListener('contextmenu', onContextMenu);
}
