<script lang="ts">
  import type { Snippet } from 'svelte';

  /** Общее поле ввода OCHISTKA с плавающей подписью внутри (одна реализация для поля подписки VPN и
      полей поиска): пустое и без фокуса — подпись по центру; при наведении, фокусе или значении
      уменьшается и поднимается. `overlay` рисуется поверх, когда `hideText` включён (поле подписки). */
  let {
    value = $bindable(''),
    focused = $bindable(false),
    label,
    ariaLabel,
    type = 'text',
    id,
    hideText = false,
    onenter,
    overlay,
  }: {
    value?: string;
    focused?: boolean;
    label: string;
    ariaLabel?: string;
    type?: 'text' | 'search';
    id?: string;
    hideText?: boolean;
    onenter?: () => void;
    overlay?: Snippet;
  } = $props();

  let hovered = $state(false);
  const lifted = $derived(focused || hovered || value !== '');
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
    {id}
    bind:value
    {type}
    class:hide-text={hideText}
    autocomplete="off"
    spellcheck="false"
    aria-label={ariaLabel ?? label}
    onfocus={() => (focused = true)}
    onblur={() => (focused = false)}
    onkeydown={(e) => e.key === 'Enter' && onenter?.()}
  />
  <span class="lbl" aria-hidden="true">{label}</span>
  {#if hideText && overlay}
    <span class="overlay" aria-hidden="true">{@render overlay()}</span>
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

  /* Не редактируется: вместо настоящего текста показано представление поверх. */
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
    /* Та же область, что у текста поля (отступы 20 и 6 px), текст по центру: строка не скачет
       при переходе между настоящим текстом и представлением. */
    top: 20px;
    bottom: 6px;
    align-items: center;
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
