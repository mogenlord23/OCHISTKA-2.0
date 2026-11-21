<script lang="ts">
  import DecryptedText from './DecryptedText.svelte';
  import { segments, visualLine } from '../lib/secretMask';

  /** Поле для секретной ссылки: плавающая подпись внутри поля. Пока поле не редактируется, вместо
      ссылки показана «приманка»: адрес сервера и случайные символы вместо токена, по которым спокойно
      пробегает эффект Decrypted Text (порт React Bits): участки строки перемешиваются по очереди.
      В фокусе поле обычное: можно править, выделять и вставлять настоящую ссылку. Настоящее значение
      `value` не меняется и уходит в импорт. Приманка не выводится из символов токена. */
  let {
    value = $bindable(''),
    label = 'Ссылка подписки',
    onenter,
  }: { value?: string; label?: string; onenter?: () => void } = $props();

  const PARTS = 3;
  const PERIOD = 4200; // один круг: участок 1, участок 2, участок 3 и пауза
  const GAP = 1100;

  let focused = $state(false);
  let hovered = $state(false);
  let parts = $state<({ play: () => void } | undefined)[]>([]);

  const hasValue = $derived(value !== '');
  const lifted = $derived(focused || hovered || hasValue);
  const showOverlay = $derived(hasValue && !focused);
  // Новая приманка только при смене ссылки, а не на каждый кадр анимации.
  const line = $derived(visualLine(value));
  const chunks = $derived(segments(line, PARTS));

  $effect(() => {
    if (!showOverlay || chunks.length === 0) return;
    if (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) return;
    const timers: ReturnType<typeof setTimeout>[] = [];
    const circle = () => {
      chunks.forEach((_, i) => timers.push(setTimeout(() => parts[i]?.play(), i * GAP)));
    };
    timers.push(setTimeout(circle, 500));
    const loop = setInterval(circle, PERIOD + chunks.length * GAP);
    return () => {
      timers.forEach(clearTimeout);
      clearInterval(loop);
    };
  });
</script>

<div
  class="field"
  class:lifted
  class:focused
  role="group"
  onmouseenter={() => (hovered = true)}
  onmouseleave={() => (hovered = false)}
>
  <input
    bind:value
    class:hide-text={showOverlay}
    type="text"
    autocomplete="off"
    spellcheck="false"
    aria-label={label}
    onfocus={() => (focused = true)}
    onblur={() => (focused = false)}
    onkeydown={(e) => e.key === 'Enter' && onenter?.()}
  />
  <span class="lbl" aria-hidden="true">{label}</span>
  {#if showOverlay}
    <span class="overlay" aria-hidden="true">
      {#each chunks as chunk, i (i)}
        <DecryptedText
          bind:this={parts[i]}
          text={chunk}
          animateOn="manual"
          speed={70}
          maxIterations={12}
          sequential={false}
        />
      {/each}
    </span>
  {/if}
</div>

<style>
  .field {
    position: relative;
    flex: 1;
    min-width: 0;
    height: 52px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--black);
    transition: border-color 160ms ease;
  }

  .field:hover {
    border-color: #5a5a5a;
  }

  .field.focused {
    border-color: var(--accent);
  }

  input {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 20px 16px 6px;
    border: 0;
    border-radius: inherit;
    background: transparent;
    color: var(--ink);
    font: inherit;
    outline: none;
  }

  /* Не редактируется: вместо настоящего текста показана приманка поверх. */
  input.hide-text {
    color: transparent;
  }

  .lbl,
  .overlay {
    position: absolute;
    left: 16px;
    pointer-events: none;
    white-space: nowrap;
  }

  .lbl {
    top: 15px;
    color: var(--muted, #8b8b8b);
    transform-origin: left top;
    transition:
      transform 160ms cubic-bezier(0.2, 0, 0, 1),
      color 160ms ease;
  }

  .field.lifted .lbl {
    transform: translateY(-9px) scale(0.76);
  }

  .field.focused .lbl {
    color: var(--accent);
  }

  .overlay {
    top: 20px;
    right: 16px;
    /* Участки строки стоят вплотную, без пробелов между ними. */
    display: flex;
    overflow: hidden;
    color: var(--ink);
    text-overflow: clip;
  }

  @media (prefers-reduced-motion: reduce) {
    .field,
    .lbl {
      transition: none;
    }
  }
</style>
