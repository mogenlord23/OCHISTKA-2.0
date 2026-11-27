<script lang="ts">
  import { onMount } from 'svelte';

  let visible = $state(true);
  onMount(() => {
    const sync = () => { visible = !document.hidden; };
    sync();
    document.addEventListener('visibilitychange', sync);
    return () => document.removeEventListener('visibilitychange', sync);
  });
</script>

<svg class="connect-pulses" class:paused={!visible} aria-hidden="true">
  <rect pathLength="100" rx="12" />
  <rect class="second" pathLength="100" rx="12" />
</svg>

<style>
  svg {
    position: absolute;
    inset: 3px;
    width: calc(100% - 6px);
    height: calc(100% - 6px);
    overflow: visible;
    pointer-events: none;
  }
  rect {
    x: 1px;
    y: 1px;
    width: calc(100% - 2px);
    height: calc(100% - 2px);
    fill: none;
    stroke: #ff009d;
    stroke-width: 1.5px;
    stroke-linecap: round;
    stroke-dasharray: 17 83;
    opacity: .48;
    animation: follow 4s linear infinite;
  }
  rect.second { stroke: #caff00; stroke-dashoffset: -50; animation-name: follow-second; }
  @keyframes follow { to { stroke-dashoffset: -100; } }
  @keyframes follow-second { from { stroke-dashoffset: -50; } to { stroke-dashoffset: -150; } }
  .paused rect { animation-play-state: paused; }
  @media (prefers-reduced-motion: reduce) { rect { animation: none; } }
  :global(:root[data-reduce-motion='true']) rect { animation: none; }
</style>
