<script lang="ts">
  import { SYSTEM_CHANGE_REASON } from '../lib/computer';
  import Home from './Home.svelte';
  import PlaceholderPage from './PlaceholderPage.svelte';
  import DriversPage from './DriversPage.svelte';

  let { subsection }: { subsection: string } = $props();
</script>

{#if subsection === 'components'}
  <Home />
{:else if subsection === 'drivers'}
  <DriversPage />
{:else}
  <PlaceholderPage note={SYSTEM_CHANGE_REASON} />
{/if}
