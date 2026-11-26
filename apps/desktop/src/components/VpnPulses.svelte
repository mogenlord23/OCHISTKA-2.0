<script lang="ts">
  import { onMount } from 'svelte';

  let layer: SVGSVGElement;
  let origin = $state({ x: 0, y: 0, side: 0, reach: 0, width: 0, height: 0 });
  const travel = $derived(Math.max(1, origin.reach - origin.side / 2));
  // Постоянные 60 CSS-пикселей в секунду при любой длине карточки.
  const period = $derived(travel / 60);
  const rings = 3;

  onMount(() => {
    const syncMotion = () => {
      if (document.visibilityState === 'hidden' || !document.hasFocus()) layer.pauseAnimations?.();
      else layer.unpauseAnimations?.();
    };
    document.addEventListener('visibilitychange', syncMotion);
    window.addEventListener('blur', syncMotion);
    window.addEventListener('focus', syncMotion);
    syncMotion();
    const button = layer.parentElement?.querySelector<HTMLButtonElement>('.power');
    if (!button || typeof ResizeObserver !== 'function') return () => {
      document.removeEventListener('visibilitychange', syncMotion);
      window.removeEventListener('blur', syncMotion);
      window.removeEventListener('focus', syncMotion);
    };
    function measure() {
      if (!button) return;
      const bounds = layer.getBoundingClientRect();
      const tile = button.getBoundingClientRect();
      const x = tile.left - bounds.left + tile.width / 2;
      const y = tile.top - bounds.top + tile.height / 2;
      origin = { x, y, side: tile.width, reach: Math.max(x, bounds.width - x, y, bounds.height - y) + 3, width: bounds.width, height: bounds.height };
    }
    const observer = new ResizeObserver(measure);
    observer.observe(layer);
    observer.observe(button);
    return () => {
      observer.disconnect();
      document.removeEventListener('visibilitychange', syncMotion);
      window.removeEventListener('blur', syncMotion);
      window.removeEventListener('focus', syncMotion);
    };
  });
</script>

<svg class="connect-pulses" bind:this={layer} aria-hidden="true"
  style:--period={period + 's'}>
  {#if origin.side > 0}
    {#each Array(rings) as _, index (index)}
      <rect x={origin.x - origin.side / 2} y={origin.y - origin.side / 2}
        width={origin.side} height={origin.side} rx={12}
        vector-effect="non-scaling-stroke" style:--delay={(-period * index / rings) + 's'}>
        <animate attributeName="x" from={origin.x - origin.side / 2} to="3"
          dur={period + 's'} begin={(-period * index / rings) + 's'} repeatCount="indefinite" />
        <animate attributeName="y" from={origin.y - origin.side / 2} to="3"
          dur={period + 's'} begin={(-period * index / rings) + 's'} repeatCount="indefinite" />
        <animate attributeName="width" from={origin.side} to={Math.max(1, origin.width - 6)}
          dur={period + 's'} begin={(-period * index / rings) + 's'} repeatCount="indefinite" />
        <animate attributeName="height" from={origin.side} to={Math.max(1, origin.height - 6)}
          dur={period + 's'} begin={(-period * index / rings) + 's'} repeatCount="indefinite" />
      </rect>
    {/each}
  {/if}
</svg>

<style>
  svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    overflow: hidden;
    border-radius: inherit;
  }
  rect {
    fill: none;
    stroke: var(--vpn, var(--accent));
    stroke-width: 1;
    animation: fade var(--period) ease-in-out var(--delay) infinite;
  }
  @keyframes fade {
    0%, 100% { opacity: 0; }
    15% { opacity: .32; }
    70% { opacity: .15; }
  }
  @media (prefers-reduced-motion: reduce) {
    svg { display: none; }
  }
  :global(:root[data-reduce-motion='true']) svg { display: none; }
</style>
