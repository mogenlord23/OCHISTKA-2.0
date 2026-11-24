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
      phase = time * 0.0022;
      if (moving || fraction < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  });

  // The visible liquid and the ink over the lettering use exactly the same geometry.
  const water = $derived.by(() => {
    const amplitude = Math.min(0.032, fill, 1 - fill);
    const line = (x: number) => (1 - fill + Math.sin(x * Math.PI * 4 - phase) * amplitude).toFixed(4);
    return `M0 ${line(0)} Q.125 ${line(.125)} .25 ${line(.25)} T.5 ${line(.5)} T.75 ${line(.75)} T1 ${line(1)} V1 H0 Z`;
  });

  const particles = [
    { x: 10, size: 24, duration: 2.2, delay: -0.7 },
    { x: 24, size: 38, duration: 2.9, delay: -2.2 },
    { x: 32, size: 13, duration: 1.7, delay: -0.6 },
    { x: 40, size: 20, duration: 1.9, delay: -1.5 },
    { x: 56, size: 29, duration: 2.5, delay: -0.3 },
    { x: 73, size: 35, duration: 3.1, delay: -2.6 },
    { x: 81, size: 15, duration: 2.4, delay: -1.1 },
    { x: 89, size: 22, duration: 1.8, delay: -0.9 },
  ];
</script>

<span class="tank" class:running data-testid="scan-liquid" data-level={Math.round(level * 100)}
  style:--level={`${fill * 100}%`} aria-hidden="true">
  {#if running}
    <span class="rain" data-testid="falling-viruses">
      {#each particles as particle (particle.x)}
        <span class="drop" style:left={`${particle.x}%`} style:--size={`${particle.size}px`}
          style:--duration={`${particle.duration}s`} style:--delay={`${particle.delay}s`}>
          <svg viewBox="0 0 32 32" fill="none">
            <path d="M16 3v6m0 14v6M3 16h6m14 0h6M7 7l4 4m10 10 4 4m0-18-4 4m-10 10-4 4"
              stroke="currentColor" stroke-width="2" stroke-linecap="round" />
            <circle cx="16" cy="16" r="8" fill="#77a713" stroke="currentColor" stroke-width="2" />
            <circle cx="14" cy="13" r="2.2" fill="currentColor" />
            <circle cx="19" cy="17" r="1.6" fill="currentColor" />
            <circle cx="13" cy="20" r="1" fill="currentColor" />
          </svg>
        </span>
      {/each}
    </span>
  {/if}
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
  <span class="acid" class:empty={fill === 0}>
    {#if running}
      <span class="bubble bubble-left"></span>
      <span class="bubble bubble-center"></span>
      <span class="bubble bubble-right"></span>
      <span class="splash splash-left"></span>
      <span class="splash splash-right"></span>
    {/if}
  </span>
</span>

<style>
  .tank {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .rain {
    position: absolute;
    inset: 0 0 var(--level);
    overflow: hidden;
    animation: reveal 760ms ease both;
  }

  .drop {
    position: absolute;
    top: 0;
    width: var(--size);
    height: 100%;
    color: #d9ff66;
    animation: sink var(--duration) linear var(--delay) infinite;
  }

  .drop:nth-child(2n) { color: #bdf04a; }

  .drop svg {
    display: block;
    width: 100%;
    animation: turn var(--duration) linear infinite;
  }

  .acid {
    position: absolute;
    inset: auto 0 0;
    height: var(--level);
  }

  .acid.empty { visibility: hidden; }

  .liquid {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .bubble {
    position: absolute;
    bottom: 8%;
    width: var(--size);
    height: var(--size);
    border: 1px solid #465f0d;
    border-radius: 50%;
    background: #e2ff6c55;
    animation: rise 2s ease-in var(--delay) infinite;
  }

  .bubble-left { left: 18%; --size: 12px; --delay: -0.6s; }
  .bubble-center { left: 43%; --size: 8px; --delay: -1.4s; }
  .bubble-right { left: 80%; --size: 17px; --delay: 0s; }

  .splash {
    position: absolute;
    top: 1px;
    width: 19px;
    height: 6px;
    border: 1px solid #efff9f;
    border-radius: 50%;
    animation: ripple 2.1s ease-out var(--delay) infinite;
  }

  .splash-left { left: 25%; --delay: -0.8s; }
  .splash-right { left: 72%; --delay: -1.6s; }

  @keyframes sink {
    0% { transform: translate(-50%, -15%); opacity: 0; }
    24%, 68% { opacity: 0.9; }
    100% { transform: translate(-50%, 100%); opacity: 0; }
  }

  @keyframes reveal { from { opacity: 0; } to { opacity: 1; } }

  @keyframes turn { to { transform: rotate(140deg); } }

  @keyframes ripple {
    0% { transform: scale(0.35); opacity: 0.9; }
    48%, 100% { transform: scale(2.4); opacity: 0; }
  }

  @keyframes rise {
    from { bottom: 8%; opacity: 0; }
    30% { opacity: 0.8; }
    to { bottom: 96%; opacity: 0; }
  }

  @media (prefers-reduced-motion: reduce) {
    .rain, .bubble, .splash { display: none; }
  }

  :global(:root[data-reduce-motion='true']) .rain,
  :global(:root[data-reduce-motion='true']) .bubble,
  :global(:root[data-reduce-motion='true']) .splash { display: none; }
</style>
