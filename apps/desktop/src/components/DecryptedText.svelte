<script lang="ts">
  /**
   * Decrypted Text: порт на Svelte 5 с оригинала React Bits (David Haz, MIT + Commons Clause),
   * https://reactbits.dev/text-animations/decrypted-text, исходник DecryptedText.jsx.
   * Перенесены: движок кадров (маска раскрытия, шаг `speed`, `maxIterations`, режим `sequential`,
   * `revealDirection`), пул символов, сжатие подменённого символа до ширины настоящего (fit),
   * запуск по наведению / показу / клику, зацикливание, эффекты `scramble` и `fade`,
   * prefers-reduced-motion (без анимации).
   * Не перенесены: эффект `flip` и локальное «нагревание» вокруг курсора (`hoverRadius`).
   * Дополнение для OCHISTKA: метод `play()` для запуска из кода.
   * ВАЖНО: в `text` передаётся только безопасное или уже замаскированное представление. Подмена букв
   * берётся из пула случайных символов, настоящая секретная строка в компонент не попадает.
   */
  import { onDestroy, onMount } from 'svelte';

  let {
    text,
    speed = 50,
    maxIterations = 10,
    sequential = false,
    revealDirection = 'start',
    useOriginalCharsOnly = false,
    characters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!@#$%^&*()_+',
    class: className = '',
    parentClass = '',
    encryptedClass = '',
    animateOn = 'hover',
    clickMode = 'once',
    effect: effectKind = 'scramble',
    scrambleColor = '',
    loop = false,
    loopDelay = 2000,
  }: {
    text: string;
    speed?: number;
    maxIterations?: number;
    sequential?: boolean;
    revealDirection?: 'start' | 'end' | 'center' | 'random';
    useOriginalCharsOnly?: boolean;
    characters?: string;
    class?: string;
    parentClass?: string;
    encryptedClass?: string;
    animateOn?: 'hover' | 'view' | 'click' | 'inViewHover' | 'manual';
    clickMode?: 'once' | 'toggle';
    effect?: 'scramble' | 'fade';
    scrambleColor?: string;
    loop?: boolean;
    loopDelay?: number;
  } = $props();

  const FADE_FLOOR = 0.16;
  const FADE_PEAK = 0.62;
  const FADE_LEAD = 6;
  const SETTLE =
    'color 0.7s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.6s cubic-bezier(0.22, 1, 0.36, 1)';

  const isSpace = (char: string) => /\s/.test(char);
  const prefersReducedMotion = () =>
    window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;

  // Измерение ширины символов для сжатия подменённых (как advance/readFont в оригинале).
  const glyphWidths = new Map<string, Map<string, number>>();
  let measureContext: CanvasRenderingContext2D | null = null;
  const readFont = (node: Element) => {
    const style = window.getComputedStyle(node);
    return {
      font: `${style.fontStyle} ${style.fontWeight} ${style.fontSize} ${style.fontFamily}`,
      spacing: parseFloat(style.letterSpacing) || 0,
    };
  };
  const advance = (font: string, glyph: string) => {
    let widths = glyphWidths.get(font);
    if (!widths) {
      widths = new Map();
      glyphWidths.set(font, widths);
    }
    let width = widths.get(glyph);
    if (width === undefined) {
      try {
        measureContext = measureContext || document.createElement('canvas').getContext('2d');
      } catch {
        measureContext = null;
      }
      if (!measureContext) return 0;
      measureContext.font = font;
      width = measureContext.measureText(glyph).width;
      widths.set(glyph, width);
    }
    return width;
  };

  type Frame = {
    shown: string[];
    revealed: boolean[];
    tint: number[];
    dim: number[];
    squeeze: number[];
  };

  const chars = $derived(Array.from(String(text ?? '')));
  const pool = $derived.by(() => {
    const letters = chars.filter((char) => !isSpace(char));
    const source = useOriginalCharsOnly ? Array.from(new Set(letters)) : Array.from(characters);
    return source.length ? source : letters;
  });
  const startsHidden = $derived(
    animateOn === 'click' || animateOn === 'view' || animateOn === 'inViewHover',
  );
  const hoverable = $derived(animateOn === 'hover' || animateOn === 'inViewHover');
  const step = $derived(Math.max(16, speed));

  let frame = $state<Frame>({ shown: [], revealed: [], tint: [], dim: [], squeeze: [] });

  let container: HTMLElement | undefined;
  let mask: boolean[] = [];
  let run: {
    forward: boolean;
    cycle: boolean;
    iteration: number;
    queue: (number | null)[];
  } | null = null;
  let timer: ReturnType<typeof setInterval> | null = null;
  let hold: ReturnType<typeof setTimeout> | null = null;
  let pending: (() => void) | null = null;
  let visible = true;
  let target = true;
  let played = false;
  let fontMetrics: { font: string; spacing: number } | null = null;

  const revealOrder = () => {
    const order: number[] = [];
    chars.forEach((char, index) => {
      if (!isSpace(char)) order.push(index);
    });
    if (revealDirection === 'end') return order.reverse();
    if (revealDirection === 'center') {
      const middle = Math.floor(chars.length / 2);
      return order.sort((a, b) => Math.abs(a - middle) - Math.abs(b - middle));
    }
    if (revealDirection === 'random') {
      for (let i = order.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [order[i], order[j]] = [order[j], order[i]];
      }
    }
    return order;
  };

  const fit = (index: number, glyph: string) => {
    if (!fontMetrics && container) fontMetrics = readFont(container);
    const metrics = fontMetrics;
    if (!metrics) return 1;
    const room = advance(metrics.font, chars[index] ?? glyph) + metrics.spacing;
    const need = advance(metrics.font, glyph) + metrics.spacing;
    return need > room + 0.01 ? Math.round(Math.max(0.05, room / need) * 1000) / 1000 : 1;
  };

  const paint = () => {
    const ranks =
      run?.forward && sequential
        ? new Map<number | null, number>(run.queue.map((index, rank) => [index, rank]))
        : null;
    const ramp =
      run?.forward && !sequential ? Math.min(1, run.iteration / Math.max(1, maxIterations)) : 0;
    const next: Frame = { shown: [], revealed: [], tint: [], dim: [], squeeze: [] };

    chars.forEach((char, index) => {
      const blank = isSpace(char);
      const hidden = !blank && !mask[index];
      const glyph = hidden ? pool[Math.floor(Math.random() * pool.length)] : char;
      next.revealed.push(!hidden);
      next.shown.push(glyph);
      next.squeeze.push(hidden ? fit(index, glyph) : 1);
      if (!hidden) {
        next.tint.push(0);
        next.dim.push(1);
      } else {
        const rank = ranks?.get(index);
        const lift = rank === undefined ? ramp : Math.max(0, 1 - rank / FADE_LEAD);
        next.tint.push(1);
        next.dim.push(FADE_FLOOR + (FADE_PEAK - FADE_FLOOR) * lift);
      }
    });
    frame = next;
  };

  const stopTicker = () => {
    if (timer) clearInterval(timer);
    timer = null;
  };
  const ensureTicker = () => {
    if (timer) return;
    timer = setInterval(tick, step);
  };
  const stopRun = () => {
    run = null;
    if (hold) clearTimeout(hold);
    hold = null;
    pending = null;
  };
  const schedule = (task: () => void, delay: number) => {
    if (hold) clearTimeout(hold);
    hold = setTimeout(() => {
      hold = null;
      if (visible) task();
      else pending = task;
    }, delay);
  };

  const startRun = (
    forward: boolean,
    { from, cycle = false, lead = 0 }: { from?: boolean[]; cycle?: boolean; lead?: number } = {},
  ) => {
    stopRun();
    target = forward;
    if (from) mask = from;

    if (prefersReducedMotion()) {
      mask = chars.map((char) => forward || isSpace(char));
      paint();
      return;
    }

    const order = forward ? revealOrder() : revealOrder().reverse();
    const queue = sequential ? order.filter((index) => mask[index] !== forward) : [];
    run = { forward, cycle, iteration: 0, queue: [...new Array(lead).fill(null), ...queue] };
    paint();
    ensureTicker();
  };

  const finishRun = (finished: NonNullable<typeof run>) => {
    if (!loop) return;
    if (finished.forward) {
      schedule(() => startRun(false, { cycle: true }), Math.max(0, loopDelay));
    } else if (finished.cycle) {
      startRun(true, { cycle: true, lead: sequential ? Math.max(2, Math.round(320 / step)) : 0 });
    }
  };

  function tick() {
    const current = run;
    if (current) {
      current.iteration += 1;
      let next = mask;
      let done: boolean;

      if (sequential) {
        const index = current.queue.shift();
        if (index !== undefined && index !== null) {
          next = next.slice();
          next[index] = current.forward;
        }
        done = current.queue.length === 0;
      } else if (current.forward) {
        done = current.iteration >= maxIterations;
        if (done) next = chars.map(() => true);
      } else {
        const shownIdx: number[] = [];
        next.forEach((shown, index) => {
          if (shown && !isSpace(chars[index])) shownIdx.push(index);
        });
        const hideCount = Math.max(
          1,
          Math.ceil(chars.filter((char) => !isSpace(char)).length / Math.max(1, maxIterations)),
        );
        next = next.slice();
        for (let i = 0; i < hideCount && shownIdx.length; i++) {
          next[shownIdx.splice(Math.floor(Math.random() * shownIdx.length), 1)[0]] = false;
        }
        done = shownIdx.length === 0 || current.iteration >= maxIterations;
        if (done) next = chars.map((char) => isSpace(char));
      }

      mask = next;
      if (done) {
        run = null;
        finishRun(current);
      }
    }
    paint();
    if (!run) stopTicker();
  }

  // Начальное состояние и сброс при смене текста или режима запуска.
  $effect(() => {
    void animateOn;
    void chars;
    stopRun();
    played = false;
    const hidden =
      animateOn === 'click' ||
      ((animateOn === 'view' || animateOn === 'inViewHover') && !prefersReducedMotion());
    target = !hidden;
    mask = chars.map((char) => !hidden || isSpace(char));
    paint();
  });

  // Смена скорости перезапускает таймер.
  $effect(() => {
    void step;
    if (!timer) return;
    clearInterval(timer);
    timer = null;
    ensureTicker();
  });

  onMount(() => {
    // Размер шрифта изменился: ширины подменённых символов считаем заново.
    let observer: ResizeObserver | null = null;
    if (container && typeof ResizeObserver !== 'undefined') {
      observer = new ResizeObserver(() => {
        if (!container) return;
        const metrics = readFont(container);
        glyphWidths.delete(metrics.font);
        fontMetrics = metrics;
        frame = {
          ...frame,
          squeeze: frame.shown.map((glyph, index) =>
            frame.revealed[index] ? 1 : fit(index, glyph),
          ),
        };
      });
      observer.observe(container);
    }
    // Запуск по появлению в окне (view / inViewHover) и пауза цикла, пока слово не видно.
    let view: IntersectionObserver | null = null;
    const viewTrigger = animateOn === 'view' || animateOn === 'inViewHover';
    if ((viewTrigger || loop) && container && typeof IntersectionObserver !== 'undefined') {
      view = new IntersectionObserver(
        (entries) => {
          visible = entries.some((entry) => entry.isIntersecting);
          if (!visible) return;
          if (viewTrigger && !played) {
            played = true;
            startRun(true);
          }
          const resume = pending;
          pending = null;
          resume?.();
          if (!loop) view?.disconnect();
        },
        { threshold: 0.1 },
      );
      view.observe(container);
    }
    return () => {
      observer?.disconnect();
      view?.disconnect();
      visible = true;
    };
  });

  onDestroy(() => {
    stopTicker();
    if (hold) clearTimeout(hold);
  });

  /** Запуск из кода: все буквы скрываются и восстанавливаются, как при наведении. */
  export function play() {
    startRun(true, { from: chars.map((char) => isSpace(char)) });
  }

  const onEnter = () => {
    if (run) return;
    play();
  };
  const onLeave = () => {
    stopRun();
    target = true;
    mask = chars.map(() => true);
    paint();
  };
  const onClick = () => {
    if (clickMode === 'toggle') {
      startRun(!target);
      return;
    }
    if (!target) startRun(true);
  };

  const groups = $derived.by(() => {
    const result: (string | { key: number; letters: number[] })[] = [];
    let word: { key: number; letters: number[] } | null = null;
    chars.forEach((char, index) => {
      if (isSpace(char)) {
        result.push(char);
        word = null;
        return;
      }
      if (!word) {
        word = { key: index, letters: [] };
        result.push(word);
      }
      word.letters.push(index);
    });
    return result;
  });

  const tinted = $derived(Boolean(scrambleColor));
  const fading = $derived(effectKind === 'fade');
  const settles = $derived(tinted || fading);

  const glyphStyle = (index: number) => {
    const squeeze = frame.squeeze[index] ?? 1;
    return squeeze < 1
      ? `position:absolute;top:0;left:50%;transform:translateX(-50%) scaleX(${squeeze})`
      : 'position:absolute;top:0;left:50%;transform:translateX(-50%)';
  };

  const cellStyle = (index: number, revealed: boolean) => {
    const parts: string[] = [];
    if (!revealed) parts.push('position:relative', 'display:inline-block');
    if (settles) {
      parts.push(
        `transition:${revealed ? SETTLE : `color 0.12s linear, opacity ${step}ms linear`}`,
      );
    }
    if (tinted && !revealed) {
      const amount = Math.round((frame.tint[index] ?? 1) * 100);
      parts.push(
        `color:${amount >= 100 ? scrambleColor : `color-mix(in srgb, ${scrambleColor} ${amount}%, currentColor)`}`,
      );
    }
    if (fading) parts.push(`opacity:${revealed ? 1 : (frame.dim[index] ?? FADE_FLOOR)}`);
    return parts.join(';');
  };
</script>

<span
  class={parentClass}
  bind:this={container}
  style="display:inline-block;white-space:pre-wrap"
  role="presentation"
  onmouseenter={hoverable ? onEnter : undefined}
  onmouseleave={hoverable ? onLeave : undefined}
  onclick={animateOn === 'click' ? onClick : undefined}
>
  <span class="sr-only">{chars.join('')}</span>
  <span aria-hidden="true">
    {#each groups as group, gi (gi)}
      {#if typeof group === 'string'}
        {group}
      {:else}
        <span style="white-space:nowrap">
          {#each group.letters as index (index)}
            {@const revealed = frame.revealed[index] ?? true}
            <span
              class={revealed ? className : encryptedClass}
              style={cellStyle(index, revealed)}
            >
              {#if revealed}{chars[index]}{:else}<span style="visibility:hidden"
                  >{chars[index]}</span
                ><span style={glyphStyle(index)}>{frame.shown[index] ?? chars[index]}</span>{/if}
            </span>
          {/each}
        </span>
      {/if}
    {/each}
  </span>
</span>
