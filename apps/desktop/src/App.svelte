<script lang="ts">
  import Home from './components/Home.svelte';
  import Placeholder from './components/Placeholder.svelte';
  import { SECTIONS, type SectionId } from './lib/sections';

  let activeId = $state<SectionId>('home');
  const activeSection = $derived(SECTIONS.find((section) => section.id === activeId));
</script>

<div class="layout">
  <nav aria-label="Разделы">
    {#each SECTIONS as section (section.id)}
      <button
        type="button"
        aria-current={section.id === activeId ? 'page' : undefined}
        onclick={() => (activeId = section.id)}
      >
        {section.title}
      </button>
    {/each}
  </nav>

  <main>
    {#if activeId === 'home'}
      <Home />
    {:else}
      <Placeholder title={activeSection?.title ?? ''} />
    {/if}
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
    font-family: system-ui, sans-serif;
    background: #0d1117;
    color: #c9d1d9;
  }

  .layout {
    display: flex;
    min-height: 100vh;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 220px;
    padding: 16px 8px;
    background: #161b22;
  }

  nav button {
    padding: 8px 12px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  nav button:hover {
    background: #21262d;
  }

  nav button[aria-current='page'] {
    background: #1f6feb;
    color: #ffffff;
  }

  main {
    flex: 1;
    padding: 24px;
  }
</style>
