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
  import { onMount, tick } from 'svelte';
  import { watchMaximized } from './lib/windowState';
  import {
    attachUiSounds,
    playUiSound,
    preloadSounds,
    readSoundEnabled,
    writeSoundEnabled,
  } from './lib/sound';
  import {
    BODY_FONT,
    HEADING_FONT,
    NEUTRAL,
    RADIUS,
    SHELL,
    PATTERN,
    TYPE_SCALE,
    textOn,
  } from './lib/tokens';

  let maximized = $state(false);
  let soundOn = $state(readSoundEnabled());

  function toggleSound() {
    soundOn = !soundOn;
    writeSoundEnabled(soundOn);
    if (soundOn) playUiSound('click');
  }
  let dockPinned = $state(readPinned());
  let dockHover = $state(false);
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
    const stopWatching = watchMaximized((value) => (maximized = value));
    return () => {
      detachSounds();
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
      const body = all.filter((el) => !el.matches('h1, .lead, .subtabs, footer'));
      for (const el of [...head, ...body]) el.animate(FADE_IN, { duration: 90, easing: EASE });
    });
  });
  let lastId: SectionId = 'vpn';

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
>
  <div class="body">
    <div class="stage">
      <TitleBar {maximized} />
      <main bind:this={main}>
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
      <div
        class="dock"
        role="group"
        onmouseenter={() => (dockHover = true)}
        onmouseleave={() => (dockHover = false)}
      >
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
                onclick={() => (i18n.lang = 'ru')}>{t('langRu')}</button
              >
              <button
                type="button"
                class="pill"
                aria-pressed={i18n.lang === 'en'}
                onclick={() => (i18n.lang = 'en')}>{t('langEn')}</button
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

  h1 {
    margin: 8px 0 0;
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
    padding-top: 8px;
    font-size: 10px;
    font-weight: 700;
    color: var(--page-text);
  }

  .side {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    flex: none;
    width: 110px;
    background: var(--window);
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

  /* Сворачивание: высота блока и прозрачность (маленький блок, поэтому без нагрузки). */
  .bottom-wrap {
    position: absolute;
    right: 6px;
    bottom: 28px;
    left: 6px;
    z-index: 10;
    display: grid;
    /* Карточка поверх панели: своя обводка и скругление из токена, чтобы срез плитки под ней
       читался как перекрытие, а не как обрезанная плитка. isolation: свой слой наложения. */
    isolation: isolate;
    overflow: hidden;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    /* Сплошной фон окна: раскрытый док целиком закрывает пункты под собой (без просвета). Прозрачность
       не анимируется, иначе в середине анимации цвет выбранной плитки просвечивает. */
    background-color: var(--window);
    grid-template-rows: 0fr;
    visibility: hidden;
    transition:
      grid-template-rows 220ms cubic-bezier(0.2, 0, 0, 1),
      visibility 0s linear 220ms;
  }

  .bottom-wrap.open {
    border-color: var(--border);
    grid-template-rows: 1fr;
    visibility: visible;
    transition:
      grid-template-rows 220ms cubic-bezier(0.2, 0, 0, 1),
      visibility 0s;
  }

  .bottom {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    min-height: 0;
    padding: 0 10px;
    overflow: hidden;
  }

  .bottom > :global(:first-child) {
    margin-top: 8px;
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
