<script lang="ts">
  import { mountVpnGlobe } from '../lib/vpnGlobe';

  let { active, country, connected, label }: {
    active: boolean;
    country: string | null;
    connected: boolean;
    label: string;
  } = $props();

  let canvas = $state<HTMLCanvasElement>();
  let invalidate: (() => void) | undefined;

  $effect(() => {
    if (!active || !canvas || typeof ResizeObserver !== 'function') return;
    const renderer = mountVpnGlobe(canvas, () => ({ country, connected }));
    invalidate = renderer?.invalidate;
    return () => {
      invalidate = undefined;
      renderer?.destroy();
    };
  });

  $effect(() => {
    void country;
    void connected;
    invalidate?.();
  });
</script>

<div class="globe" role="img" aria-label={label} data-country={country ?? undefined} data-connected={connected}>
  <canvas bind:this={canvas} aria-hidden="true"></canvas>
</div>

<style>
  .globe {
    position: relative;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    border-radius: var(--radius-adaptive);
    background: #000;
    box-shadow: inset 0 0 0 2px #000;
  }

  canvas {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
