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
  import {
    BODY_FONT,
    HEADING_FONT,
    NEUTRAL,
    RADIUS,
    SHELL,
    TYPE_SCALE,
    textOn,
  } from './lib/tokens';

  const VERSION = '0.1.0';

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
  style:--window={SHELL.window}
  style:--btn-bg={SHELL.button}
  style:--btn-border={SHELL.buttonBorder}
  style:--btn-hover={SHELL.buttonHover}
  style:--rail-hover={SHELL.railHover}
>
  <div class="body">
    <div class="stage">
      <TitleBar />
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
    </div>

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
    background: #0b0b0b;
    color: #fafafa;
  }

  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    font-family: var(--font-body, system-ui, sans-serif);
    background: var(--window);
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

  /* Цветная область раздела: скруглённый прямоугольник с отступом от краёв окна. */
  .stage {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    margin: 8px 0 8px 8px;
    border-radius: var(--radius-lg);
    overflow: hidden;
    background-color: var(--accent);
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='120' viewBox='0 0 120 120'%3E%3Cg fill='%23000' fill-opacity='.045'%3E%3Cpath d='M10 20q8-8 16 0q-8 8-16 0z'/%3E%3Cpath d='M70 8l-6 12h6l-4 12 10-15h-6l4-9z'/%3E%3Ccircle cx='96' cy='58' r='5'/%3E%3Cpath d='M20 80h10l2 5h-14zM34 80h10l2 5h-14z'/%3E%3Cpath d='M64 88l3 6 7 1-5 5 1 7-6-3-6 3 1-7-5-5 7-1z'/%3E%3Cpath d='M100 100q4-10 10 0'/%3E%3C/g%3E%3C/svg%3E");
  }

  main {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    padding: 0 22px 10px;
  }

  h1 {
    margin: 12px 0 0;
    font-family: var(--font-heading);
    font-size: 34px;
    font-weight: 900;
    line-height: 1.1;
    text-transform: uppercase;
    color: var(--page-text);
  }

  .lead {
    margin: 4px 0 0;
    font-size: var(--text-small);
    font-weight: 700;
    color: var(--page-text);
  }

  footer {
    margin-top: auto;
    padding-top: 14px;
    font-size: 10px;
    font-weight: 700;
    color: var(--page-text);
  }

  .side {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    width: 110px;
    background: var(--window);
  }

  .bottom {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 10px 10px 14px;
  }

  .pill {
    padding: 5px 10px;
    border: 1px solid var(--btn-border);
    border-radius: var(--radius-sm);
    background: var(--btn-bg);
    color: var(--ink);
    font: inherit;
    font-size: 10px;
    font-weight: 800;
    cursor: pointer;
  }

  .pill[aria-pressed='false'] {
    color: var(--muted);
  }

  .pill[aria-pressed='true'] {
    border-color: #4a4a4a;
  }

  /* Звук недоступен: приглушённый, но читаемый (--muted на --surface даёт около 5.4:1). */
  .pill.sound {
    width: 84px;
  }

  .pill.sound:disabled {
    background: var(--surface);
    color: var(--ink-soft);
    cursor: not-allowed;
  }

  .reason {
    display: block;
    font-size: 9px;
    font-weight: 600;
    color: var(--muted);
    text-transform: none;
  }

  .lang {
    display: flex;
    gap: 8px;
  }

  .bottom small {
    text-align: center;
    color: var(--muted);
    font-size: 10px;
  }
</style>
