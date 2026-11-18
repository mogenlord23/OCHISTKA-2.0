<script lang="ts">
  import Navigation from './components/Navigation.svelte';
  import Subtabs from './components/Subtabs.svelte';
  import TitleBar from './components/TitleBar.svelte';
  import VirusesPage from './components/VirusesPage.svelte';
  import VpnPage from './components/VpnPage.svelte';
  import ComputerPage from './components/ComputerPage.svelte';
  import DisksPage from './components/DisksPage.svelte';
  import ProcessesPage from './components/ProcessesPage.svelte';
  import ProgramsPage from './components/ProgramsPage.svelte';
  import PlaceholderPage from './components/PlaceholderPage.svelte';
  import AppearancePage from './components/AppearancePage.svelte';
  import { SECTIONS, type SectionId } from './lib/sections';
  import { i18n, t } from './lib/i18n.svelte';
  import { BODY_FONT, HEADING_FONT, NEUTRAL, RADIUS, TYPE_SCALE, textOn } from './lib/tokens';

  const VERSION = '0.5.5';

  let activeId = $state<SectionId>('vpn');
  let activeSub = $state('subscription');

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
  style:--ink={NEUTRAL.ink}
  style:--ink-soft={NEUTRAL.inkSoft}
  style:--black={NEUTRAL.black}
  style:--muted={NEUTRAL.muted}
  style:--surface={NEUTRAL.surface}
  style:--surface-alt={NEUTRAL.surfaceAlt}
  style:--border={NEUTRAL.border}
  style:--radius-sm={RADIUS.sm}
  style:--radius-md={RADIUS.md}
  style:--radius-lg={RADIUS.lg}
  style:--text-caption={TYPE_SCALE.caption}
  style:--text-small={TYPE_SCALE.small}
  style:--text-body={TYPE_SCALE.body}
  style:--text-lead={TYPE_SCALE.lead}
  style:--text-title={TYPE_SCALE.title}
  style:--text-section={TYPE_SCALE.section}
  style:--text-display={TYPE_SCALE.display}
>
  <TitleBar />

  <div class="body">
    <main>
      <h1>{t(section.titleKey)}</h1>
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
      {:else if activeId === 'disks'}
        <DisksPage subsection={activeSub} />
      {:else if activeId === 'processes'}
        <ProcessesPage subsection={activeSub} />
      {:else if activeId === 'programs'}
        <ProgramsPage subsection={activeSub} />
      {:else if activeId === 'design'}
        <AppearancePage subsection={activeSub} />
      {:else if section.subsections.length === 0}
        <PlaceholderPage note={t('subsectionsPending')} />
      {:else}
        <PlaceholderPage />
      {/if}

      <footer>{t('footer')}</footer>
    </main>

    <div class="side">
      <Navigation active={activeId} onSelect={select} />
      <div class="bottom">
        <button type="button" class="pill sound" disabled title={t('soundTitle')}>
          {t('sound')}
          <span class="reason">{t('soundReason')}</span>
        </button>
        <div class="lang">
          <button
            type="button"
            class="pill"
            aria-pressed={i18n.lang === 'ru'}
            onclick={() => (i18n.lang = 'ru')}>{t('langRu')}</button
          >
          <button
            type="button"
            class="pill"
            aria-pressed={i18n.lang === 'en'}
            onclick={() => (i18n.lang = 'en')}>{t('langEn')}</button
          >
        </div>
        <small>{VERSION}</small>
      </div>
    </div>
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    background: #080808;
    color: #fafafa;
  }

  .shell {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    font-family: var(--font-body, system-ui, sans-serif);
    background-color: var(--accent);
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160' viewBox='0 0 160 160'%3E%3Ccircle cx='24' cy='28' r='6' fill='%23fff' fill-opacity='.10'/%3E%3Cpath d='M110 20l6 12 13 2-9 9 2 13-12-6-12 6 2-13-9-9 13-2z' fill='%23fff' fill-opacity='.10'/%3E%3Crect x='60' y='96' width='14' height='14' rx='3' fill='%23fff' fill-opacity='.10'/%3E%3Cpath d='M22 120q10-14 22 0t22 0' stroke='%23fff' stroke-opacity='.10' stroke-width='3' fill='none'/%3E%3C/svg%3E");
  }

  /* Видимый фокус с клавиатуры, двойное кольцо. Внешнее (цвет текста страницы) читается
     на цветной странице, но на правой панели сливается с её фоном (оба #080808).
     Внутреннее светлое (--ink, 2px, ровно на величину отступа) читается на панели. */
  :global(button:focus-visible, input:focus-visible) {
    outline: 3px solid var(--page-text);
    outline-offset: 2px;
    box-shadow: 0 0 0 2px var(--ink);
  }

  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  main {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    padding: 0 24px 14px;
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
    margin-top: auto;
    padding-top: 20px;
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

  /* Звук недоступен: приглушённый, но читаемый (--muted на --surface даёт около 5.4:1). */
  .pill.sound:disabled {
    opacity: 1;
    background: var(--surface);
    color: var(--muted);
  }

  .reason {
    display: block;
    font-size: var(--text-caption);
    font-weight: 700;
    text-transform: none;
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
