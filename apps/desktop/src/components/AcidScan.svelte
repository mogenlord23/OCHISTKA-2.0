<script lang="ts">
  let { level, running }: { level: number; running: boolean } = $props();

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
      <path d="M0 10Q20 0 40 9T80 8T120 9T160 8T200 9T240 8V18H0Z" fill="#99c623" />
      <path d="M0 13Q20 6 40 12T80 11T120 12T160 11T200 12T240 11V18H0Z" fill="#d6fa5a" />
    </svg>
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
    transition: bottom 620ms cubic-bezier(0.23, 0.78, 0.32, 1);
  }

  .drop {
    position: absolute;
    top: 0;
    width: var(--size);
    height: 100%;
    color: #d9ff66;
    animation: sink var(--duration) linear var(--delay) infinite;
  }

  .drop:nth-child(2n) { color: #bdf04a; opacity: 0.78; }

  .drop svg {
    display: block;
    width: 100%;
    animation: turn var(--duration) linear infinite;
  }

  .acid {
    position: absolute;
    inset: auto 0 0;
    height: var(--level);
    background:
      radial-gradient(ellipse at 78% -20%, #ecff826e, transparent 60%),
      linear-gradient(174deg, #d6fa5a, #b0df25 48%, #6b9710);
    transition: height 620ms cubic-bezier(0.23, 0.78, 0.32, 1);
  }

  .acid.empty { visibility: hidden; }

  .surface {
    position: absolute;
    inset: auto auto calc(100% - 1px) -12%;
    width: 124%;
    height: 16px;
  }

  .running .surface { animation: wave 3.6s ease-in-out infinite alternate; }

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
    from { transform: translate(-50%, -20%); }
    to { transform: translate(-50%, 100%); }
  }

  @keyframes turn { to { transform: rotate(140deg); } }

  @keyframes wave {
    from { transform: translateX(-5%) scaleY(0.82); }
    to { transform: translateX(5%) scaleY(1.08); }
  }

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
    .acid { transition: none; }
    .running .surface { animation: none; }
  }

  :global(:root[data-reduce-motion='true']) .rain,
  :global(:root[data-reduce-motion='true']) .bubble,
  :global(:root[data-reduce-motion='true']) .splash { display: none; }
  :global(:root[data-reduce-motion='true']) .acid { transition: none; }
  :global(:root[data-reduce-motion='true']) .running .surface { animation: none; }
</style>
