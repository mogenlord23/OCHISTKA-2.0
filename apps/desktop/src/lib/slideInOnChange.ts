const slide = [{ transform: 'translateY(-8px)', opacity: 0.82 }, { transform: 'none', opacity: 1 }];

export function slideInOnChange(node: HTMLElement, initial: string | null) {
  let previous = initial;
  let animation: Animation | undefined;

  return {
    update(key: string | null) {
      if (key === previous) return;
      const wasVisible = previous !== null;
      previous = key;
      animation?.cancel();
      animation = undefined;
      if (
        key !== null && wasVisible && node.animate &&
        document.documentElement.dataset.reduceMotion !== 'true' &&
        !window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
      ) {
        animation = node.animate(slide, { duration: 200, easing: 'cubic-bezier(0.2, 0, 0, 1)' });
      }
    },
    destroy() {
      animation?.cancel();
    },
  };
}
