<script lang="ts">
  import type { SectionId } from '../lib/sections';
  let { id }: { id: SectionId } = $props();
</script>

<svg viewBox="0 0 24 24" aria-hidden="true">
  {#if id === 'vpn'}
    <circle cx="12" cy="12" r="9" />
    <path d="M3 12h18" />
    <path d="M12 3c-3.2 3-3.2 15 0 18M12 3c3.2 3 3.2 15 0 18" />
  {:else if id === 'viruses'}
    <path d="M12 3 4.5 6v5.5c0 4.6 3 8 7.5 9.5 4.5-1.5 7.5-4.9 7.5-9.5V6z" />
  {:else if id === 'processes'}
    <path d="M6.5 3h11a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z" />
    <path d="M9 8h6M9 12h6M9 16h6" />
  {:else if id === 'programs'}
    <rect x="3.5" y="3.5" width="7" height="7" rx="1.5" />
    <rect x="13.5" y="3.5" width="7" height="7" rx="1.5" />
    <rect x="3.5" y="13.5" width="7" height="7" rx="1.5" />
    <rect x="13.5" y="13.5" width="7" height="7" rx="1.5" />
  {:else if id === 'disks'}
    <path
      d="M5.2 4h13.6a1.5 1.5 0 0 1 1.4 1l2 5.7V18a2 2 0 0 1-2 2H3.8a2 2 0 0 1-2-2v-7.3l2-5.7a1.5 1.5 0 0 1 1.4-1z"
    />
    <path d="M1.8 11h20.4" />
    <path d="M6 15.5h6" />
    <circle cx="17.5" cy="15.5" r="0.6" />
  {:else if id === 'computer'}
    <rect x="2.5" y="3.5" width="19" height="13" rx="2" />
    <path d="M8 20.5h8M12 16.5v4" />
  {:else}
    <path d="M4 7h16M4 12h16M4 17h16" />
    <circle cx="9" cy="7" r="2.2" class="knob" />
    <circle cx="16" cy="12" r="2.2" class="knob" />
    <circle cx="8" cy="17" r="2.2" class="knob" />
  {/if}
</svg>

<style>
  svg {
    display: block;
    width: var(--section-icon-size, 28px);
    height: var(--section-icon-size, 28px);
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
