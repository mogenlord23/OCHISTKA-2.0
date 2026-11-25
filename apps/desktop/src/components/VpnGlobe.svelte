<script lang="ts">
  import type { mountVpnGlobe } from '../lib/vpnGlobe';

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
    const element = canvas;
    let disposed = false;
    let renderer: ReturnType<typeof mountVpnGlobe>;
    void import('../lib/vpnGlobe').then(({ mountVpnGlobe }) => {
      if (disposed) return;
      renderer = mountVpnGlobe(element, () => ({ country, connected }));
      invalidate = renderer?.invalidate;
    });
    return () => {
      disposed = true;
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
    background: var(--surface-alt);
    box-shadow: inset 0 0 0 2px #000;
  }

  canvas {
    position: absolute;
    inset: 2px;
    display: block;
    width: calc(100% - 4px);
    height: calc(100% - 4px);
    border-radius: inherit;
  }
</style>
