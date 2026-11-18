<script lang="ts">
  import { cursorSvg, CURSOR_DEFS, tilesFor, type AppearanceKind } from '../lib/appearance';
  import {
    applyReduceMotion,
    loadChoice,
    loadSettings,
    saveChoice,
    saveSettings,
    type AppearanceChoice,
  } from '../lib/appearanceStore';
  import { i18n } from '../lib/i18n.svelte';
  import pkg from '../../package.json';

  let { subsection }: { subsection: string } = $props();

  const APPLY_REASON = 'Применение к Windows пока не подключено';
  const PAGE_SIZE = 40;
  const REPO_URL = 'https://github.com/mogenlord23/OCHISTKA-2.0';

  const KIND_BY_SUBSECTION: Record<string, AppearanceKind> = {
    wallpapers: 'wallpaper',
    cursors: 'cursor',
    colors: 'color',
  };

  const HEADINGS: Record<AppearanceKind, { title: string; hint: string }> = {
    wallpaper: {
      title: 'Обои',
      hint: 'Нажми на картинку, чтобы выбрать фон. Выбор запоминается; установка в Windows появится позже.',
    },
    cursor: {
      title: 'Курсоры',
      hint: 'Полные схемы курсора Windows здесь доступны только для просмотра. Системная схема не изменяется.',
    },
    color: {
      title: 'Цвета',
      hint: 'Нажми на цвет, чтобы выбрать акцент. Выбор запоминается; применение появится позже.',
    },
  };

  let choice = $state<AppearanceChoice>(loadChoice());
  let reduceMotion = $state(loadSettings().reduceMotion);
  let visible = $state(PAGE_SIZE);
  let sentinel = $state<HTMLElement | null>(null);

  const isSettings = $derived(subsection === 'settings');
  const kind = $derived<AppearanceKind>(KIND_BY_SUBSECTION[subsection] ?? 'wallpaper');
  const tiles = $derived(tilesFor(kind));
  const selectedId = $derived(choice[kind]);
  const selectedLabel = $derived(tiles.find((tile) => tile.id === selectedId)?.label ?? '');
  const shown = $derived(kind === 'wallpaper' ? tiles.slice(0, visible) : tiles);
  const cursorById = new Map(CURSOR_DEFS.map((def) => [def.id, def.cursor]));

  function pick(id: string) {
    saveChoice(kind, id);
    choice = { ...choice, [kind]: id };
  }

  function toggleMotion() {
    reduceMotion = !reduceMotion;
    saveSettings({ reduceMotion });
    applyReduceMotion(reduceMotion);
  }

  // Обои подгружаются порциями: когда низ списка виден, добавляем ещё порцию.
  $effect(() => {
    if (!sentinel || typeof IntersectionObserver === 'undefined') return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) visible += PAGE_SIZE;
      },
      { rootMargin: '400px' },
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  });
</script>

<section class="appearance" aria-label="Оформление">
  {#key subsection}
    <div class="pane">
      {#if isSettings}
        <h2>Настройки приложения</h2>
        <p class="hint">Меняют только это приложение, Windows не затрагивают.</p>

        <div class="setting">
          <span class="setting-name" id="lang-label">Язык / Language</span>
          <div class="seg" role="group" aria-labelledby="lang-label">
            <button
              type="button"
              aria-pressed={i18n.lang === 'ru'}
              onclick={() => (i18n.lang = 'ru')}>RU</button
            >
            <button
              type="button"
              aria-pressed={i18n.lang === 'en'}
              onclick={() => (i18n.lang = 'en')}>ENG</button
            >
          </div>
        </div>

        <div class="setting">
          <span class="setting-name" id="motion-label">Меньше анимации</span>
          <button
            type="button"
            class="switch"
            role="switch"
            aria-checked={reduceMotion}
            aria-labelledby="motion-label"
            onclick={toggleMotion}
          >
            <span class="knob"></span>
          </button>
        </div>

        <div class="setting">
          <span class="setting-name">Версия</span>
          <span>OCHISTKA {pkg.version}</span>
        </div>
        <div class="setting">
          <span class="setting-name">Ссылки</span>
          <a href={REPO_URL} target="_blank" rel="noopener noreferrer">Исходный код на GitHub</a>
        </div>
      {:else}
        <h2>{HEADINGS[kind].title}</h2>
        <p class="hint">{HEADINGS[kind].hint}</p>

        <div class="actions">
          {#if kind === 'wallpaper'}
            <button type="button" disabled title={APPLY_REASON}>Добавить свои обои…</button>
            <button type="button" disabled title={APPLY_REASON}>Вернуть прежний фон</button>
          {/if}
          <button type="button" disabled aria-describedby="apply-reason">Применить к Windows</button
          >
        </div>
        <p class="current">
          Выбрано: <strong>{selectedLabel}</strong>
          <span id="apply-reason" class="reason">{APPLY_REASON}</span>
        </p>

        <ul class="grid grid-{kind}">
          {#each shown as tile (tile.id)}
            {@const isSelected = tile.id === selectedId}
            {@const cursor = cursorById.get(tile.id)}
            <li>
              <button
                type="button"
                class="tile"
                class:selected={isSelected}
                aria-pressed={isSelected}
                title={tile.description}
                style:background-image={tile.preview.includes('gradient') ? tile.preview : null}
                style:background-color={tile.preview.includes('gradient') ? null : tile.preview}
                onclick={() => pick(tile.id)}
              >
                {#if tile.thumb}
                  <img
                    src={tile.thumb}
                    alt=""
                    loading="lazy"
                    decoding="async"
                    width="288"
                    height="162"
                  />
                {:else if cursor}
                  <!-- eslint-disable-next-line svelte/no-at-html-tags -- собственные данные, не ввод -->
                  <span class="cursor" aria-hidden="true">{@html cursorSvg(cursor, tile.id)}</span>
                {/if}
                <span class="label">{tile.label}</span>
                {#if isSelected}<span class="state">✓<span class="sr">Выбрано</span></span>{/if}
              </button>
            </li>
          {/each}
        </ul>
        {#if kind === 'wallpaper' && visible < tiles.length}
          <div bind:this={sentinel} class="more">
            <button type="button" onclick={() => (visible += PAGE_SIZE)}>
              Показать ещё ({tiles.length - visible})
            </button>
          </div>
        {/if}
      {/if}
    </div>
  {/key}
</section>

<style>
  .appearance {
    color: var(--page-text);
  }

  .pane {
    animation: pane-in 220ms ease-out both;
  }

  @keyframes pane-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }

  h2 {
    margin: 0 0 6px;
    font-family: var(--font-heading);
    font-size: var(--text-section);
    font-weight: 900;
    text-transform: uppercase;
  }

  .hint {
    margin: 0 0 14px;
    font-size: var(--text-small);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .actions button,
  .more button {
    min-height: 44px;
    padding: 10px 14px;
    border: 1px solid var(--btn-border, #343434);
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
    font: inherit;
    font-size: var(--text-small);
    font-weight: 600;
  }

  .actions button {
    cursor: not-allowed;
  }

  .actions button:disabled {
    color: var(--ink-soft);
  }

  .more {
    display: flex;
    justify-content: center;
    margin: 14px 0;
  }

  .more button {
    cursor: pointer;
  }

  .reason {
    margin: 0 0 0 12px;
    font-size: var(--text-caption);
    font-weight: 700;
  }

  .current {
    margin: 8px 0 10px;
    font-size: var(--text-small);
  }

  .grid {
    display: grid;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .grid-wallpaper {
    grid-template-columns: repeat(5, minmax(0, 1fr));
  }

  .grid-cursor,
  .grid-color {
    grid-template-columns: repeat(7, minmax(0, 1fr));
  }

  .grid li {
    content-visibility: auto;
    contain-intrinsic-size: auto 90px;
  }

  .tile {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    aspect-ratio: 16 / 9;
    min-height: 44px;
    padding: 0;
    overflow: hidden;
    border: 0;
    border-radius: 14px;
    background-origin: border-box;
    background-repeat: no-repeat;
    box-shadow: inset 0 0 0 1px rgb(0 0 0 / 30%);
    color: var(--ink);
    font: inherit;
    cursor: pointer;
    transition:
      transform 160ms ease,
      box-shadow 160ms ease;
  }

  .grid-cursor .tile {
    background-color: #161616 !important;
    aspect-ratio: 140 / 102;
  }

  .tile:hover {
    transform: translateY(-3px);
  }

  .tile.selected {
    outline: 3px solid #fff;
    outline-offset: -3px;
    animation: pop 220ms ease-out;
  }

  @keyframes pop {
    50% {
      transform: scale(1.06);
    }
  }

  .tile img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    background: #161616;
  }

  .cursor {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
  }

  .cursor :global(svg) {
    width: 62%;
    height: 78%;
  }

  .label {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }

  .state {
    position: absolute;
    right: 6px;
    bottom: 6px;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--black);
    color: var(--ink);
    font-size: 13px;
    font-weight: 800;
  }

  .setting {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    max-width: 560px;
    min-height: 52px;
    margin-bottom: 8px;
    padding: 8px 16px;
    border-radius: var(--radius-md);
    background: var(--black);
    color: var(--ink);
    font-size: var(--text-small);
  }

  .setting-name {
    font-weight: 700;
  }

  .setting a {
    color: var(--ink);
  }

  .seg {
    display: flex;
    gap: 4px;
  }

  .seg button {
    min-width: 56px;
    min-height: 36px;
    border: 1px solid #343434;
    border-radius: 999px;
    background: transparent;
    color: var(--ink);
    font: inherit;
    font-weight: 700;
    cursor: pointer;
  }

  .seg button[aria-pressed='true'] {
    background: var(--ink);
    color: var(--black);
  }

  .switch {
    position: relative;
    width: 52px;
    height: 30px;
    border: 0;
    border-radius: 999px;
    background: #3a3a3a;
    cursor: pointer;
    transition: background 160ms ease;
  }

  .switch[aria-checked='true'] {
    background: var(--ink);
  }

  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: #fff;
    transition: transform 160ms ease;
  }

  .switch[aria-checked='true'] .knob {
    background: var(--black);
    transform: translateX(22px);
  }

  @media (prefers-reduced-motion: reduce) {
    .pane,
    .tile,
    .tile.selected,
    .switch,
    .knob {
      animation: none;
      transition: none;
    }

    .tile:hover {
      transform: none;
    }
  }

  :global(:root[data-reduce-motion='true'] *),
  :global(:root[data-reduce-motion='true'] *::before),
  :global(:root[data-reduce-motion='true'] *::after) {
    animation: none !important;
    transition: none !important;
  }
</style>
