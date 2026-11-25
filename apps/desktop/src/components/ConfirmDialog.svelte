<script lang="ts">
  import { onMount } from 'svelte';

  let {
    title,
    text,
    confirmLabel = 'Применить',
    onconfirm,
    oncancel,
  }: {
    title: string;
    text: string;
    confirmLabel?: string;
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();

  let cancelButton = $state<HTMLButtonElement | null>(null);
  let confirmButton = $state<HTMLButtonElement | null>(null);

  onMount(() => cancelButton?.focus());

  function onKey(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      oncancel();
    } else if (event.key === 'Tab') {
      // Фокус остаётся внутри окна.
      const first = cancelButton;
      const last = confirmButton;
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    }
  }
</script>

<div class="backdrop">
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="confirm-title"
    aria-describedby="confirm-text"
    tabindex="-1"
    onkeydown={onKey}
  >
    <h3 id="confirm-title">{title}</h3>
    <p id="confirm-text">{text}</p>
    <div class="actions">
      <button type="button" bind:this={cancelButton} onclick={oncancel}>Отмена</button>
      <button type="button" class="primary" bind:this={confirmButton} onclick={onconfirm}
        >{confirmLabel}</button
      >
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.6);
  }

  .dialog {
    width: min(460px, calc(100vw - 32px));
    padding: 20px;
    border-radius: var(--radius-adaptive);
    background: var(--surface, #111);
    color: var(--ink, #fafafa);
    animation: dialog-in 160ms cubic-bezier(0.2, 0, 0, 1);
  }

  h3 {
    margin: 0 0 8px;
  }

  p {
    margin: 0;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }

  button {
    min-height: 44px;
    padding: 8px 18px;
    border: 1px solid #444;
    border-radius: var(--radius-adaptive);
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }

  button.primary {
    border-color: var(--btn-border, #cf00ff);
    background: var(--btn-bg, #1a1a1a);
  }

  @keyframes dialog-in {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .dialog {
      animation: none;
    }
  }
</style>
