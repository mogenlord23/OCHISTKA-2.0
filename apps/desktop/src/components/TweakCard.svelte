<script lang="ts">
  import { SAVED_NOTE } from '../lib/tweaksText';

  let {
    title,
    description,
    checked,
    disabled = false,
    note = SAVED_NOTE,
    onchange,
  }: {
    title: string;
    description: string;
    checked: boolean;
    disabled?: boolean;
    note?: string | null;
    /** Запрошено новое состояние; галочка меняется только после ответа бэкенда. */
    onchange: (next: boolean) => void;
  } = $props();
</script>

<label class="card" class:checked class:disabled>
  <span class="head">
    <input
      type="checkbox"
      {checked}
      {disabled}
      onchange={(event) => {
        const box = event.currentTarget;
        // Состояние показывает только бэкенд: возвращаем галочку, пока не подтвердят.
        box.checked = checked;
        onchange(!checked);
      }}
    />
    <span class="title">{title}</span>
  </span>
  <span class="text">{description}</span>
  {#if note}<span class="note">{note}</span>{/if}
</label>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 16px 18px;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
    cursor: pointer;
    transition: border-color 160ms ease;
  }

  .card:hover {
    border-color: #444;
  }

  .card:focus-within {
    outline: 2px solid var(--ink, #fafafa);
    outline-offset: 2px;
  }

  .card.checked {
    border-color: var(--btn-border, #cf00ff);
  }

  .card.disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  input {
    width: 20px;
    height: 20px;
    margin: 0;
    accent-color: var(--btn-border, #cf00ff);
  }

  .title {
    font-size: 18px;
    font-weight: 600;
  }

  .text {
    font-size: var(--text-small);
  }

  .note {
    font-size: var(--text-caption);
    color: #b8b8b8;
  }
</style>
