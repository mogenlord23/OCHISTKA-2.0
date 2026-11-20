<script lang="ts">
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

<!-- Значки как в прежнем приложении: контур, закруглённые концы, чёткие линии на сетке 24. -->
{#snippet icon(id: SectionId)}
  <svg viewBox="0 0 24 24" aria-hidden="true">
    {#if id === 'vpn'}
      <circle cx="12" cy="12" r="9" />
      <path d="M3 12h18" />
      <path d="M12 3c-3.2 3-3.2 15 0 18M12 3c3.2 3 3.2 15 0 18" />
    {:else if id === 'viruses'}
      <path d="M12 3 4.5 6v5.5c0 4.6 3 8 7.5 9.5 4.5-1.5 7.5-4.9 7.5-9.5V6z" />
    {:else if id === 'processes'}
      <path d="M6.5 3h11a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z" />
      <path d="M9 8h6M9 12h6M9 16h6" />
    {:else if id === 'programs'}
      <rect x="3.5" y="3.5" width="7" height="7" rx="1.5" />
      <rect x="13.5" y="3.5" width="7" height="7" rx="1.5" />
      <rect x="3.5" y="13.5" width="7" height="7" rx="1.5" />
      <rect x="13.5" y="13.5" width="7" height="7" rx="1.5" />
    {:else if id === 'disks'}
      <path
        d="M5.2 4h13.6a1.5 1.5 0 0 1 1.4 1l2 5.7V18a2 2 0 0 1-2 2H3.8a2 2 0 0 1-2-2v-7.3l2-5.7a1.5 1.5 0 0 1 1.4-1z"
      />
      <path d="M1.8 11h20.4" />
      <path d="M6 15.5h6" />
      <circle cx="17.5" cy="15.5" r="0.6" />
    {:else if id === 'computer'}
      <rect x="2.5" y="3.5" width="19" height="13" rx="2" />
      <path d="M8 20.5h8M12 16.5v4" />
    {:else}
      <path d="M4 7h16M4 12h16M4 17h16" />
      <circle cx="9" cy="7" r="2.2" class="knob" />
      <circle cx="16" cy="12" r="2.2" class="knob" />
      <circle cx="8" cy="17" r="2.2" class="knob" />
    {/if}
  </svg>
{/snippet}

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
      {@render icon(section.id)}
      <span class="label">{t(section.titleKey)}</span>
    </button>
  {/each}
</nav>

<style>
  /* Пункт — квадрат 88 px, размер фиксированный: при уменьшении окна не сжимается.
     Верх первого квадрата на линии верха кнопок окна (Y=12: строка 64 px, кнопки 40 px).
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
    padding: 12px 0 0;
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

  svg {
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

  .item:hover svg {
    transform: scale(1.08);
  }

  svg :global(.knob) {
    fill: var(--window);
  }

  .item.active svg :global(.knob) {
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
