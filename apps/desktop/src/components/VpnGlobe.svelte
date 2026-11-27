<script lang="ts">
  import type { mountVpnGlobe } from '../lib/vpnGlobe';

  let { active, visible, country, connected, label }: {
    active: boolean;
    visible: boolean;
    country: string | null;
    connected: boolean;
    label: string;
  } = $props();

  let canvas = $state<HTMLCanvasElement>();
  const activeCanvas = $derived(active ? canvas : undefined);
  let renderer = $state.raw<ReturnType<typeof mountVpnGlobe>>();

  $effect(() => {
    if (!activeCanvas || typeof ResizeObserver !== 'function') return;
    const element = activeCanvas;
    let disposed = false;
    void import('../lib/vpnGlobe').then(({ mountVpnGlobe }) => {
      if (disposed) return;
      renderer = mountVpnGlobe(element, () => ({ country, connected }));
    });
    return () => {
      disposed = true;
      renderer?.destroy();
      renderer = undefined;
    };
  });

  $effect(() => renderer?.setVisible(visible));

  $effect(() => {
    void country;
    void connected;
    renderer?.invalidate();
  });
</script>

<div class="globe" class:paused={!visible} role="img" aria-label={label} data-country={country ?? undefined} data-connected={connected}>
  <canvas bind:this={canvas} aria-hidden="true"></canvas>
</div>

<style>
  .globe {
    position: relative;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    border-radius: var(--radius-adaptive);
    background: var(--surface-alt);
    box-shadow: inset 0 0 0 2px #000;
  }

  canvas,
  .globe :global(.orbit-layer) {
    position: absolute;
    inset: 2px;
    display: block;
    width: calc(100% - 4px);
    height: calc(100% - 4px);
    border-radius: inherit;
  }
  .globe :global(.orbit-layer[hidden]) { display: none; }
  .globe :global(.globe-pulse) {
    position: absolute;
    pointer-events: none;
    border-radius: 25%;
    border: 1px solid currentColor;
    animation: pulse 2s linear infinite;
  }
  .paused :global(.globe-pulse) { animation-play-state: paused; }
  @keyframes pulse {
    from { transform: translate(-50%,-50%) scale(.85); opacity:.65; }
    to { transform: translate(-50%,-50%) scale(2.4); opacity:0; }
  }
</style>
