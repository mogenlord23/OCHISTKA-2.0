<script lang="ts">
  import { LOGO_PATHS, type LogoPathId } from '../lib/driverLogos';
  import { VENDORS, type VendorId } from '../lib/driversCatalog';

  let { vendor, size = 58 }: { vendor: VendorId; size?: number } = $props();

  const info = $derived(VENDORS[vendor]);
  const path = $derived(vendor in LOGO_PATHS ? LOGO_PATHS[vendor as LogoPathId] : null);
</script>

<svg width={size} height={size} viewBox="0 0 58 58" aria-hidden="true" focusable="false">
  <rect width="58" height="58" rx="10" fill={info.color} />
  {#if path}
    <g transform="translate(11 11) scale(1.5)" fill="#fff"><path d={path} /></g>
  {:else if info.mark}
    <text
      x="29"
      y="36"
      text-anchor="middle"
      fill="#fff"
      font-family="Arial Black, Segoe UI, sans-serif"
      font-weight="900"
      font-size={info.mark.length > 1 ? 15 : 28}>{info.mark}</text
    >
  {/if}
</svg>
