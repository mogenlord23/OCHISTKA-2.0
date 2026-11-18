<script lang="ts">
  import type { DiskKind } from '../lib/generated/DiskKind';

  let { kind }: { kind: DiskKind } = $props();
</script>

<!-- Иконки накопителей в одном стиле: тёмный корпус, светлые детали, толстая линия 2.
     HDD: 3,5" жёсткий диск сверху, видны пластина, круглый шпиндель и рычаг головки.
     SSD: тонкая плата 2,5" с чипами памяти, контроллером и разъёмом. -->
<svg
  class="disk-icon"
  viewBox="0 0 64 64"
  width="56"
  height="56"
  data-kind={kind === 'ssd' ? 'ssd' : 'hdd'}
  aria-hidden="true"
  focusable="false"
>
  {#if kind === 'ssd'}
    <rect x="4" y="14" width="56" height="36" rx="4" fill="#080808" />
    <rect
      x="8"
      y="18"
      width="56"
      height="28"
      rx="2"
      fill="none"
      stroke="#f5f5f5"
      stroke-width="2"
    />
    <rect data-part="chip" x="12" y="22" width="10" height="8" rx="1" fill="#f5f5f5" />
    <rect data-part="chip" x="25" y="22" width="10" height="8" rx="1" fill="#f5f5f5" />
    <rect data-part="chip" x="12" y="34" width="10" height="8" rx="1" fill="#f5f5f5" />
    <rect data-part="chip" x="25" y="34" width="10" height="8" rx="1" fill="#f5f5f5" />
    <rect data-part="controller" x="40" y="26" width="12" height="12" rx="1" fill="#b4ff00" />
    <path d="M60 24h2v16h-2z" fill="#080808" />
    <g data-part="connector" fill="#f5f5f5">
      <rect x="5" y="46" width="2" height="2" />
      <rect x="9" y="46" width="2" height="2" />
      <rect x="13" y="46" width="2" height="2" />
    </g>
  {:else}
    <rect x="10" y="4" width="44" height="56" rx="4" fill="#080808" />
    <circle data-part="platter" cx="32" cy="27" r="18" fill="#d9d9d9" />
    <circle cx="32" cy="27" r="12" fill="none" stroke="#9a9a9a" stroke-width="2" />
    <circle
      data-part="spindle"
      cx="32"
      cy="27"
      r="5"
      fill="#080808"
      stroke="#f5f5f5"
      stroke-width="2"
    />
    <path
      data-part="arm"
      d="M46 52 L38 36"
      stroke="#f5f5f5"
      stroke-width="3"
      stroke-linecap="round"
      fill="none"
    />
    <circle cx="46" cy="52" r="3" fill="#f5f5f5" />
    <circle cx="14" cy="8" r="1.5" fill="#f5f5f5" />
    <circle cx="50" cy="8" r="1.5" fill="#f5f5f5" />
    <circle cx="14" cy="56" r="1.5" fill="#f5f5f5" />
    <rect x="18" y="52" width="14" height="4" rx="1" fill="#f5f5f5" />
  {/if}
</svg>

<style>
  .disk-icon {
    display: block;
  }
</style>
