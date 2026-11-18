<script lang="ts" generics="T">
  import type { Snippet } from 'svelte';

  /**
   * Длинный список строк одинаковой высоты: в разметке только видимые строки и небольшой запас.
   * Окно строк сдвигается через `transform` (без пересчёта раскладки), прокрутка и изменение
   * размера читаются раз в кадр. Высота области задаётся CSS (`--vl-max`), JS её не считает.
   */
  let {
    items,
    rowHeight,
    itemKey,
    row,
    head,
    label,
    overscan = 6,
    maxHeight = 'calc(100vh - 468px)',
  }: {
    items: T[];
    rowHeight: number;
    itemKey: (item: T) => string | number;
    row: Snippet<[T, number]>;
    head?: Snippet;
    label: string;
    overscan?: number;
    maxHeight?: string;
  } = $props();

  /** Высота области до первого замера (и в окружении без раскладки, например в тестах). */
  const FALLBACK_HEIGHT = 640;
  const PAD = 4;

  let viewport = $state<HTMLDivElement | null>(null);
  let headBox = $state<HTMLDivElement | null>(null);
  let scrollTop = $state(0);
  let viewHeight = $state(0);
  let headHeight = $state(0);
  let frame = 0;

  function measure() {
    frame = 0;
    if (!viewport) return;
    scrollTop = viewport.scrollTop;
    viewHeight = viewport.clientHeight;
    headHeight = headBox?.offsetHeight ?? 0;
  }

  function schedule() {
    if (frame) return;
    frame = requestAnimationFrame(measure);
  }

  $effect(() => {
    if (!viewport) return;
    measure();
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(schedule);
    observer.observe(viewport);
    return () => {
      observer.disconnect();
      if (frame) cancelAnimationFrame(frame);
      frame = 0;
    };
  });

  const total = $derived(items.length * rowHeight);
  const range = $derived.by(() => {
    const height = (viewHeight || FALLBACK_HEIGHT) - headHeight;
    const from = Math.max(0, Math.floor((scrollTop - PAD) / rowHeight) - overscan);
    const to = Math.min(items.length, Math.ceil((scrollTop + height) / rowHeight) + overscan);
    return { from, to: Math.max(from, to) };
  });
  const slice = $derived(items.slice(range.from, range.to));
</script>

<div
  class="vl"
  role="table"
  aria-label={label}
  aria-rowcount={items.length + 1}
  style:--vl-max={maxHeight}
  bind:this={viewport}
  onscroll={schedule}
>
  {#if head}
    <div class="vl-head" bind:this={headBox}>{@render head()}</div>
  {/if}
  <div class="vl-body" role="rowgroup" style:height="{total + PAD * 2}px">
    <div class="vl-window" style:transform="translateY({PAD + range.from * rowHeight}px)">
      {#each slice as item, offset (itemKey(item))}
        {@render row(item, range.from + offset)}
      {/each}
    </div>
  </div>
</div>

<style>
  .vl {
    max-height: var(--vl-max);
    min-height: 180px;
    overflow: hidden auto;
    overscroll-behavior: contain;
    border-radius: 16px;
    background: #111;
    color: var(--ink);
    font-size: var(--text-small);
  }
  .vl-head {
    position: sticky;
    top: 0;
    z-index: 1;
  }
  .vl-body {
    position: relative;
    padding-inline: 6px;
    box-sizing: border-box;
  }
  .vl-window {
    position: absolute;
    inset: 0 6px auto;
    will-change: transform;
  }
</style>
