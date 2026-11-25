<script lang="ts">
  import Switch from './Switch.svelte';

  let {
    title,
    description,
    checked,
    disabled = false,
    pending = false,
    failure = '',
    reason = null,
    restart = '',
    onchange,
  }: {
    title: string;
    description: string;
    checked: boolean;
    disabled?: boolean;
    pending?: boolean;
    /** Короткая фраза, если применить не получилось (переключатель уже вернулся назад). */
    failure?: string;
    /** Почему строка недоступна (серая). */
    reason?: string | null;
    /** Когда вступит в силу, если не сразу. */
    restart?: string;
    /** Запрошено новое состояние; переключатель меняет родитель. */
    onchange: (next: boolean) => void;
  } = $props();
</script>

<div class="row" class:disabled>
  <div class="body">
    <span class="title">{title}</span>
    <span class="text">{description}</span>
    <!-- Строка под описанием занимает место всегда: сообщение об ошибке или причина
         появляются без изменения высоты, страница не прыгает при переключении. -->
    {#if failure}
      <span class="meta fail" role="alert">{failure}</span>
    {:else if reason}
      <span class="meta">{reason}</span>
    {:else if restart}
      <span class="meta">Вступит в силу {restart}.</span>
    {:else}
      <span class="meta" aria-hidden="true">&nbsp;</span>
    {/if}
  </div>
  <Switch ariaLabel={title} {checked} {disabled} {pending} error={!!failure} {onchange} />
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 16px;
    padding: 8px 14px;
    border-radius: var(--radius-adaptive);
    background: var(--black, #080808);
    color: var(--ink, #fafafa);
  }

  .row.disabled .body {
    opacity: 0.55;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .title {
    font-size: var(--text-body);
    font-weight: 600;
  }

  .text {
    font-size: var(--text-small);
    color: var(--ink-soft, #d4d4d4);
  }

  .meta {
    min-height: 1.3em;
    font-size: var(--text-caption);
    color: #b8b8b8;
  }

  .meta.fail {
    color: #ff7b72;
  }
</style>
