<script module lang="ts">
  export type IconKind = 'cpu' | 'gpu' | 'ram' | 'board' | 'disks' | 'system';
</script>

<script lang="ts">
  let { kind, size = 88 }: { kind: IconKind; size?: number } = $props();

  const BLACK = '#0b0b0b';
  const CREAM = '#f5f5f0';
  const PURPLE = '#cf00ff';
  const DARK = '#1d1d1d';
</script>

<svg
  class="icon-art"
  width={size}
  height={size}
  viewBox="0 0 88 88"
  aria-hidden="true"
  focusable="false"
  data-kind={kind}
>
  {#if kind === 'cpu'}
    <rect width="88" height="88" rx="12" fill={PURPLE} />
    {#each [24, 34, 44, 54, 64] as p (p)}
      <rect x={p - 2} y="12" width="4" height="8" fill={BLACK} />
      <rect x={p - 2} y="68" width="4" height="8" fill={BLACK} />
      <rect x="12" y={p - 2} width="8" height="4" fill={BLACK} />
      <rect x="68" y={p - 2} width="8" height="4" fill={BLACK} />
    {/each}
    <rect x="20" y="20" width="48" height="48" rx="5" fill={BLACK} />
    <rect x="31" y="31" width="26" height="26" rx="2" fill={CREAM} />
  {:else if kind === 'gpu'}
    <rect width="88" height="88" rx="12" fill={PURPLE} />
    <rect x="4" y="18" width="6" height="42" rx="1" fill={CREAM} />
    <rect x="10" y="20" width="70" height="38" rx="4" fill={BLACK} />
    {#each [31, 60] as cx (cx)}
      <circle {cx} cy="39" r="14" fill={DARK} stroke={CREAM} stroke-width="1.5" />
      {#each [0, 60, 120, 180, 240, 300] as angle (angle)}
        <ellipse {cx} cy="32" rx="3" ry="6.5" fill={CREAM} transform="rotate({angle} {cx} 39)" />
      {/each}
      <circle {cx} cy="39" r="3.4" fill={PURPLE} />
    {/each}
    {#each [18, 25, 32, 39, 46, 53, 60] as x (x)}
      <rect {x} y="62" width="4" height="7" fill={CREAM} />
    {/each}
  {:else if kind === 'ram'}
    <rect width="88" height="88" rx="12" fill={PURPLE} />
    {#each [14, 50] as y (y)}
      <rect x="8" {y} width="72" height="24" rx="3" fill={BLACK} />
      {#each [16, 30, 44, 58] as x (x)}
        <rect {x} y={y + 4} width="10" height="9" rx="1" fill={CREAM} />
      {/each}
      {#each [12, 18, 24, 30, 36, 42, 48, 54, 60, 66, 72] as x (x)}
        <rect {x} y={y + 18} width="3" height="4" fill={CREAM} />
      {/each}
    {/each}
  {:else if kind === 'board'}
    <rect width="88" height="88" rx="12" fill={PURPLE} />
    <rect x="16" y="12" width="56" height="64" rx="3" fill={BLACK} />
    <rect x="26" y="22" width="18" height="18" fill={PURPLE} />
    <rect x="50" y="22" width="14" height="4" fill={PURPLE} opacity="0.7" />
    <rect x="50" y="30" width="14" height="4" fill={PURPLE} opacity="0.7" />
    <rect x="26" y="52" width="38" height="4" fill={PURPLE} opacity="0.7" />
    <rect x="26" y="62" width="38" height="4" fill={PURPLE} opacity="0.7" />
  {:else if kind === 'disks'}
    <rect width="88" height="88" rx="12" fill={PURPLE} />
    <rect x="10" y="20" width="68" height="48" rx="5" fill={BLACK} />
    <rect x="17" y="27" width="36" height="16" rx="2" fill={CREAM} />
    <circle cx="66" cy="35" r="4" fill={PURPLE} />
    {#each [17, 23, 29, 35, 41, 47] as x (x)}
      <rect {x} y="55" width="3" height="7" fill={CREAM} />
    {/each}
    <rect x="58" y="57" width="12" height="3" rx="1.5" fill={PURPLE} />
  {:else}
    <rect width="88" height="88" rx="12" fill={PURPLE} />
    <rect x="12" y="16" width="48" height="36" rx="3" fill={BLACK} />
    <rect x="17" y="21" width="38" height="26" fill={CREAM} />
    <rect x="30" y="52" width="12" height="8" fill={BLACK} />
    <rect x="20" y="60" width="32" height="5" rx="1" fill={BLACK} />
    <rect x="64" y="16" width="12" height="49" rx="2" fill={BLACK} />
    <circle cx="70" cy="26" r="2" fill={PURPLE} />
    <rect x="66" y="56" width="8" height="3" fill={PURPLE} />
  {/if}
</svg>

<style>
  .icon-art {
    display: block;
    flex: none;
  }
</style>
