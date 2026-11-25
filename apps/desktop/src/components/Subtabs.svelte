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
  let indicator = $state<HTMLElement>();
  let positioned = $state(false);
  let frame: number | undefined;

  const tabs = (): HTMLElement[] =>
    track ? Array.from(track.querySelectorAll<HTMLElement>('[role="tab"]')) : [];

  function schedulePosition() {
    if (frame !== undefined) return;
    frame = requestAnimationFrame(() => {
      frame = undefined;
      const selected = track?.querySelector<HTMLElement>('[aria-selected="true"]');
      if (!selected || !indicator) {
        positioned = false;
        return;
      }
      const { offsetLeft, offsetWidth } = selected;
      indicator.style.transform = `translateX(${offsetLeft}px)`;
      indicator.style.width = `${offsetWidth}px`;
      positioned = offsetWidth > 0;
    });
  }

  $effect(() => {
    void active;
    schedulePosition();
  });

  $effect(() => {
    void items;
    if (!track) return;
    const observer = typeof ResizeObserver === 'function' ? new ResizeObserver(schedulePosition) : null;
    observer?.observe(track);
    for (const tab of tabs()) observer?.observe(tab);
    schedulePosition();
    return () => {
      observer?.disconnect();
      if (frame !== undefined) cancelAnimationFrame(frame);
      frame = undefined;
    };
  });

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
    class:positioned
    role="tablist"
    tabindex="-1"
    aria-label={t('subnavLabel')}
    bind:this={track}
    use:dragSelect={{ axis: 'x', items: tabs, onSelect: select }}
    onkeydown={onKeydown}
  >
    <span class="indicator" bind:this={indicator} aria-hidden="true"></span>
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
    --corner-size: var(--subtabs-height, 48px);
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
    border-radius: var(--radius-adaptive);
    background: var(--black);
    touch-action: none;
    cursor: pointer;
  }

  .indicator {
    --corner-size: var(--tab-height);
    position: absolute;
    top: var(--track-padding);
    left: 0;
    height: var(--tab-height);
    border-radius: var(--radius-adaptive);
    background: var(--accent);
    box-shadow: inset 0 0 0 2px rgba(8, 8, 8, 0.72);
    visibility: hidden;
    pointer-events: none;
    contain: strict;
  }

  .positioned .indicator {
    visibility: visible;
    transition:
      transform var(--selection-duration, 190ms) var(--selection-ease, ease-out),
      width var(--selection-duration, 190ms) var(--selection-ease, ease-out);
  }

  button {
    --corner-size: var(--tab-height);
    position: relative;
    z-index: 1;
    min-width: 0;
    height: var(--tab-height);
    padding: 0 18px;
    border: 0;
    border-radius: var(--radius-adaptive);
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
    color: var(--page-text);
  }

  .track:not(.positioned) button[aria-selected='true'] {
    background: var(--accent);
    box-shadow: inset 0 0 0 2px rgba(8, 8, 8, 0.72);
  }
</style>
