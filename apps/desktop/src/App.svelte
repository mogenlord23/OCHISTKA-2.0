<script lang="ts">
  import Navigation from './components/Navigation.svelte';
  import Subtabs from './components/Subtabs.svelte';
  import TitleBar from './components/TitleBar.svelte';
  import VirusesPage from './components/VirusesPage.svelte';
  import VpnPage from './components/VpnPage.svelte';
  import ComputerPage from './components/ComputerPage.svelte';
  import PlaceholderPage from './components/PlaceholderPage.svelte';
  import { SECTIONS, type SectionId } from './lib/sections';
  import { BODY_FONT, HEADING_FONT, textOn } from './lib/tokens';

  const VERSION = '0.1.0';

  let activeId = $state<SectionId>('vpn');
  let activeSub = $state('subscription');
  let soundOn = $state(true);

  const section = $derived(SECTIONS.find((item) => item.id === activeId) ?? SECTIONS[0]);

  function select(id: SectionId) {
    activeId = id;
    activeSub = SECTIONS.find((item) => item.id === id)?.subsections[0]?.id ?? '';
  }
</script>

<div
  class="shell"
  style:--accent={section.accent}
  style:--page-text={textOn(section.accent)}
  style:--font-heading={HEADING_FONT}
  style:--font-body={BODY_FONT}
>
  <TitleBar />

  <div class="body">
    <main>
      <h1>{section.title}</h1>
      {#if section.lead}<p class="lead">{section.lead}</p>{/if}

      {#if section.subsections.length > 0}
        <Subtabs
          items={section.subsections}
          active={activeSub}
          onSelect={(id) => (activeSub = id)}
        />
      {/if}

      {#if activeId === 'vpn'}
        <VpnPage subsection={activeSub} />
      {:else if activeId === 'viruses'}
        <VirusesPage subsection={activeSub} />
      {:else if activeId === 'computer'}
        <ComputerPage subsection={activeSub} />
      {:else if section.subsections.length === 0}
        <PlaceholderPage note="Подразделы уточняются" />
      {:else}
        <PlaceholderPage />
      {/if}

      <footer>ЛОКАЛЬНО. ПОД ТВОИМ КОНТРОЛЕМ.</footer>
    </main>

    <div class="side">
      <Navigation active={activeId} onSelect={select} />
      <div class="bottom">
        <button
          type="button"
          class="pill"
          onclick={() => (soundOn = !soundOn)}
          aria-pressed={soundOn}
        >
          ЗВУК: {soundOn ? 'ВКЛ' : 'ВЫКЛ'}
        </button>
        <div class="lang">
          <button type="button" class="pill" aria-pressed="true">RU</button>
          <button type="button" class="pill" disabled title="Английский появится позже">ENG</button>
        </div>
        <small>{VERSION}</small>
      </div>
    </div>
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    font-family: var(--font-body, system-ui, sans-serif);
    background: #080808;
    color: #fafafa;
  }

  .shell {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    background-color: var(--accent);
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160' viewBox='0 0 160 160'%3E%3Ccircle cx='24' cy='28' r='6' fill='%23000' fill-opacity='.07'/%3E%3Cpath d='M110 20l6 12 13 2-9 9 2 13-12-6-12 6 2-13-9-9 13-2z' fill='%23000' fill-opacity='.07'/%3E%3Crect x='60' y='96' width='14' height='14' rx='3' fill='%23000' fill-opacity='.07'/%3E%3Cpath d='M22 120q10-14 22 0t22 0' stroke='%23000' stroke-opacity='.07' stroke-width='3' fill='none'/%3E%3C/svg%3E");
  }

  /* Видимый фокус с клавиатуры: цвет текста страницы, контраст как у текста. */
  :global(button:focus-visible, input:focus-visible) {
    outline: 3px solid var(--page-text);
    outline-offset: 2px;
  }

  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  main {
    flex: 1;
    min-width: 0;
    padding: 0 24px 16px;
  }

  h1 {
    margin: 4px 0 0;
    font-family: var(--font-heading);
    font-size: 44px;
    font-weight: 900;
    text-transform: uppercase;
    color: var(--page-text);
  }

  .lead {
    margin: 4px 0 0;
    font-size: 14px;
    font-weight: 700;
    color: var(--page-text);
  }

  footer {
    margin-top: 24px;
    font-size: 11px;
    color: var(--page-text);
  }

  .side {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    background: #080808;
  }

  .bottom {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
    padding: 12px 8px;
  }

  .pill {
    padding: 8px 6px;
    border: 0;
    border-radius: 8px;
    background: #141414;
    color: #fafafa;
    font: inherit;
    font-size: 11px;
    font-weight: 800;
    cursor: pointer;
  }

  .pill[aria-pressed='true'] {
    background: #fafafa;
    color: #080808;
  }

  .pill:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .lang {
    display: flex;
    gap: 6px;
  }

  .lang .pill {
    flex: 1;
  }

  .bottom small {
    text-align: center;
    color: #8b8b8b;
    font-size: 11px;
  }
</style>
