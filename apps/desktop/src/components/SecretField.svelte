<script lang="ts">
  import FloatingField from './FloatingField.svelte';
  import { decoyChars, mutateDecoy, splitSecret, type DecoyChar } from '../lib/secretMask';

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

  let focused = $state(false);
  let visible = $state(true);
  let secret = $state<DecoyChar[]>([]);

  const hasValue = $derived(value !== '');
  const showOverlay = $derived(hasValue && !focused);
  const parts = $derived(splitSecret(value));

  // Новая приманка только при смене числа скрытых символов (то есть при новой ссылке).
  $effect(() => {
    const length = parts.secretLength;
    secret = decoyChars(length);
  });

  $effect(() => {
    const onVisibility = () => (visible = document.visibilityState !== 'hidden');
    onVisibility();
    document.addEventListener('visibilitychange', onVisibility);
    return () => document.removeEventListener('visibilitychange', onVisibility);
  });

  $effect(() => {
    if (!showOverlay || !visible || parts.secretLength === 0) return;
    if (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) return;
    const timer = setInterval(() => {
      secret = mutateDecoy(secret, Math.random() < 0.3 ? 2 : 1);
    }, TICK_MS);
    return () => clearInterval(timer);
  });
</script>

<FloatingField bind:value bind:focused {label} {onenter} hideText={showOverlay}>
  {#snippet overlay()}
    <span class="line">
      <span class="open">{parts.open}</span>
      {#if secret.length > 0}
        <span class="secret">
          {#each secret as c (c.id)}<span class="ch">{c.ch}</span>{/each}
        </span>
      {/if}
    </span>
  {/snippet}
</FloatingField>

<style>
  /* Длинная строка не обрывается посреди буквы у края поля, а мягко растворяется. */
  .line {
    display: flex;
    width: 100%;
    overflow: hidden;
    mask-image: linear-gradient(to right, #000 calc(100% - 18px), transparent);
  }

  .open,
  .secret {
    white-space: pre;
  }

  /* Моноширинный шрифт: замена буквы не двигает строку. */
  .secret {
    font-family: ui-monospace, 'Cascadia Mono', Consolas, 'Liberation Mono', monospace;
    font-size: inherit;
  }

  .ch {
    display: inline-block;
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
    .ch {
      animation: none;
    }
  }
</style>
