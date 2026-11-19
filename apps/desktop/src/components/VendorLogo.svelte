<script lang="ts">
  import { LOGO_PATHS, type LogoPathId } from '../lib/driverLogos';
  import { VENDORS, type VendorId } from '../lib/driversCatalog';

  let { vendor, size = 58 }: { vendor: VendorId | 'all'; size?: number } = $props();

  const info = $derived(vendor === 'all' ? null : VENDORS[vendor]);
  const path = $derived(
    vendor !== 'all' && vendor in LOGO_PATHS ? LOGO_PATHS[vendor as LogoPathId] : null,
  );
</script>

<svg width={size} height={size} viewBox="0 0 58 58" aria-hidden="true" focusable="false">
  <rect width="58" height="58" rx="10" fill={info?.color ?? '#3a3a3a'} />
  {#if path}
    <g transform="translate(11 11) scale(1.5)" fill="#fff"><path d={path} /></g>
  {:else if info?.mark}
    <text
      x="29"
      y="36"
      text-anchor="middle"
      fill="#fff"
      font-family="Arial Black, Segoe UI, sans-serif"
      font-weight="900"
      font-size={info.mark.length > 1 ? 15 : 28}>{info.mark}</text
    >
  {:else}
    <text
      x="29"
      y="35"
      text-anchor="middle"
      fill="#fff"
      font-family="Arial Black, Segoe UI, sans-serif"
      font-weight="900"
      font-size="15">ВСЕ</text
    >
  {/if}
</svg>
