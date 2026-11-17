<script lang="ts">
  import Navigation from './components/Navigation.svelte';
  import Subtabs from './components/Subtabs.svelte';
  import VirusesPage from './components/VirusesPage.svelte';
  import ComputerPage from './components/ComputerPage.svelte';
  import PlaceholderPage from './components/PlaceholderPage.svelte';
  import { SECTIONS, type SectionId } from './lib/sections';

  let activeId = $state<SectionId>('viruses');
  let activeSub = $state('scan');

  const section = $derived(SECTIONS.find((item) => item.id === activeId) ?? SECTIONS[0]);

  function select(id: SectionId) {
    activeId = id;
    activeSub = SECTIONS.find((item) => item.id === id)?.subsections[0]?.id ?? '';
  }
</script>

<div class="app" style:--accent={section.accent}>
  <main>
    <h1>{section.title}</h1>

    {#if section.subsections.length > 0}
      <Subtabs items={section.subsections} active={activeSub} onSelect={(id) => (activeSub = id)} />
    {/if}

    {#if activeId === 'viruses'}
      <VirusesPage subsection={activeSub} />
    {:else if activeId === 'computer'}
      <ComputerPage subsection={activeSub} />
    {:else if section.subsections.length === 0}
      <PlaceholderPage note="Подразделы уточняются" />
    {:else}
      <PlaceholderPage />
    {/if}
  </main>

  <Navigation active={activeId} onSelect={select} />
</div>

<style>
  :global(body) {
    margin: 0;
    font-family: system-ui, sans-serif;
    background: #080808;
    color: #fafafa;
  }

  .app {
    display: flex;
    min-height: 100vh;
  }

  main {
    flex: 1;
    padding: 24px 28px;
    min-width: 0;
  }

  h1 {
    margin: 0;
    font-size: 40px;
    font-weight: 900;
    text-transform: uppercase;
    color: var(--accent);
  }
</style>
