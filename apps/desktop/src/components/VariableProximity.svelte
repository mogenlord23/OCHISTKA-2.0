<script lang="ts">
  /**
   * Variable Proximity: порт на Svelte 5 с оригинала React Bits (David Haz, MIT + Commons Clause),
   * https://reactbits.dev/text-animations/variable-proximity, исходник VariableProximity.jsx.
   * Перенесены алгоритм и параметры: расстояние от курсора до центра каждой буквы → сила (linear,
   * exponential, gaussian) → сглаживание по времени (smoothing) → `font-variation-settings` по осям,
   * цвет (color-mix), масштаб и подъём, ожидающая «волна» (idleWave).
   * Не перенесён перенос слова на несколько строк (lineStarts): нашему логотипу он не нужен.
   */

  let {
    label = '',
    fromFontVariationSettings = "'wght' 400, 'opsz' 9",
    toFontVariationSettings = "'wght' 1000, 'opsz' 40",
    containerRef = null,
    radius = 50,
    falloff = 'linear',
    smoothing = 0.2,
    fromColor,
    toColor,
    scale = 1,
    lift = 0,
    idleWave = false,
    class: className = '',
  }: {
    label?: string;
    fromFontVariationSettings?: string;
    toFontVariationSettings?: string;
    containerRef?: HTMLElement | null;
    radius?: number;
    falloff?: 'linear' | 'exponential' | 'gaussian';
    smoothing?: number;
    fromColor?: string;
    toColor?: string;
    scale?: number;
    lift?: number;
    idleWave?: boolean;
    class?: string;
  } = $props();

  const GAUSSIAN_EDGE = Math.exp(-2);
  const WAVE_PERIOD = 3.2;
  const WAVE_PEAK = 0.7;

  const parseSettings = (settings: string) =>
    new Map<string, number>(
      settings
        .split(',')
        .map((part) => part.trim())
        .filter(Boolean)
        .map((part) => {
          const [name, value] = part.split(/\s+/);
          return [name.replace(/['"]/g, ''), parseFloat(value)] as [string, number];
        }),
    );

  const strengthAt = (distance: number, r: number, mode: string) => {
    if (distance >= r) return 0;
    const norm = 1 - distance / r;
    switch (mode) {
      case 'exponential':
        return norm * norm;
      case 'gaussian':
        return (Math.exp(-2 * (distance / r) ** 2) - GAUSSIAN_EDGE) / (1 - GAUSSIAN_EDGE);
      default:
        return norm;
    }
  };

  let root = $state<HTMLElement>();
  // Состояние живёт дольше пересоздания эффекта, как stateRef в оригинале.
  const shared = {
    pointer: { x: 0, y: 0, active: false },
    levels: [] as number[],
    targets: [] as number[],
  };

  const words = $derived(label.split(' '));

  $effect(() => {
    // Зависимости эффекта: параметры и текст.
    const from = fromFontVariationSettings;
    const to = toFontVariationSettings;
    void label;
    if (!root) return;
    const letters = Array.from(root.querySelectorAll<HTMLElement>('[data-letter]'));
    const fromSettings = parseSettings(from);
    const toSettings = parseSettings(to);
    const axes = Array.from(fromSettings.entries()).map(([axis, fromValue]) => ({
      axis,
      fromValue,
      toValue: toSettings.get(axis) ?? fromValue,
    }));
    const { pointer, levels, targets } = shared;
    const written: string[] = [];
    const painted: string[] = [];
    const moved: string[] = [];
    const anchors: { x: number; y: number }[] = [];
    const tinted = Boolean(fromColor || toColor);
    const restColor = fromColor || 'currentColor';
    const nearColor = toColor || 'currentColor';
    const growth = (Number(scale) || 1) - 1;
    const rise = Number(lift) || 0;
    const transformed = growth !== 0 || rise !== 0;
    const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    const waving = idleWave && !reduced;
    let frame = 0;
    let disposed = false;
    let lastTime = 0;
    let dirty = true;
    let layoutDirty = true;
    let nearAny = false;
    let lastNear = -Infinity;
    let waveMix = 0;
    let span = 1;
    let visible = true;

    letters.forEach((letter) => {
      if (!letter) return;
      if (!tinted) letter.style.color = '';
      if (!transformed) letter.style.transform = '';
    });

    const settingsAt = (level: number) =>
      axes
        .map(
          ({ axis, fromValue, toValue }) =>
            `'${axis}' ${fromValue + (toValue - fromValue) * level}`,
        )
        .join(', ');

    const measureAnchors = () => {
      let left = Infinity;
      let top = Infinity;
      let right = -Infinity;
      letters.forEach((letter, index) => {
        if (!letter) return;
        const rect = letter.getBoundingClientRect();
        anchors[index] = { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
        left = Math.min(left, rect.left);
        top = Math.min(top, rect.top);
        right = Math.max(right, rect.right);
      });
      anchors.forEach((anchor) => {
        anchor.x -= left;
        anchor.y -= top;
      });
      span = Math.max(220, (right - left) * 0.85);
    };

    const measure = () => {
      targets.length = letters.length;
      targets.fill(0);
      nearAny = false;
      if (!pointer.active) return;

      const area = containerRef?.getBoundingClientRect();
      if (area) {
        const dx = Math.max(area.left - pointer.x, 0, pointer.x - area.right);
        const dy = Math.max(area.top - pointer.y, 0, pointer.y - area.bottom);
        if (dx * dx + dy * dy >= radius * radius) return;
      }

      letters.forEach((letter, index) => {
        if (!letter) return;
        const rect = letter.getBoundingClientRect();
        const distance = Math.hypot(
          pointer.x - (rect.left + rect.width / 2),
          pointer.y - (rect.top + rect.height / 2),
        );
        targets[index] = strengthAt(distance, radius, falloff);
        if (targets[index] > 0) nearAny = true;
      });
    };

    const waveAt = (index: number, time: number) => {
      const anchor = anchors[index];
      if (!anchor) return 0;
      const phase = ((anchor.x + anchor.y * 0.6) / span - time / WAVE_PERIOD) * Math.PI * 2;
      return WAVE_PEAK * ((1 + Math.cos(phase)) / 2) ** 2.2;
    };

    const tick = (now: number) => {
      frame = 0;
      const delta = lastTime ? Math.min(now - lastTime, 64) : 16;
      lastTime = now;

      if (dirty) {
        measure();
        dirty = false;
      }
      if (waving && layoutDirty) {
        measureAnchors();
        layoutDirty = false;
      }

      if (nearAny) lastNear = now;
      const idle = waving && visible && !nearAny && now - lastNear > 500;
      const goal = idle ? 1 : 0;
      waveMix += (goal - waveMix) * (1 - Math.exp(-delta / 600));
      if (Math.abs(goal - waveMix) < 0.002) waveMix = goal;

      const ease = smoothing > 0 ? 1 - Math.exp(-delta / (smoothing * 400)) : 1;
      const time = now / 1000;
      let settling = false;

      letters.forEach((letter, index) => {
        if (!letter) return;
        let target = targets[index] ?? 0;
        if (waveMix > 0) target = Math.max(target, waveMix * waveAt(index, time));
        const current = levels[index] ?? 0;
        let next = current + (target - current) * ease;
        if (Math.abs(target - next) < 0.002) {
          next = target;
        } else {
          settling = true;
        }
        levels[index] = next;

        const settings = next === 0 ? from : settingsAt(next);
        if (written[index] !== settings) {
          written[index] = settings;
          letter.style.fontVariationSettings = settings;
        }

        if (tinted) {
          const paint =
            next === 0
              ? fromColor || ''
              : `color-mix(in oklab, ${restColor} ${((1 - next) * 100).toFixed(1)}%, ${nearColor})`;
          if (painted[index] !== paint) {
            painted[index] = paint;
            letter.style.color = paint;
          }
        }

        if (transformed) {
          const shift =
            next === 0
              ? ''
              : `translateY(${(-rise * next).toFixed(3)}em) scale(${(1 + growth * next).toFixed(3)})`;
          if (moved[index] !== shift) {
            moved[index] = shift;
            letter.style.transform = shift;
          }
        }
      });

      if (settling || waveMix > 0 || (waving && visible && !nearAny)) {
        frame = requestAnimationFrame(tick);
      } else {
        lastTime = 0;
      }
    };

    const schedule = () => {
      dirty = true;
      if (!disposed && !frame) frame = requestAnimationFrame(tick);
    };

    const relayout = () => {
      layoutDirty = true;
      schedule();
    };

    const handleMove = (x: number, y: number) => {
      pointer.x = x;
      pointer.y = y;
      pointer.active = true;
      schedule();
    };

    const handleMouseMove = (event: MouseEvent) => handleMove(event.clientX, event.clientY);
    const handleTouchMove = (event: TouchEvent) => {
      const touch = event.touches[0];
      if (touch) handleMove(touch.clientX, touch.clientY);
    };
    const handleLeave = () => {
      pointer.active = false;
      schedule();
    };

    const docRoot = document.documentElement;
    window.addEventListener('mousemove', handleMouseMove, { passive: true });
    window.addEventListener('touchmove', handleTouchMove, { passive: true });
    window.addEventListener('touchend', handleLeave);
    window.addEventListener('touchcancel', handleLeave);
    window.addEventListener('scroll', schedule, { capture: true, passive: true });
    window.addEventListener('resize', relayout);
    docRoot.addEventListener('mouseleave', handleLeave);

    let observer: IntersectionObserver | null = null;
    if (waving && root && 'IntersectionObserver' in window) {
      observer = new IntersectionObserver((entries) => {
        visible = entries.some((entry) => entry.isIntersecting);
        schedule();
      });
      observer.observe(root);
    }
    if (waving) void document.fonts?.ready.then(relayout);

    schedule();

    return () => {
      disposed = true;
      cancelAnimationFrame(frame);
      observer?.disconnect();
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('touchmove', handleTouchMove);
      window.removeEventListener('touchend', handleLeave);
      window.removeEventListener('touchcancel', handleLeave);
      window.removeEventListener('scroll', schedule, { capture: true });
      window.removeEventListener('resize', relayout);
      docRoot.removeEventListener('mouseleave', handleLeave);
    };
  });

</script>

<span bind:this={root} class="{className} variable-proximity" style:display="inline">
  {#each words as word, wordIndex (wordIndex)}
    <span data-word="" style:display="inline-block" style:white-space="nowrap">
      {#each word.split('') as letter, i (i)}
        <span
          data-letter
          aria-hidden="true"
          style:display="inline-block"
          style:font-variation-settings={fromFontVariationSettings}
          style:color={fromColor || undefined}
          style:transform-origin="50% 80%">{letter}</span
        >
      {/each}
      {#if wordIndex < words.length - 1}<span style:display="inline-block">&nbsp;</span>{/if}
    </span>
  {/each}
  <span class="sr-only">{label}</span>
</span>
