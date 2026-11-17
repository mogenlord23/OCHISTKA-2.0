<script lang="ts">
  import type { Subsection } from '../lib/sections';

  let {
    items,
    active,
    onSelect,
  }: {
    items: readonly Subsection[];
    active: string;
    onSelect: (id: string) => void;
  } = $props();
</script>

<nav class="subtabs" aria-label="Подразделы">
  {#each items as item (item.id)}
    <button
      type="button"
      class:active={active === item.id}
      aria-current={active === item.id ? 'page' : undefined}
      onclick={() => onSelect(item.id)}
    >
      {item.title}
    </button>
  {/each}
</nav>

<style>
  .subtabs {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 16px 0 20px;
  }

  button {
    min-height: 44px;
    padding: 10px 16px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    font: inherit;
    font-weight: 800;
    text-transform: uppercase;
    font-size: var(--text-body);
    cursor: pointer;
  }

  button.active {
    background: var(--ink);
    color: var(--black);
  }
</style>
