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
    {#each [24, 36, 48, 60] as p (p)}
      <rect x={p} y="12" width="4" height="8" fill={BLACK} />
      <rect x={p} y="68" width="4" height="8" fill={BLACK} />
      <rect x="12" y={p} width="8" height="4" fill={BLACK} />
      <rect x="68" y={p} width="8" height="4" fill={BLACK} />
    {/each}
    <rect x="20" y="20" width="48" height="48" rx="6" fill={BLACK} />
    <rect x="31" y="31" width="26" height="26" rx="3" fill={CREAM} />
  {:else if kind === 'gpu'}
    <rect width="88" height="88" rx="12" fill={PINK} />
    <rect x="8" y="22" width="72" height="36" rx="5" fill={BLACK} />
    {#each [30, 58] as cx (cx)}
      <circle {cx} cy="40" r="12" fill={PINK} />
      <rect x={cx - 2} y="30" width="4" height="20" rx="2" fill={BLACK} />
      <rect x={cx - 10} y="38" width="20" height="4" rx="2" fill={BLACK} />
      <circle {cx} cy="40" r="3" fill={PINK} />
    {/each}
    <rect x="16" y="62" width="56" height="4" fill={YELLOW} />
  {:else if kind === 'ram'}
    <rect width="88" height="88" rx="12" fill={BLUE} />
    {#each [19, 49] as y (y)}
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
    <rect x="12" y="20" width="48" height="34" rx="3" fill={BLACK} />
    <rect x="17" y="25" width="38" height="24" fill={BLUE} />
    <rect x="30" y="54" width="12" height="6" fill={BLACK} />
    <rect x="20" y="60" width="32" height="8" rx="2" fill={BLACK} />
    <rect x="64" y="20" width="12" height="48" rx="2" fill={BLACK} />
    <circle cx="70" cy="30" r="2" fill={LIME} />
    <rect x="66" y="58" width="8" height="3" fill={LIME} />
  {/if}
</svg>

<style>
  .icon-art {
    display: block;
    flex: none;
  }
</style>
