<script lang="ts">
  /** Кнопка «Удалить выбранное» и причина, по которой она сейчас выключена.
      Пока перенос в карантин не доказан, кнопка выключена — но причина написана простыми словами. */
  let {
    disabled,
    reason = '',
    onremove,
  }: {
    disabled: boolean;
    reason?: string;
    onremove?: () => void;
  } = $props();
</script>

<div class="remove">
  <button type="button" {disabled} onclick={() => onremove?.()}>Удалить выбранное</button>
  {#if reason}
    <span class="reason" data-testid="remove-reason">{reason}</span>
  {/if}
</div>

<style>
  .remove {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
  }

  button {
    min-height: 38px;
    padding: 8px 14px;
    border: 0;
    border-radius: var(--radius-adaptive);
    background: var(--btn-bg);
    box-shadow: inset 0 0 0 1px var(--btn-border);
    color: var(--ink);
    font: inherit;
    font-size: var(--text-caption);
    font-weight: 700;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .reason {
    max-width: 320px;
    color: var(--muted);
    font-size: var(--text-small);
    line-height: 1.3;
  }
</style>
