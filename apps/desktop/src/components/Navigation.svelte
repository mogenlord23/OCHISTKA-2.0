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
  const activeIndex = $derived(SECTIONS.findIndex((section) => section.id === active));

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
  style:--active-index={activeIndex}
  bind:this={rail}
  use:dragSelect={{ axis: 'y', items: tiles, onSelect: select }}
>
  <span class="indicator" aria-hidden="true"></span>
  {#each SECTIONS as section (section.id)}
    <button
      type="button"
      class="item"
      class:active={active === section.id}
      aria-label={t(section.titleKey)}
      aria-current={active === section.id ? 'page' : undefined}
    >
      <SectionIcon id={section.id} />
      <span class="label" aria-hidden="true">{t(section.titleKey)}</span>
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
    padding: var(--gap) var(--gap) 0;
    background: var(--window);
  }

  .indicator {
    --corner-size: var(--tile);
    position: absolute;
    top: var(--gap);
    left: var(--gap);
    width: var(--tile);
    height: var(--tile);
    border-radius: var(--radius-adaptive);
    background: var(--accent);
    transform: translateY(calc(var(--active-index) * (var(--tile) + var(--gap))));
    transition:
      transform var(--selection-duration, 190ms) var(--selection-ease, ease-out),
      background-color var(--selection-duration, 190ms) ease-out;
    pointer-events: none;
  }

  .item {
    --corner-size: var(--tile);
    position: relative;
    z-index: 1;
    display: grid;
    flex: none;
    place-items: center;
    width: var(--tile);
    height: var(--tile);
    padding: 0;
    border: 0;
    border-radius: var(--radius-adaptive);
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
    color: var(--page-text);
  }

  .item :global(svg) {
    width: var(--section-icon-size);
    height: var(--section-icon-size);
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    shape-rendering: geometricPrecision;
    transition: transform 180ms var(--selection-ease, ease-out);
  }

  .item:is(:hover, :focus-visible) :global(svg) {
    transform: translateY(-9px) scale(0.76);
  }

  .item :global(svg .knob) {
    fill: var(--window);
  }

  .item.active :global(svg .knob) {
    fill: var(--accent);
  }

  .label {
    position: absolute;
    inset: auto 0 18px;
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
    opacity: 0;
    transform: translateY(-10px);
    transition:
      opacity 160ms ease,
      transform 180ms var(--selection-ease, ease-out);
    pointer-events: none;
  }

  .item:is(:hover, :focus-visible) .label {
    opacity: 1;
    transform: none;
  }

  @media (hover: none) {
    .item :global(svg) {
      transform: translateY(-9px) scale(0.76);
    }
    .label {
      opacity: 1;
      transform: none;
    }
  }
</style>
