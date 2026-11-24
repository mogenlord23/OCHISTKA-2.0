<script lang="ts">
  import { onMount, untrack } from 'svelte';

  let { level, running, clipId }: { level: number; running: boolean; clipId: string } = $props();
  let fill = $state(0);
  let phase = $state(0);
  let reduced = $state(false);
  const gradient = $derived(`${clipId}-fluid`);

  onMount(() => {
    const query = window.matchMedia?.('(prefers-reduced-motion: reduce)');
    const read = () => {
      reduced = !!query?.matches || document.documentElement.dataset.reduceMotion === 'true';
    };
    const observer = new MutationObserver(read);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-reduce-motion'] });
    query?.addEventListener?.('change', read);
    read();
    return () => {
      observer.disconnect();
      query?.removeEventListener?.('change', read);
    };
  });

  $effect(() => {
    const target = level;
    const moving = running;
    if (reduced || typeof requestAnimationFrame !== 'function') {
      fill = target;
      return;
    }
    const initial = untrack(() => fill);
    const start = performance.now();
    let frame: number;
    const step = (time: number) => {
      const fraction = Math.min(1, Math.max(0, (time - start) / 620));
      fill = initial + (target - initial) * (1 - (1 - fraction) ** 3);
      if (moving) phase = time * 0.00065;
      if (moving || fraction < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  });

  // The visible liquid and the ink over the lettering use exactly the same geometry.
  const water = $derived.by(() => {
    const amplitude = reduced ? 0 : Math.min(0.012, fill, 1 - fill);
    const angle = (x: number) => x * 2 * Math.PI - phase;
    const height = (x: number) => 1 - fill + amplitude * Math.sin(angle(x));
    const slope = (x: number) => amplitude * 2 * Math.PI * Math.cos(angle(x));
    // Match both the height and slope where the four curves meet.
    const step = 0.25;
    const handle = step / 3;
    let path = "M0 " + height(0).toFixed(5);
    for (let start = 0; start < 1; start += step) {
      const end = start + step;
      path += " C" + [
        start + handle, height(start) + handle * slope(start),
        end - handle, height(end) - handle * slope(end),
        end, height(end),
      ].map((value) => value.toFixed(5)).join(" ");
    }
    return path + " V1 H0 Z";
  });
</script>

<span class="tank" data-testid="scan-liquid" data-level={Math.round(level * 100)} aria-hidden="true">
  <svg class="liquid" viewBox="0 0 1 1" preserveAspectRatio="none">
    <defs>
      <clipPath id={clipId} clipPathUnits="objectBoundingBox"><path d={water} /></clipPath>
      <linearGradient id={gradient} x2="12%" y2="100%">
        <stop offset="0.06" stop-color="#e3ff72" />
        <stop offset="0.55" stop-color="#c5ed39" />
        <stop offset="1" stop-color="#77a311" />
      </linearGradient>
    </defs>
    <path d={water} fill={`url(#${gradient})`} />
  </svg>
  {#if running && !reduced}
    <span class="reflections" data-testid="scan-reflections" style:clip-path={"url(#" + clipId + ")"}>
      <span></span>
      <span></span>
      <span></span>
    </span>
  {/if}
</span>

<style>
  .tank {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .liquid {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .reflections {
    position: absolute;
    inset: 0;
  }

  .reflections span {
    position: absolute;
    top: 84%;
    left: 17%;
    width: 14%;
    height: 2px;
    border-radius: var(--radius-pill);
    background: linear-gradient(90deg, transparent, #f6ffce, transparent);
    animation: drift 8s linear -2s infinite;
  }

  .reflections span:nth-child(2) { top: 72%; left: 54%; animation-delay: -5s; }
  .reflections span:nth-child(3) { top: 94%; left: 35%; animation-delay: -7s; }

  @keyframes drift {
    from { transform: translateX(-70%); opacity: 0; }
    32%, 65% { opacity: 0.38; }
    to { transform: translateX(95%); opacity: 0; }
  }

  @media (prefers-reduced-motion: reduce) {
    .reflections { display: none; }
  }

  :global(:root[data-reduce-motion='true']) .reflections { display: none; }
</style>
