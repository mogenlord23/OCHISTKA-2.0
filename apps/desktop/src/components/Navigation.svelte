<script lang="ts">
  import SectionIcon from './SectionIcon.svelte';
  import { SECTIONS, type SectionId } from '../lib/sections';
  import { onMount } from 'svelte';
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
  let indicator = $state<HTMLElement>();
  let placed = false;

  const tiles = (): HTMLElement[] =>
    rail ? Array.from(rail.querySelectorAll<HTMLElement>('.item')) : [];

  function select(index: number, source: 'press' | 'drag' | 'key') {
    const section = SECTIONS[index];
    if (!section) return;
    // Нажатие и клавиатуру озвучивает общий слушатель окна; переключение перетаскиванием играет здесь.
    if (source === 'drag') playActionSound('switch');
    onSelect(section.id);
  }

  /** Ползунок: блок 88x88 цвета раздела, ездит transform между плитками; первая установка без анимации. */
  function place(jump: boolean) {
    if (!indicator) return;
    const current = tiles().find((tile) => tile.getAttribute('aria-current') === 'page');
    if (!current) return;
    if (jump) indicator.style.transition = 'none';
    indicator.style.transform = `translate(${current.offsetLeft}px, ${current.offsetTop}px)`;
    if (jump) {
      void indicator.offsetWidth;
      indicator.style.transition = '';
    }
  }

  $effect(() => {
    void active;
    place(!placed);
    placed = true;
  });

  onMount(() => {
    const observer =
      typeof ResizeObserver === 'function' ? new ResizeObserver(() => place(true)) : null;
    if (rail) observer?.observe(rail);
    return () => observer?.disconnect();
  });
</script>

<nav
  class="rail"
  aria-label={t('navLabel')}
  bind:this={rail}
  use:dragSelect={{ axis: 'y', items: tiles, onSelect: select }}
>
  <span class="ind" bind:this={indicator} aria-hidden="true"></span>
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
  /* Пункт — квадрат 88 px, размер фиксированный: при уменьшении окна не сжимается.
     Верх первого квадрата совпадает с верхней строкой окна.
     Шрифт — системный Segoe UI (не вшит): на Windows вес 800 может выбрать «Segoe UI Black»,
     он заметно шире. Поэтому подпись набирается жирным (700), без межбуквенного интервала, а
     самая длинная («Оформление») занимает не больше ~76% плитки даже в Black.
     Минимальная высота окна (tauri.conf.json) рассчитана так, чтобы все 7 пунктов были видны. */
  .rail {
    position: relative;
    touch-action: none;
    --gap: 8px;
    --tile: 88px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--gap);
    min-height: 0;
    padding: var(--frame-inset-block, 14px) 0 0;
    background: var(--window);
  }

  /* Ползунок выбранного раздела: тот же квадрат 88 px, что и плитка, цвет раздела. */
  .ind {
    position: absolute;
    top: 0;
    left: 0;
    z-index: 0;
    width: var(--tile);
    height: var(--tile);
    border-radius: var(--radius-md);
    background: var(--accent);
    transition:
      transform 190ms cubic-bezier(0.2, 0, 0, 1),
      background-color var(--stage-ms) cubic-bezier(0, 0, 0.2, 1);
    will-change: transform;
    pointer-events: none;
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
  }

  .item:hover:not(.active) {
    background: var(--rail-hover);
  }

  /* Фон выбранной плитки рисует ползунок; сама плитка прозрачная, текст цвета textOn(акцент). */
  .item.active {
    background: transparent;
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
