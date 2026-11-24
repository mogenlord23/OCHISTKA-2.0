<script lang="ts">
  let { level, running }: { level: number; running: boolean } = $props();

  const particles = [
    { x: 10, size: 24, duration: 2.2, delay: -0.7 },
    { x: 24, size: 38, duration: 2.9, delay: -2.2 },
    { x: 40, size: 20, duration: 1.9, delay: -1.5 },
    { x: 56, size: 29, duration: 2.5, delay: -0.3 },
    { x: 73, size: 35, duration: 3.1, delay: -2.6 },
    { x: 89, size: 22, duration: 1.8, delay: -0.9 },
  ];
</script>

<span class="tank" data-testid="scan-liquid" data-level={Math.round(level * 100)}
  style:--level={`${level * 100}%`} aria-hidden="true">
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
  <span class="acid" class:empty={level === 0}>
    <svg class="surface" viewBox="0 0 240 18" preserveAspectRatio="none">
      <path d="M0 10Q20 0 40 9T80 8T120 9T160 8T200 9T240 8V18H0Z" fill="#bdea28" />
      <path d="M0 13Q20 6 40 12T80 11T120 12T160 11T200 12T240 11V18H0Z" fill="#d1f743" />
    </svg>
    {#if running}
      <span class="bubble bubble-left"></span>
      <span class="bubble bubble-center"></span>
      <span class="bubble bubble-right"></span>
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
    transition: bottom 380ms ease-out;
  }

  .drop {
    position: absolute;
    top: 0;
    width: var(--size);
    height: 100%;
    color: #d9ff66;
    animation: sink var(--duration) linear var(--delay) infinite;
  }

  .drop svg {
    display: block;
    width: 100%;
    animation: turn var(--duration) linear infinite;
  }

  .acid {
    position: absolute;
    inset: auto 0 0;
    height: var(--level);
    background: linear-gradient(174deg, #d1f743, #a7d31b 50%, #78a510);
    transition: height 380ms ease-out;
  }

  .acid.empty { visibility: hidden; }

  .surface {
    position: absolute;
    inset: auto 0 calc(100% - 1px);
    width: 100%;
    height: 14px;
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

  @keyframes sink {
    from { transform: translate(-50%, -20%); }
    to { transform: translate(-50%, 100%); }
  }

  @keyframes turn { to { transform: rotate(140deg); } }

  @keyframes rise {
    from { bottom: 8%; opacity: 0; }
    30% { opacity: 0.8; }
    to { bottom: 96%; opacity: 0; }
  }

  @media (prefers-reduced-motion: reduce) {
    .rain, .bubble { display: none; }
    .acid { transition: none; }
  }

  :global(:root[data-reduce-motion='true']) .rain,
  :global(:root[data-reduce-motion='true']) .bubble { display: none; }
  :global(:root[data-reduce-motion='true']) .acid { transition: none; }
</style>
