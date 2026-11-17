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
    margin: 12px 0 20px;
  }

  button {
    padding: 8px 14px;
    border: 1px solid #343434;
    border-radius: 8px;
    background: #141414;
    color: #d4d4d4;
    font: inherit;
    cursor: pointer;
  }

  button.active {
    border-color: var(--accent);
    background: var(--accent);
    color: #080808;
    font-weight: 700;
  }
</style>
