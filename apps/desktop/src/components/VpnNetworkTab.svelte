<script lang="ts">
  import SpeedTest from './SpeedTest.svelte';
  import GeoCards from './GeoCards.svelte';

  let { active = true }: { active?: boolean } = $props();
</script>

<SpeedTest {active} />
<GeoCards />
