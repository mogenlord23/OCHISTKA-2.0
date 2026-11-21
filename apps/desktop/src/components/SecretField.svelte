<script lang="ts">
  import { onDestroy } from 'svelte';
  import { maskSubscriptionUrl, scrambleFrame } from '../lib/secretMask';

  /** Поле для секретной ссылки: плавающая подпись, маска (видны схема и сервер) и показ по кнопке.
      Значение `value` всегда настоящее: именно оно уходит в импорт. */
  let {
    value = $bindable(''),
    label = 'Ссылка подписки',
    onenter,
  }: { value?: string; label?: string; onenter?: () => void } = $props();

  let revealed = $state(false);
  let focused = $state(false);
  let hovered = $state(false);
  let scramble = $state<string | null>(null);
  let timer: ReturnType<typeof setInterval> | undefined;

  const hasValue = $derived(value !== '');
  const lifted = $derived(focused || hovered || hasValue);
  const masked = $derived(!revealed && scramble === null);
  const overlay = $derived(scramble ?? maskSubscriptionUrl(value));
  const showOverlay = $derived(hasValue && ((masked && !focused) || scramble !== null));

  function stop() {
    clearInterval(timer);
    timer = undefined;
  }

  function reveal() {
    const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    if (reduced || value.length === 0) {
      revealed = true;
      return;
    }
    // Лёгкий эффект «расшифровки»: ~240 мс, 8 кадров, символы слева направо.
    let step = 0;
    stop();
    scramble = scrambleFrame(value, 0);
    timer = setInterval(() => {
      step += 1;
      if (step >= 8) {
        stop();
        scramble = null;
        revealed = true;
      } else {
        scramble = scrambleFrame(value, Math.ceil((value.length * step) / 8));
      }
    }, 30);
  }

  function toggle() {
    if (revealed) {
      stop();
      scramble = null;
      revealed = false;
    } else {
      reveal();
    }
  }

  onDestroy(stop);
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
    class:masked
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
    <span class="overlay" aria-hidden="true">{overlay}</span>
  {/if}
  <button
    type="button"
    class="eye"
    aria-pressed={revealed}
    aria-label={revealed ? 'Скрыть ссылку' : 'Показать ссылку'}
    title={revealed ? 'Скрыть ссылку' : 'Показать ссылку'}
    onclick={toggle}
  >
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M2 12s3.6-6.5 10-6.5S22 12 22 12s-3.6 6.5-10 6.5S2 12 2 12z" />
      <circle cx="12" cy="12" r="3" />
      {#if !revealed}<path d="M4 4l16 16" />{/if}
    </svg>
  </button>
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
    padding: 20px 46px 6px 16px;
    border: 0;
    border-radius: inherit;
    background: transparent;
    color: var(--ink);
    font: inherit;
    outline: none;
  }

  /* Скрыто и поле в фокусе: ввод идёт точками, счёт символов настоящий, каретка на месте. */
  input.masked {
    -webkit-text-security: disc;
  }

  /* Скрыто и вне фокуса: вместо текста показана маска поверх (схема и сервер видны). */
  input.hide-text {
    color: transparent;
    -webkit-text-security: none;
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
    right: 46px;
    overflow: hidden;
    color: var(--ink);
    text-overflow: clip;
  }

  .eye {
    position: absolute;
    top: 50%;
    right: 6px;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink);
    transform: translateY(-50%);
    cursor: pointer;
  }

  .eye:hover {
    background: rgba(255, 255, 255, 0.09);
  }

  .eye:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: -2px;
  }

  .eye svg {
    width: 20px;
    height: 20px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  @media (prefers-reduced-motion: reduce) {
    .field,
    .lbl {
      transition: none;
    }
  }
</style>
