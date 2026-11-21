<script lang="ts">
  import FloatingField from './FloatingField.svelte';
  import {
    decoyChars,
    MAX_RENDERED,
    mutateDecoy,
    splitSecret,
    type DecoyChar,
  } from '../lib/secretMask';

  /** Поле для секретной ссылки: плавающая подпись внутри поля. Пока поле не редактируется, вместо
      ссылки показано оформление: адрес сервера стоит неподвижно, а вместо токена — вымышленные
      случайные символы, которые изредка по одному плавно заменяются другими. Приманка не выводится
      из символов токена (от секрета зависит только их число). В фокусе поле обычное: можно править
      настоящую ссылку (в этот момент она видна). Значение `value` не меняется и уходит в импорт.
      Это оформление, а не защита от снимка экрана. */
  let {
    value = $bindable(''),
    label = 'Ссылка подписки',
    onenter,
  }: { value?: string; label?: string; onenter?: () => void } = $props();

  /** Пауза между заменами: одна-две буквы раз в ~0,7 с. */
  const TICK_MS = 700;
  /** Ширина ячейки вымышленного знака в em: ровные ячейки того же шрифта, строка не «плывёт». */
  const CELL_EM = 0.72;

  let focused = $state(false);
  let visible = $state(true);
  let line = $state<HTMLElement | null>(null);
  /** Замены после первого показа: { для какой ссылки (длина), набор }. */
  let swapped = $state<{ key: number; chars: DecoyChar[] } | null>(null);

  const hasValue = $derived(value !== '');
  const showOverlay = $derived(hasValue && !focused);
  const parts = $derived(splitSecret(value));
  const rendered = $derived(Math.min(parts.secretLength, MAX_RENDERED));
  // Набор считается сразу вместе с показом (без эффекта): первого кадра без приманок не бывает.
  // Он пересоздаётся только при новой ссылке (другое число скрытых символов).
  const initial = $derived(decoyChars(rendered));
  const secret = $derived(swapped && swapped.key === rendered ? swapped.chars : initial);

  $effect(() => {
    const onVisibility = () => (visible = document.visibilityState !== 'hidden');
    onVisibility();
    document.addEventListener('visibilitychange', onVisibility);
    return () => document.removeEventListener('visibilitychange', onVisibility);
  });

  /** Сколько знаков сейчас помещается в поле: меняем только их, остальные скрыты краем поля. */
  function reachable(): number {
    if (!line) return rendered;
    const em = parseFloat(getComputedStyle(line).fontSize) || 16;
    const open = line.querySelector<HTMLElement>('.open')?.offsetWidth ?? 0;
    const cells = Math.ceil((line.clientWidth - open) / (em * CELL_EM)) + 1;
    return cells > 0 ? Math.min(rendered, cells) : rendered;
  }

  $effect(() => {
    if (!showOverlay || !visible || rendered === 0) return;
    if (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) return;
    const key = rendered;
    const timer = setInterval(() => {
      const base = swapped && swapped.key === key ? swapped.chars : secret;
      const head = mutateDecoy(base.slice(0, reachable()), Math.random() < 0.3 ? 2 : 1);
      swapped = { key, chars: [...head, ...base.slice(head.length)] };
    }, TICK_MS);
    return () => clearInterval(timer);
  });
</script>

<FloatingField bind:value bind:focused {label} {onenter} hideText={showOverlay}>
  {#snippet overlay()}
    <span class="line" bind:this={line}
      ><span class="open">{parts.open}</span><span class="secret"
        >{#each secret as c (c.id)}<span class="ch" class:fresh={c.fresh}>{c.ch}</span>{/each}</span
      ></span
    >
  {/snippet}
</FloatingField>

<style>
  /* Вся строка — один шрифт, размер, толщина и цвет (наследуются от поля) и одна базовая линия.
     Длинная строка не обрывается посреди буквы у края поля, а мягко растворяется. */
  .line {
    display: block;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: pre;
    mask-image: linear-gradient(to right, #000 calc(100% - 18px), transparent);
  }

  /* Ячейка фиксированной ширины: замена знака не сдвигает строку. */
  .ch {
    display: inline-block;
    width: 0.72em;
    text-align: center;
  }

  /* Мягко появляется только заменённый знак; начальная приманка видна сразу, без мигания. */
  .ch.fresh {
    animation: ch-in 0.9s ease both;
  }

  @keyframes ch-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .ch.fresh {
      animation: none;
    }
  }
</style>
