<script lang="ts">
  import SectionIcon from './SectionIcon.svelte';
  import { SECTIONS, type SectionId } from '../lib/sections';
  import { t } from '../lib/i18n.svelte';
  import { dragSelect } from '../lib/dragSelect';
  import { playActionSound } from '../lib/sound';

  let {
    active,
    onSelect,
  }: {
    active: SectionId;
    onSelect: (id: SectionId) => void;
  } = $props();

  let rail = $state<HTMLElement>();

  const tiles = (): HTMLElement[] =>
    rail ? Array.from(rail.querySelectorAll<HTMLElement>('.item')) : [];

  function select(index: number, source: 'press' | 'drag' | 'key') {
    const section = SECTIONS[index];
    if (!section) return;
    // Нажатие и клавиатуру озвучивает общий слушатель окна; переключение перетаскиванием играет здесь.
    if (source === 'drag') playActionSound('switch');
    onSelect(section.id);
  }

</script>

<nav
  class="rail"
  aria-label={t('navLabel')}
  bind:this={rail}
  use:dragSelect={{ axis: 'y', items: tiles, onSelect: select }}
>
  {#each SECTIONS as section (section.id)}
    <button
      type="button"
      class="item"
      class:active={active === section.id}
      aria-label={t(section.titleKey)}
      aria-current={active === section.id ? 'page' : undefined}
    >
      <SectionIcon id={section.id} />
      <span class="label">{t(section.titleKey)}</span>
    </button>
  {/each}
</nav>

<style>
  .rail {
    position: relative;
    touch-action: none;
    --gap: var(--frame-inset, 14px);
    --tile: var(--rail-tile, 88px);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--gap);
    min-height: 0;
    padding: var(--gap);
    background: var(--window);
  }

  .item {
    position: relative;
    z-index: 1;
    display: flex;
    flex: none;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    width: var(--tile);
    height: var(--tile);
    padding: 0;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: #fff;
    font: inherit;
    font-size: var(--text-caption);
    font-weight: 800;
    line-height: 1.1;
    cursor: pointer;
    transition: none;
  }

  .item:hover:not(.active) {
    background: var(--rail-hover);
  }

  .item.active {
    background: var(--accent);
    color: var(--page-text);
  }

  .item :global(svg) {
    flex: none;
    width: calc(var(--tile) * 0.36);
    height: calc(var(--tile) * 0.36);
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    shape-rendering: geometricPrecision;
    transition: transform 150ms ease;
  }

  .item:hover :global(svg) {
    transform: scale(1.08);
  }

  .item :global(svg .knob) {
    fill: var(--window);
  }

  .item.active :global(svg .knob) {
    fill: var(--accent);
  }

  .label {
    box-sizing: border-box;
    max-width: 100%;
    padding: 0 4px;
    overflow: hidden;
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0;
    text-align: center;
    text-overflow: clip;
    white-space: nowrap;
  }
</style>
