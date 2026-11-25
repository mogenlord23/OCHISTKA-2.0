<script lang="ts">
  import type { Snippet } from 'svelte';

  /** Единый переключатель «включить / выключить» (ADR 0012). Пилюля в стиле кнопки VPN:
      выкл: тёмная с белой обводкой, ручка слева; вкл: заливка цветом раздела, ручка справа.
      Состояние показывает только родитель (`checked`): нажатие лишь сообщает желаемое значение,
      а оптимистичный показ и откат при ошибке делает родитель. Ожидание: ручка пульсирует.
      Ошибка: короткая красная обводка. Размер соразмерен тексту (высота 24 px). */
  let {
    checked = false,
    disabled = false,
    pending = false,
    error = false,
    square = false,
    label,
    ariaLabel,
    children,
    onchange,
  }: {
    checked?: boolean;
    disabled?: boolean;
    pending?: boolean;
    error?: boolean;
    square?: boolean;
    /** Подпись справа. Вместо неё можно передать содержимое. */
    label?: string;
    /** Имя для скринридера, когда видимой подписи рядом нет (подпись в соседнем тексте). */
    ariaLabel?: string;
    children?: Snippet;
    onchange?: (next: boolean) => void;
  } = $props();

  function press() {
    if (disabled) return;
    onchange?.(!checked);
  }
</script>

{#snippet control()}
  <button
    type="button"
    role="switch"
    class="track"
    class:on={checked}
    class:pending
    class:error
    class:square
    aria-checked={checked}
    aria-busy={pending}
    aria-label={ariaLabel ?? (label && !children ? label : undefined)}
    {disabled}
    onclick={press}
  >
    <span class="knob" aria-hidden="true"></span>
  </button>
{/snippet}

{#if label || children}
  <label class="switch" class:disabled>
    {@render control()}
    {#if label}<span class="text">{label}</span>{/if}
    {@render children?.()}
  </label>
{:else}
  {@render control()}
{/if}

<style>
  .switch {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }

  .switch.disabled {
    cursor: not-allowed;
  }

  .track {
    position: relative;
    flex: none;
    box-sizing: border-box;
    width: 44px;
    height: 24px;
    min-height: 0;
    padding: 0;
    border: 0;
    border-radius: var(--radius-pill);
    background: #1c1c1c;
    box-shadow: 0 0 0 2px var(--ink, #fafafa);
    cursor: pointer;
    transition:
      background-color 160ms ease,
      box-shadow 160ms ease;
  }

  .track.on {
    background: var(--accent, #f83c5c);
  }

  .track.error {
    box-shadow: 0 0 0 2px #da3633;
  }

  .track:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0 0 2px #000;
    transition: transform 160ms ease;
  }

  .track.on .knob {
    transform: translateX(20px);
  }

  .track.square {
    border-radius: var(--radius-sm);
  }

  .track.square .knob {
    border-radius: var(--radius-xs);
  }

  .track.pending .knob {
    animation: knob-pulse 700ms ease-in-out infinite;
  }

  @keyframes knob-pulse {
    50% {
      opacity: 0.45;
    }
  }

  .track:focus-visible {
    outline: 3px solid var(--page-text, #fff);
    outline-offset: 3px;
  }

  @media (prefers-reduced-motion: reduce) {
    .track,
    .knob {
      transition: none;
    }

    .track.pending .knob {
      animation: none;
    }
  }
</style>
