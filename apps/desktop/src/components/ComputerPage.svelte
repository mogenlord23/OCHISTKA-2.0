<script lang="ts">
  import Home from './Home.svelte';
  import SettingsPage from './SettingsPage.svelte';
  import BoostPage from './BoostPage.svelte';
  import DriversPage from './DriversPage.svelte';

  let { subsection }: { subsection: string } = $props();
</script>

{#if subsection === 'components'}
  <Home />
{:else if subsection === 'settings'}
  <SettingsPage />
{:else if subsection === 'boost'}
  <BoostPage />
{:else if subsection === 'drivers'}
  <DriversPage />
{/if}
