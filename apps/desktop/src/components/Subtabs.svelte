<script lang="ts">
  import { onMount } from 'svelte';
  import type { Subsection } from '../lib/sections';
  import { i18n, t } from '../lib/i18n.svelte';
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

  let row = $state<HTMLElement>();
  let track = $state<HTMLElement>();
  let indicator = $state<HTMLElement>();
  let lastItems: readonly Subsection[] | null = null;

  const tabs = (): HTMLElement[] =>
    track ? Array.from(track.querySelectorAll<HTMLElement>('[role="tab"]')) : [];

  function select(index: number, source: 'press' | 'drag' | 'key') {
    const item = items[index];
    if (!item) return;
    // Нажатие и клавиатуру озвучивает общий слушатель окна; переключение перетаскиванием играет здесь.
    if (source === 'drag') playActionSound('switch');
    onSelect(item.id);
  }

  /** Ползунок ездит transform и шириной; при смене раздела встаёт на место без анимации. */
  function place(jump: boolean) {
    if (!indicator) return;
    const current = tabs().find((b) => b.getAttribute('aria-selected') === 'true');
    if (!current) return;
    if (jump) indicator.style.transition = 'none';
    indicator.style.width = `${current.offsetWidth}px`;
    indicator.style.transform = `translateX(${current.offsetLeft}px)`;
    if (jump) {
      void indicator.offsetWidth;
      indicator.style.transition = '';
    }
  }

  $effect(() => {
    void active;
    void i18n.lang;
    const jump = items !== lastItems;
    lastItems = items;
    place(jump);
  });

  onMount(() => {
    const observer =
      typeof ResizeObserver === 'function' ? new ResizeObserver(() => place(true)) : null;
    if (row) observer?.observe(row);
    void document.fonts?.ready.then(() => place(true));
    return () => observer?.disconnect();
  });

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

<div class="subtabs" bind:this={row}>
  <div
    class="track"
    role="tablist"
    tabindex="-1"
    aria-label={t('subnavLabel')}
    bind:this={track}
    use:dragSelect={{ axis: 'x', items: tabs, onSelect: select }}
    onkeydown={onKeydown}
  >
    <span class="ind" bind:this={indicator} aria-hidden="true"></span>
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
  /* Строка вкладок прямо под шапкой: тонкая чёрная дорожка и ползунок цвета раздела. */
  .subtabs {
    display: flex;
    flex: none;
    align-items: center;
    min-width: 0;
    min-height: 48px;
    margin: 6px 0 12px;
  }

  .track {
    position: relative;
    display: inline-grid;
    grid-auto-flow: column;
    grid-auto-columns: max-content;
    box-sizing: border-box;
    overflow-x: auto;
    overflow-y: hidden;
    max-width: 100%;
    gap: 2px;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--black);
    touch-action: none;
    cursor: pointer;
  }

  .ind {
    position: absolute;
    top: 4px;
    left: 0;
    z-index: 0;
    width: 40px;
    height: 38px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    box-shadow: inset 0 0 0 2px rgba(8, 8, 8, 0.72);
    transform: translateX(4px);
    transition:
      transform 190ms cubic-bezier(0.2, 0, 0, 1),
      width 190ms cubic-bezier(0.2, 0, 0, 1),
      background-color var(--stage-ms) cubic-bezier(0, 0, 0.2, 1);
    will-change: transform, width;
    pointer-events: none;
  }

  button {
    position: relative;
    z-index: 1;
    min-width: 0;
    height: 38px;
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
  }

  button:hover:not([aria-selected='true']) {
    background: var(--rail-hover);
  }

  /* Выбранная вкладка: фон даёт ползунок, текст цвета textOn(акцент). */
  button[aria-selected='true'] {
    background: transparent;
    color: var(--page-text);
  }
</style>
