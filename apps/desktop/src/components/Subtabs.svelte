<script lang="ts">
  import type { Subsection } from '../lib/sections';
  import { t } from '../lib/i18n.svelte';
  import { dragSelect } from '../lib/dragSelect';
  import { playActionSound } from '../lib/sound';

  let {
    items,
    active,
    onSelect,
  }: {
    items: readonly Subsection[];
    active: string;
    onSelect: (id: string) => void;
  } = $props();

  let track = $state<HTMLElement>();

  const tabs = (): HTMLElement[] =>
    track ? Array.from(track.querySelectorAll<HTMLElement>('[role="tab"]')) : [];

  function select(index: number, source: 'press' | 'drag' | 'key') {
    const item = items[index];
    if (!item) return;
    // Нажатие и клавиатуру озвучивает общий слушатель окна; переключение перетаскиванием играет здесь.
    if (source === 'drag') playActionSound('switch');
    onSelect(item.id);
  }

  function onKeydown(event: KeyboardEvent) {
    const n = items.length;
    const index = items.findIndex((item) => item.id === active);
    let next = index;
    if (event.key === 'ArrowRight') next = (index + 1) % n;
    else if (event.key === 'ArrowLeft') next = (index - 1 + n) % n;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = n - 1;
    else return;
    event.preventDefault();
    onSelect(items[next].id);
    tabs()[next]?.focus();
  }
</script>

<div class="subtabs">
  <div
    class="track"
    role="tablist"
    tabindex="-1"
    aria-label={t('subnavLabel')}
    bind:this={track}
    use:dragSelect={{ axis: 'x', items: tabs, onSelect: select }}
    onkeydown={onKeydown}
  >
    {#each items as item (item.id)}
      <button
        type="button"
        role="tab"
        aria-selected={active === item.id}
        tabindex={active === item.id ? 0 : -1}
      >
        {t(item.titleKey)}
      </button>
    {/each}
  </div>
</div>

<style>
  .subtabs {
    --track-padding: var(--subtabs-padding, 4px);
    --tab-height: calc(var(--subtabs-height, 48px) - 2 * (var(--track-padding) + 1px));
    display: flex;
    flex: none;
    align-items: center;
    min-width: 0;
    min-height: var(--subtabs-height, 48px);
    margin: 6px 0 12px;
  }

  .track {
    position: relative;
    display: inline-grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(max-content, 1fr);
    flex: 1;
    min-width: 0;
    box-sizing: border-box;
    overflow-x: auto;
    overflow-y: hidden;
    max-width: 100%;
    gap: 2px;
    padding: var(--track-padding);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--black);
    touch-action: none;
    cursor: pointer;
  }

  button {
    position: relative;
    z-index: 1;
    min-width: 0;
    height: var(--tab-height);
    padding: 0 18px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink);
    font: inherit;
    font-size: var(--text-small);
    font-weight: 800;
    white-space: nowrap;
    cursor: pointer;
    transition: none;
  }

  button:hover:not([aria-selected='true']) {
    background: var(--rail-hover);
  }

  button[aria-selected='true'] {
    background: var(--accent);
    box-shadow: inset 0 0 0 2px rgba(8, 8, 8, 0.72);
    color: var(--page-text);
  }
</style>
