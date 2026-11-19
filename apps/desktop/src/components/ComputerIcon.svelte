<script module lang="ts">
  export type IconKind = 'cpu' | 'gpu' | 'ram' | 'board' | 'disks' | 'system';
</script>

<script lang="ts">
  let { kind, size = 88 }: { kind: IconKind; size?: number } = $props();

  const BLACK = '#0b0b0b';
  const CREAM = '#f5f5f0';
  const LIME = '#b4ff00';
  const PINK = '#ff009d';
  const BLUE = '#147cff';
  const YELLOW = '#f5ff00';
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
    <rect width="88" height="88" rx="12" fill={LIME} />
    {#each [24, 34, 44, 54, 64] as p (p)}
      <rect x={p - 2} y="12" width="4" height="8" fill={BLACK} />
      <rect x={p - 2} y="68" width="4" height="8" fill={BLACK} />
      <rect x="12" y={p - 2} width="8" height="4" fill={BLACK} />
      <rect x="68" y={p - 2} width="8" height="4" fill={BLACK} />
    {/each}
    <rect x="20" y="20" width="48" height="48" rx="5" fill={BLACK} />
    <rect x="31" y="31" width="26" height="26" rx="2" fill={CREAM} />
  {:else if kind === 'gpu'}
    <rect width="88" height="88" rx="12" fill={PINK} />
    <rect x="10" y="24" width="68" height="34" rx="4" fill={BLACK} />
    <rect x="6" y="30" width="6" height="22" fill={BLACK} />
    {#each [32, 58] as cx (cx)}
      <circle {cx} cy="41" r="11" fill={PINK} />
      <circle {cx} cy="41" r="3" fill={BLACK} />
      {#each [0, 72, 144, 216, 288] as angle (angle)}
        <ellipse {cx} cy="35" rx="2.4" ry="5" fill={BLACK} transform="rotate({angle} {cx} 41)" />
      {/each}
    {/each}
    <rect x="14" y="62" width="60" height="4" fill={YELLOW} />
    <rect x="18" y="58" width="8" height="4" fill={YELLOW} />
  {:else if kind === 'ram'}
    <rect width="88" height="88" rx="12" fill={BLUE} />
    {#each [20, 50] as y (y)}
      <rect x="8" {y} width="72" height="20" rx="3" fill={BLACK} />
      {#each [16, 30, 44, 58] as x (x)}
        <rect {x} y={y + 5} width="10" height="7" fill={LIME} />
      {/each}
      <rect x="16" y={y + 15} width="56" height="2" fill={YELLOW} />
    {/each}
  {:else if kind === 'board'}
    <rect width="88" height="88" rx="12" fill={YELLOW} />
    <rect x="16" y="12" width="56" height="64" rx="3" fill={BLACK} />
    <rect x="26" y="22" width="18" height="18" fill={YELLOW} />
    <rect x="50" y="22" width="14" height="4" fill={YELLOW} opacity="0.7" />
    <rect x="50" y="30" width="14" height="4" fill={YELLOW} opacity="0.7" />
    <rect x="26" y="52" width="38" height="4" fill={YELLOW} opacity="0.7" />
    <rect x="26" y="62" width="38" height="4" fill={YELLOW} opacity="0.7" />
  {:else if kind === 'disks'}
    <rect width="88" height="88" rx="12" fill={PINK} />
    <rect x="20" y="12" width="48" height="64" rx="5" fill={BLACK} />
    <rect x="27" y="20" width="34" height="22" rx="2" fill={CREAM} />
    <circle cx="28" cy="64" r="3" fill={PINK} />
    <rect x="38" y="62" width="22" height="4" rx="2" fill={PINK} opacity="0.6" />
  {:else}
    <rect width="88" height="88" rx="12" fill={LIME} />
    <rect x="12" y="16" width="48" height="36" rx="3" fill={BLACK} />
    <rect x="17" y="21" width="38" height="26" fill={BLUE} />
    <rect x="30" y="52" width="12" height="8" fill={BLACK} />
    <rect x="20" y="60" width="32" height="5" rx="1" fill={BLACK} />
    <rect x="64" y="16" width="12" height="49" rx="2" fill={BLACK} />
    <circle cx="70" cy="26" r="2" fill={LIME} />
    <rect x="66" y="56" width="8" height="3" fill={LIME} />
  {/if}
</svg>

<style>
  .icon-art {
    display: block;
    flex: none;
  }
</style>
