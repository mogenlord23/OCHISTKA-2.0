<script lang="ts">
  import Home from './Home.svelte';
  import PlaceholderPage from './PlaceholderPage.svelte';

  let { subsection }: { subsection: string } = $props();
</script>

{#if subsection === 'components'}
  <Home />
{:else}
  <PlaceholderPage />
{/if}
