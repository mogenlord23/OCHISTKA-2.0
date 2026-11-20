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
  import { onMount, tick } from 'svelte';
  import { navigation } from './lib/navigation.svelte';
  import { watchMaximized } from './lib/windowState';
  import { attachChromeGuards } from './lib/chromeGuards';
  import { attachUiSounds, playUiSound, preloadSounds } from './lib/sound';
  import { setSoundOn, soundState, syncSoundState } from './lib/soundState.svelte';
  import {
    BODY_FONT,
    HEADING_FONT,
    NEUTRAL,
    RADIUS,
    SHELL,
    SELECTION,
    STAGE_TRANSITION_MS,
    PATTERN,
    TYPE_SCALE,
    textOn,
  } from './lib/tokens';

  let maximized = $state(false);
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
    return () => {
      detachSounds();
      detachGuards();
      stopWatching();
    };
  });

  $effect(() => {
    document.documentElement.dataset.maximized = String(maximized);
  });

  let activeId = $state<SectionId>('vpn');
  let activeSub = $state('subscription');

  const section = $derived(SECTIONS.find((item) => item.id === activeId) ?? SECTIONS[0]);

  /* Смена страницы визуально мгновенная: только короткое проявление (90 мс), без сдвига и очереди. */
  const EASE = 'cubic-bezier(0.2, 0, 0, 1)';
  const FADE_IN = [{ opacity: 0.35 }, { opacity: 1 }];

  let main = $state<HTMLElement>();
  let firstRun = true;

  /** Смена страницы: короткое проявление 90 мс (только opacity).
      При смене раздела двигается всё содержимое, при смене вкладки только тело страницы.
      При prefers-reduced-motion: reduce анимации нет. */
  $effect(() => {
    const id = activeId;
    const sub = activeSub;
    if (firstRun) {
      firstRun = false;
      return;
    }
    void tick().then(() => {
      if (!main || typeof main.animate !== 'function') return;
      if (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) return;
      const sectionChanged = id !== lastId;
      lastId = id;
      void sub;
      const all = Array.from(main.children);
      const head = sectionChanged ? all.filter((el) => el.matches('h1, .lead, .subtabs')) : [];
      const body = all.filter((el) => !el.matches('h1, .lead, .subtabs'));
      for (const el of [...head, ...body]) el.animate(FADE_IN, { duration: 90, easing: EASE });
    });
  });
  let lastId: SectionId = 'vpn';

  /** «Показать процесс» из другого раздела: открываем «Процессы». */
  $effect(() => {
    if (navigation.processPid !== null) select('processes');
  });

  function select(id: SectionId) {
    activeId = id;
    activeSub = SECTIONS.find((item) => item.id === id)?.subsections[0]?.id ?? '';
  }
</script>

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
  style:--radius-xs={RADIUS.xs}
  style:--radius-sm={RADIUS.sm}
  style:--radius-md={RADIUS.md}
  style:--radius-lg={RADIUS.lg}
  style:--radius-pill={RADIUS.pill}
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
  style:--stage-ms="{STAGE_TRANSITION_MS}ms"
>
  <div class="body">
    <div class="stage">
      <TitleBar {maximized} />
      <main bind:this={main}>
        <h1 class="sr-only">{t(section.titleKey)}</h1>

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
              aria-pressed={soundOn}
              title={t(soundOn ? 'soundTitleOn' : 'soundTitleOff')}
              onclick={toggleSound}
            >
              {t('sound')}
              <span class="state">{t(soundOn ? 'soundOn' : 'soundOff')}</span>
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
</div>

<style>
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
    display: flex;
    flex-direction: column;
    height: 100vh;
    border-radius: var(--radius-lg);
    overflow: hidden;
    contain: layout paint;
    font-family: var(--font-body, system-ui, sans-serif);
    background: var(--window);
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
    display: flex;
    flex: 1;
    min-height: 0;
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
    /* Плавная смена цвета раздела (как в старой OCHISTKA): перетекает только общий фон области.
       Узор лежит на своём слое выше и остаётся на месте, карточки и текст не перерисовываются.
       Быстрые переключения не копятся: переход сразу идёт к последнему цвету. */
    transition: background-color var(--stage-ms) cubic-bezier(0, 0, 0.2, 1);
    /* Поля содержимого: на широком экране текст не растягивается дальше 1480 px. */
    --content-gutter: max(22px, calc((100% - 1480px) / 2));
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

  main {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden auto;
    padding: 0 var(--content-gutter) 8px;
    overscroll-behavior: contain;
    contain: layout paint;
  }

  /* Заголовок раздела на экране не показывается (вкладки сразу под шапкой),
     но остаётся для чтения с экрана. */
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
    justify-content: space-between;
    flex: none;
    width: 110px;
    background: var(--window);
    border-radius: var(--radius-lg) 0 0 var(--radius-lg);
  }

  /* Свёрнутый блок — только стрелка (высота 20 + нижний отступ 8). Раскрытая часть лежит
     над стрелкой поверх панели и не сдвигает пункты: на низком окне она перекрывает низ
     панели только пока открыта, а не выталкивает пункты за край. */
  .dock {
    position: relative;
    z-index: 10;
    background-color: var(--window);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-bottom: 8px;
  }

  /* Раскрытие: карточка 96 px «вырастает» снизу (clip-path) и чуть поднимается (transform), без смены
     высоты, без прозрачности (плитка под ней не просвечивает) и без пересчёта соседей. Открытие
     200 мс, закрытие быстрее, 160 мс. */
  .bottom-wrap {
    position: absolute;
    right: 6px;
    bottom: 28px;
    left: 6px;
    z-index: 10;
    isolation: isolate;
    overflow: hidden;
    border-radius: var(--radius-md);
    background-color: var(--black);
    visibility: hidden;
    clip-path: inset(100% 0 0 0 round var(--radius-md));
    transform: translateY(10px);
    transition:
      clip-path 160ms cubic-bezier(0.4, 0, 1, 1),
      transform 160ms cubic-bezier(0.4, 0, 1, 1),
      visibility 0s linear 160ms;
  }

  .bottom-wrap.open {
    visibility: visible;
    clip-path: inset(0 0 0 0 round var(--radius-md));
    transform: none;
    transition:
      clip-path 200ms cubic-bezier(0.2, 0, 0, 1),
      transform 200ms cubic-bezier(0.2, 0, 0, 1),
      visibility 0s;
  }

  /* Размер карточки один при любом окне: 96 px по высоте (на минимальном окне её верх на 588 px,
     ровно на верхней границе последней плитки панели, поэтому плитка не торчит полоской). */
  .bottom {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 96px;
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
    border-radius: var(--radius-sm);
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

  .pill.sound {
    width: 84px;
  }

  .state {
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
</style>
