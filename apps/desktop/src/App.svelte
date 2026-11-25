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
  import { i18n, setLang, t } from './lib/i18n.svelte';
  import { loadSubsections, saveSubsections, subFor, type SubMemory } from './lib/sectionMemory';
  import { onMount } from 'svelte';
  import { navigation } from './lib/navigation.svelte';
  import { slideInOnChange } from './lib/slideInOnChange';
  import { watchMaximized } from './lib/windowState';
  import { attachChromeGuards } from './lib/chromeGuards';
  import { attachUiSounds, playUiSound, preloadSounds } from './lib/sound';
  import { listenDesktop, type TrayAction } from './lib/tray';
  import { scan } from './lib/scanState.svelte';
  import { setSoundOn, soundState, syncSoundState } from './lib/soundState.svelte';
  import {
    BODY_FONT,
    HEADING_FONT,
    NEUTRAL,
    SHELL,
    SELECTION,
    PATTERN,
    TYPE_SCALE,
    textOn,
  } from './lib/tokens';

  let maximized = $state(false);
  let desktopNotice = $state('');
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;
  syncSoundState();
  const soundOn = $derived(soundState.on);

  function toggleSound() {
    setSoundOn(!soundState.on);
    if (soundState.on) playUiSound('click');
  }
  let dockPinned = $state(readPinned());
  let dockHover = $state(false);
  let dockLeaveTimer: ReturnType<typeof setTimeout> | undefined;

  function dockEnter() {
    clearTimeout(dockLeaveTimer);
    dockHover = true;
  }

  /** Небольшая задержка закрытия: при движении мыши между кнопками блок не мигает. */
  function dockLeave() {
    clearTimeout(dockLeaveTimer);
    dockLeaveTimer = setTimeout(() => (dockHover = false), 120);
  }
  const dockOpen = $derived(dockPinned || dockHover);

  function readPinned(): boolean {
    try {
      return localStorage.getItem('dockCollapsed') !== '1';
    } catch {
      return true;
    }
  }

  function toggleDock() {
    dockPinned = !dockOpen;
    dockHover = false;
    try {
      localStorage.setItem('dockCollapsed', dockPinned ? '0' : '1');
    } catch {
      /* без сохранения */
    }
  }

  onMount(() => {
    // Звуки декодируются заранее, чтобы щелчок и «хихи» играли без задержки.
    void preloadSounds();
    const detachSounds = attachUiSounds(window);
    const detachGuards = attachChromeGuards(window);
    const stopWatching = watchMaximized((value) => (maximized = value));
    const stopTray = listenDesktop<TrayAction>('tray-action', ({ payload }) => {
      const destination = SECTIONS.find((item) => item.id === payload);
      if (destination) select(destination.id);
    });
    const stopThreat = listenDesktop('scan-threat-found', () => {
      playUiSound('threat');
      showDesktopNotice('Обнаружена угроза. Проверка продолжается.');
    });
    window.addEventListener('pagehide', flushSubsections);
    const stopActionError = listenDesktop<string>('tray-action-error', ({ payload }) => showDesktopNotice(payload));
    return () => {
      window.removeEventListener('pagehide', flushSubsections);
      flushSubsections();
      detachSounds();
      detachGuards();
      stopWatching();
      stopTray();
      stopThreat();
      stopActionError();
      clearTimeout(noticeTimer);
    };
  });

  function showDesktopNotice(message: string) {
    desktopNotice = message;
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => (desktopNotice = ''), 8500);
  }

  $effect(() => {
    document.documentElement.dataset.maximized = String(maximized);
  });

  // Для каждого раздела помним последний открытый подраздел (и после перезапуска).
  let subMemory = $state<SubMemory>(loadSubsections());
  let activeId = $state<SectionId>('vpn');
  // svelte-ignore state_referenced_locally
  let activeSub = $state(subFor('vpn', subMemory));

  let subsectionSave: ReturnType<typeof setTimeout> | undefined;

  function flushSubsections() {
    if (subsectionSave === undefined) return;
    clearTimeout(subsectionSave);
    subsectionSave = undefined;
    saveSubsections(subMemory);
  }

  function chooseSub(id: string) {
    if (activeSub === id) return;
    activeSub = id;
    subMemory = { ...subMemory, [activeId]: id };
    clearTimeout(subsectionSave);
    subsectionSave = setTimeout(flushSubsections, 120);
  }

  const section = $derived(SECTIONS.find((item) => item.id === activeId) ?? SECTIONS[0]);

  /** «Показать процесс» из другого раздела: открываем «Процессы». */
  $effect(() => {
    if (navigation.processPid !== null) select('processes');
  });

  function select(id: SectionId) {
    if (activeId === id) return;
    activeId = id;
    activeSub = subFor(id, subMemory);
  }
</script>

{#snippet toolbar()}
  <TitleBar {maximized}>
    <Subtabs items={section.subsections} active={activeSub} onSelect={chooseSub} />
  </TitleBar>
{/snippet}

<div
  class="shell"
  data-maximized={maximized}
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
  style:--text-caption={TYPE_SCALE.caption}
  style:--text-small={TYPE_SCALE.small}
  style:--text-body={TYPE_SCALE.body}
  style:--text-lead={TYPE_SCALE.lead}
  style:--text-title={TYPE_SCALE.title}
  style:--text-section={TYPE_SCALE.section}
  style:--text-display={TYPE_SCALE.display}
  style:--pattern-opacity={PATTERN.opacity}
  style:--window={SHELL.window}
  style:--btn-bg={SHELL.button}
  style:--btn-border={SHELL.buttonBorder}
  style:--btn-hover={SHELL.buttonHover}
  style:--rail-hover={SHELL.railHover}
  style:--selection-bg={SELECTION.background}
  style:--selection-text={SELECTION.text}
>
  <div class="body">
    <div class="stage" data-tauri-drag-region>
      <div class="window-drag-strip" data-tauri-drag-region aria-hidden="true"></div>
      {#if activeId !== 'vpn'}
        <div class="section-toolbar" data-tauri-drag-region>{@render toolbar()}</div>
      {/if}
      <main
        class:vpn-main={activeId === 'vpn'}
        class:section-scroll={activeId !== 'vpn'}
        data-tauri-drag-region
        use:slideInOnChange={activeId === 'vpn' ? 'vpn' : `${activeId}:${activeSub}`}
      >
        <h1 class="sr-only">{t(section.titleKey)}</h1>

        <div class="vpn-surface" hidden={activeId !== 'vpn'}>
          <VpnPage
            subsection={subFor('vpn', subMemory)}
            active={activeId === 'vpn'}
            header={toolbar}
          />
        </div>

        {#if activeId === 'viruses'}
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
        {:else if activeId !== 'vpn' && section.subsections.length === 0}
          <PlaceholderPage note={t('subsectionsPending')} />
        {:else if activeId !== 'vpn'}
          <PlaceholderPage />
        {/if}
      </main>
    </div>

    <div class="side">
      <Navigation active={activeId} onSelect={select} />
      <div class="dock" role="group" onmouseenter={dockEnter} onmouseleave={dockLeave}>
        <div class="bottom-wrap" class:open={dockOpen} inert={!dockOpen}>
          <div class="bottom">
            <button
              type="button"
              class="pill sound"
              aria-label={t('sound')}
              aria-pressed={soundOn}
              title={t(soundOn ? 'soundTitleOn' : 'soundTitleOff')}
              onclick={toggleSound}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M11 5 6 9H3v6h3l5 4Z" />
                <path class="sound-waves" d="M15 9a5 5 0 0 1 0 6m3-9a9 9 0 0 1 0 12" />
                <path class="sound-slash" pathLength="1" d="m3 3 18 18" />
              </svg>
            </button>
            <div class="lang">
              <button
                type="button"
                class="pill"
                aria-pressed={i18n.lang === 'ru'}
                onclick={() => setLang('ru')}>{t('langRu')}</button
              >
              <button
                type="button"
                class="pill"
                aria-pressed={i18n.lang === 'en'}
                onclick={() => setLang('en')}>{t('langEn')}</button
              >
            </div>
          </div>
        </div>
        <button
          type="button"
          class="handle"
          class:open={dockOpen}
          aria-expanded={dockOpen}
          aria-label={dockOpen ? t('dockCollapse') : t('dockExpand')}
          onclick={toggleDock}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m7 10 5 5 5-5" /></svg>
        </button>
      </div>
    </div>
  </div>
  {#if desktopNotice}
    <aside class="desktop-toast" aria-live="polite">
      <span>{desktopNotice}</span>
      {#if scan.report?.findings.length}
        <button type="button" onclick={() => { select('viruses'); chooseSub('findings'); desktopNotice = ''; }}>Находки</button>
      {/if}
      <button type="button" aria-label="Скрыть уведомление" onclick={() => (desktopNotice = '')}>×</button>
    </aside>
  {/if}
</div>

<style>
  .desktop-toast {
    position: fixed;
    z-index: 20;
    inset: auto 136px 22px auto;
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: min(380px, 70vw);
    padding: 12px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-adaptive);
    background: var(--surface);
    color: var(--ink);
    font-size: var(--text-caption);
  }
  .desktop-toast button { border: 0; border-radius: var(--radius-adaptive); padding: 4px 6px; background: var(--surface-alt); color: var(--ink); cursor: pointer; }
  /* Окно прозрачное (tauri.conf.json: transparent, без тени). Скруглён только корень приложения;
     плавность изменения размера даёт contain (корень, сцена, прокручиваемая область). */
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    overflow: hidden;
    background: transparent;
    color: #fafafa;
  }

  :global(body) {
    overflow: hidden;
  }

  /* Полосы прокрутки скрыты везде, прокрутка колесом и касанием работает. */
  :global(*) {
    scrollbar-width: none;
  }

  :global(*::-webkit-scrollbar) {
    display: none;
  }

  /* Пока идёт переключение зажатой мышью, текст нигде не выделяется. */
  :global(body.dragging),
  :global(body.dragging *) {
    user-select: none;
    -webkit-user-select: none;
  }

  /* Выделение текста: в интерфейсе (заголовки, кнопки, вкладки, навигация, карточки, декор)
     выключено, разрешено в полях ввода и там, где нужно скопировать (.selectable). */
  .shell {
    user-select: none;
    -webkit-user-select: none;
  }

  .shell :global(input),
  .shell :global(textarea),
  .shell :global(.selectable),
  .shell :global(.selectable *) {
    user-select: text;
    -webkit-user-select: text;
  }

  /* Одно фирменное выделение везде (розовый, текст #080808, контраст 5.5:1). */
  .shell :global(::selection) {
    background: var(--selection-bg);
    color: var(--selection-text);
  }

  /* Нажатие и наведение: короткие переходы только по цвету и transform. */
  :global(button) {
    transition:
      background-color 130ms ease,
      border-color 130ms ease,
      color 130ms ease,
      transform 110ms cubic-bezier(0.2, 0, 0, 1);
  }

  /* Нажатие мгновенное (30 мс), возврат плавный (110 мс, из правила выше). */
  :global(button:active:not(:disabled)) {
    transform: scale(0.96);
    transition-duration: 30ms;
  }

  @media (prefers-reduced-motion: reduce) {
    :global(*),
    :global(*::before),
    :global(*::after) {
      animation: none !important;
      transition: none !important;
    }

    :global(button:active:not(:disabled)) {
      transform: none;
    }
  }

  .shell {
    --frame-inset: var(--section-inset);
    --section-gap: var(--frame-inset);
    --selection-duration: 190ms;
    --selection-ease: cubic-bezier(0.2, 0, 0, 1);
    --rail-tile: 88px;
    --side-width: calc(var(--rail-tile) + 2 * var(--frame-inset));
    display: flex;
    flex-direction: column;
    height: 100vh;
    border-radius: var(--radius-adaptive);
    overflow: hidden;
    contain: layout paint;
    font-family: var(--font-body, system-ui, sans-serif);
    /* Своего фона нет: всё окно закрыто цветной областью и чёрной панелью. Фон под ними создавал бы
       тёмную кайму по сглаженному скруглению. */
  }

  /* Развёрнутое окно: углы квадратные, от края до края. */
  .shell[data-maximized='true'] {
    border-radius: 0;
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
    position: relative;
    /* Свой слой: подложка ниже (z-index: -1) остаётся внутри .body и лежит под панелью. */
    isolation: isolate;
    display: flex;
    flex: 1;
    min-height: 0;
  }

  /* Под скруглёнными левыми углами чёрной правой панели виден цвет раздела, а не чёрный фон окна.
     Цвет лежит только до этих углов: под правыми углами панели, на самом краю окна, его нет.
     Иначе по сглаженной границе внешнего скругления цвет раздела просвечивал бы сквозь краешек
     чёрной панели (подложка и панель обе обрезаются по одной дуге и складываются). */
  .body::before {
    content: '';
    position: absolute;
    inset: 0 calc(var(--side-width) - var(--radius-limit)) 0 0;
    z-index: -1;
    background-color: var(--accent);
  }

  /* Цветная область раздела занимает окно от края до края (справа её ограничивает панель).
     Узор лежит на отдельном слое фиксированного размера: при изменении размера окна слой
     не перерисовывается, его только обрезают границы области. */
  .stage {
    position: relative;
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
    background-color: var(--accent);
    --section-gutter: max(var(--frame-inset), calc((100% - 1480px) / 2));
  }

  .stage::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    width: max(2560px, 100vw);
    height: max(1440px, 100vh);
    contain: strict;
    background: url('./assets/pop-pattern.png') 0 0 / 320px 320px;
    opacity: var(--pattern-opacity);
    pointer-events: none;
  }

  .stage > :global(*) {
    position: relative;
  }

  .stage > .window-drag-strip {
    position: absolute;
    inset: 0 0 auto;
    z-index: 1;
    height: var(--frame-inset);
    cursor: grab;
  }

  .window-drag-strip:active { cursor: grabbing; }

  .section-toolbar,
  main {
    margin-inline: var(--section-gutter);
  }

  .section-toolbar {
    padding-block: var(--frame-inset) var(--section-gap);
  }

  main.vpn-main {
    margin-block-start: var(--frame-inset);
  }

  main {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    margin-block-end: var(--frame-inset);
    contain: layout paint;
  }

  .vpn-surface {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .vpn-surface[hidden] { display: none; }

  :global(.section-scroll) {
    overflow: hidden auto;
    border-radius: var(--radius-adaptive);
    overscroll-behavior: contain;
  }

  /* Заголовок остаётся доступным программам чтения с экрана. */
  :global(.sr-only) {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
    border: 0;
  }

  .side {
    display: flex;
    flex-direction: column;
    flex: none;
    width: var(--side-width);
    background: var(--window);
    border-radius: var(--radius-adaptive) 0 0 var(--radius-adaptive);
    /* Фон панели и дока рисуется до самых краёв, поэтому углы скругляет обрезание, а не один радиус. */
    overflow: hidden;
  }

  /* Стрелка лежит посередине свободного места под последней плиткой. */
  .dock {
    --dock-gap: var(--content-gap);
    position: relative;
    z-index: 10;
    background-color: var(--window);
    display: flex;
    flex: 1;
    flex-direction: column;
    justify-content: center;
    align-items: center;
  }

  /* Карточка раскрывается над стрелкой. Поля вокруг последней плитки закрывают и её углы. */
  .bottom-wrap {
    position: absolute;
    inset-inline: 0;
    top: calc(-1 * (var(--rail-tile) + var(--dock-gap)));
    z-index: 10;
    isolation: isolate;
    overflow: hidden;
    border-radius: var(--radius-adaptive);
    background-color: var(--black);
    visibility: hidden;
    clip-path: inset(100% 0 0 0 round var(--radius-adaptive));
    transform: translateY(10px);
    transition:
      clip-path 160ms cubic-bezier(0.4, 0, 1, 1),
      transform 160ms cubic-bezier(0.4, 0, 1, 1),
      visibility 0s linear 160ms;
  }

  .bottom-wrap.open {
    visibility: visible;
    clip-path: inset(0 0 0 0 round var(--radius-adaptive));
    transform: none;
    transition:
      clip-path 200ms cubic-bezier(0.2, 0, 0, 1),
      transform 200ms cubic-bezier(0.2, 0, 0, 1),
      visibility 0s;
  }

  .bottom {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: calc(var(--rail-tile) + 2 * var(--dock-gap));
    min-height: 0;
    padding: 0 10px;
    overflow: hidden;
  }

  .handle {
    display: grid;
    place-items: center;
    width: 48px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-adaptive);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .handle:hover {
    background: var(--rail-hover);
    color: var(--ink);
  }

  .handle svg {
    width: 20px;
    height: 20px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: transform 220ms cubic-bezier(0.2, 0, 0, 1);
  }

  .handle:not(.open) svg {
    transform: rotate(180deg);
  }

  .pill {
    padding: 5px 10px;
    border: 1px solid var(--btn-border);
    border-radius: var(--radius-adaptive);
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

  .pill.sound {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    padding: 0;
  }

  .sound svg {
    width: 22px;
    height: 22px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .sound-waves {
    transition: opacity 150ms ease;
  }

  .sound-slash {
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    transition: stroke-dashoffset 180ms var(--selection-ease);
  }

  .sound[aria-pressed='false'] .sound-waves {
    opacity: 0;
  }

  .sound[aria-pressed='false'] .sound-slash {
    stroke-dashoffset: 0;
  }

  .lang {
    display: flex;
    gap: 8px;
  }
</style>
