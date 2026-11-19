<script lang="ts">
  import { onMount } from 'svelte';
  import {
    COLOR_SCHEMES,
    cursorSvg,
    CURSOR_DEFS,
    tilesFor,
    type AppearanceKind,
  } from '../lib/appearance';
  import {
    appearanceErrorMessage,
    appearanceStatus,
    applyCursorScheme,
    applyWallpaperTile,
    isCancelled,
    pickAndApplyWallpaper,
    restoreAccent,
    restoreAppearance,
    restoreCursorScheme,
    restoreWallpaper,
    setAccent,
    setFillTaskbar,
    setTransparency,
    type AppearanceStatus,
  } from '../lib/appearanceApi';
  import { applyReduceMotion, loadSettings, saveSettings } from '../lib/appearanceStore';
  import { i18n } from '../lib/i18n.svelte';

  let { subsection }: { subsection: string } = $props();

  const PAGE_SIZE = 40;

  const KIND_BY_SUBSECTION: Record<string, AppearanceKind> = {
    wallpapers: 'wallpaper',
    cursors: 'cursor',
    colors: 'color',
  };

  const ACCESSIBLE_NAME: Record<AppearanceKind, string> = {
    wallpaper: 'Обои',
    cursor: 'Курсор',
    color: 'Цвет',
  };

  const HEADINGS: Record<AppearanceKind, { title: string; hint: string }> = {
    wallpaper: {
      title: 'Обои',
      hint: 'Нажми на картинку, и она станет обоями рабочего стола Windows. Ещё один клик по выбранной вернёт прежние обои.',
    },
    cursor: {
      title: 'Курсоры',
      hint: 'Выбранная схема становится курсором Windows. Ещё один клик по выбранной вернёт прежний курсор.',
    },
    color: {
      title: 'Акцентный цвет Windows',
      hint: 'Акцент меняется во всей Windows. Ещё один клик по выбранному цвету вернёт прежний. Прозрачность и заливка панели задач включаются переключателями ниже.',
    },
  };

  /** Мгновенный выбор: плитка или переключатель выглядят выбранными сразу, система догоняет. */
  interface Optimistic {
    wallpaper?: string | null;
    cursor?: string | null;
    color?: string | null;
    transparency?: boolean;
    fill?: boolean;
  }

  let reduceMotion = $state(loadSettings().reduceMotion);
  let visible = $state(PAGE_SIZE);
  let sentinel = $state<HTMLElement | null>(null);
  let status = $state<AppearanceStatus | null>(null);
  let optimistic = $state<Optimistic>({});
  let message = $state<{ kind: 'error'; text: string } | null>(null);
  /** Команды идут в Windows по одной, в порядке кликов, а интерфейс не ждёт. */
  let queue: Promise<void> = Promise.resolve();
  const tokens: Record<keyof Optimistic, number> = {
    wallpaper: 0,
    cursor: 0,
    color: 0,
    transparency: 0,
    fill: 0,
  };

  const isSettings = $derived(subsection === 'settings');
  const kind = $derived<AppearanceKind>(KIND_BY_SUBSECTION[subsection] ?? 'wallpaper');
  const tiles = $derived(tilesFor(kind));
  const accentTileId = (accent: string | null | undefined) =>
    COLOR_SCHEMES.find((tile) => tile.preview.toUpperCase() === accent)?.id ?? null;
  /** Выбор читается из Windows (поставленные из OCHISTKA обои, схема курсора и акцент). */
  const selectedId = $derived(
    kind === 'wallpaper'
      ? optimistic.wallpaper !== undefined
        ? optimistic.wallpaper
        : status?.wallpaperTile
      : kind === 'cursor'
        ? optimistic.cursor !== undefined
          ? optimistic.cursor
          : status?.cursorScheme
        : optimistic.color !== undefined
          ? optimistic.color
          : accentTileId(status?.accentTile),
  );
  const transparencyOn = $derived(optimistic.transparency ?? status?.transparencyOn ?? false);
  const fillOn = $derived(optimistic.fill ?? status?.fillOn ?? false);
  const canRestoreWallpaper = $derived(
    Boolean(status?.hasPreviousWallpaper) || (optimistic.wallpaper ?? null) !== null,
  );
  const shown = $derived(kind === 'wallpaper' ? tiles.slice(0, visible) : tiles);
  const cursorById = new Map(CURSOR_DEFS.map((def) => [def.id, def.cursor]));

  /**
   * Мгновенно ставит `optimistic[key]`, потом в фоне выполняет команду. Успех: берём статус из
   * ответа. Ошибка: убираем мгновенный выбор (рамка возвращается) и показываем короткое сообщение.
   * Отмена окна выбора файла ошибкой не считается.
   */
  function optimisticRun(
    key: keyof Optimistic | null,
    value: Optimistic[keyof Optimistic] | undefined,
    run: () => Promise<AppearanceStatus>,
  ) {
    message = null;
    const mine = key ? ++tokens[key] : 0;
    if (key && value !== undefined) (optimistic as Record<string, unknown>)[key] = value;
    const settle = () => {
      if (key && tokens[key] === mine) delete optimistic[key];
    };
    queue = queue.then(async () => {
      try {
        status = await run();
      } catch (err) {
        if (!isCancelled(err)) message = { kind: 'error', text: appearanceErrorMessage(err) };
      } finally {
        settle();
      }
    });
  }

  /** Клик по плитке: рамка сразу; повторный клик по выбранной возвращает прежнее. */
  function pick(id: string) {
    const tile = tiles.find((item) => item.id === id);
    if (!tile) return;
    const selected = id === selectedId;
    if (kind === 'wallpaper') {
      optimisticRun('wallpaper', selected ? null : id, () =>
        selected ? restoreWallpaper() : applyWallpaperTile(id),
      );
    } else if (kind === 'cursor') {
      const def = cursorById.get(id);
      if (!def) return;
      optimisticRun('cursor', selected ? null : id, () =>
        selected
          ? restoreCursorScheme()
          : applyCursorScheme(id, def.color, def.secondary, def.pattern),
      );
    } else {
      optimisticRun('color', selected ? null : id, () =>
        selected ? restoreAccent() : setAccent(tile.preview),
      );
    }
  }

  function toggleTransparency() {
    const enabled = !transparencyOn;
    optimisticRun('transparency', enabled, () => setTransparency(enabled));
  }

  function toggleFill() {
    const enabled = !fillOn;
    optimisticRun('fill', enabled, () => setFillTaskbar(enabled));
  }

  function restoreWindows() {
    if (!status?.hasBackup) return;
    optimistic.color = null;
    optimistic.transparency = false;
    optimistic.fill = false;
    for (const key of ['color', 'transparency', 'fill'] as const) tokens[key] += 1;
    const mine = { ...tokens };
    message = null;
    queue = queue.then(async () => {
      try {
        status = await restoreAppearance();
      } catch (err) {
        if (!isCancelled(err)) message = { kind: 'error', text: appearanceErrorMessage(err) };
      } finally {
        for (const key of ['color', 'transparency', 'fill'] as const) {
          if (tokens[key] === mine[key]) delete optimistic[key];
        }
      }
    });
  }

  function toggleMotion() {
    reduceMotion = !reduceMotion;
    saveSettings({ reduceMotion });
    applyReduceMotion(reduceMotion);
  }

  async function refreshStatus() {
    try {
      status = await appearanceStatus();
    } catch {
      status = null;
    }
  }

  onMount(() => {
    void refreshStatus();
  });

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

{#snippet chip(label: string, on: boolean, onclick: (e: Event) => void, disabled = false)}
  <button
    type="button"
    class="chip"
    aria-pressed={on}
    aria-disabled={disabled}
    class:disabled
    onclick={(event) => {
      if (!disabled) onclick(event);
    }}
  >
    <span class="box" class:on aria-hidden="true"></span>
    <span class="chip-text">{label}</span>
  </button>
{/snippet}

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
      {:else}
        <h2>{HEADINGS[kind].title}</h2>
        <p class="hint">{HEADINGS[kind].hint}</p>

        {#if message}
          <p class="msg error" role="status">{message.text}</p>
        {/if}

        {#if kind === 'wallpaper'}
          <div class="actions">
            <button
              type="button"
              onclick={() => optimisticRun('wallpaper', undefined, pickAndApplyWallpaper)}
              >Добавить свои обои…</button
            >
            <button
              type="button"
              aria-disabled={!canRestoreWallpaper}
              class:disabled={!canRestoreWallpaper}
              onclick={() => {
                if (canRestoreWallpaper) optimisticRun('wallpaper', null, restoreWallpaper);
              }}>Вернуть прежний фон</button
            >
          </div>
        {:else if kind === 'color'}
          <div class="chips">
            {@render chip('Эффект прозрачности', transparencyOn, toggleTransparency)}
            {@render chip('Залить панель задач и Пуск', fillOn, toggleFill)}
            {@render chip('Вернуть оформление Windows', false, restoreWindows, !status?.hasBackup)}
          </div>
        {/if}

        <ul class="grid grid-{kind}">
          {#each shown as tile, index (tile.id)}
            {@const isSelected = tile.id === selectedId}
            {@const cursor = cursorById.get(tile.id)}
            <li>
              <button
                type="button"
                class="tile"
                class:selected={isSelected}
                aria-pressed={isSelected}
                aria-label="{ACCESSIBLE_NAME[kind]} {index + 1}"
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
                  <span class="cursor" aria-hidden="true"
                    >{@html cursorSvg(cursor, tile.id, 64)}</span
                  >
                  <!-- Превью анимации «Занят»: бегущая дуга второго цвета крутится при наведении -->
                  <span
                    class="spin"
                    aria-hidden="true"
                    style:--spin={cursor.pattern === 'neon' ? cursor.color : cursor.secondary}
                  ></span>
                {/if}
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
    margin: 0 0 8px;
    font-size: var(--text-small);
  }

  .msg {
    margin: 0 0 8px;
    font-size: var(--text-small);
    font-weight: 700;
  }

  .msg.error {
    display: inline-block;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: #ff8a8a;
  }

  .actions,
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 8px;
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
    cursor: pointer;
  }

  .actions button.disabled,
  .chip.disabled {
    color: var(--ink-soft);
    cursor: not-allowed;
  }

  .more {
    display: flex;
    justify-content: center;
    margin: 8px 0;
  }

  /* Строка настроек: тёмные скруглённые плашки с квадратным индикатором, как на экране владельца. */
  .chip {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 52px;
    padding: 8px 20px 8px 14px;
    border: 2px solid #2a2a2a;
    border-radius: 14px;
    background: #121212;
    color: var(--ink);
    font: inherit;
    font-size: var(--text-small);
    font-weight: 800;
    text-align: left;
    cursor: pointer;
    transition: transform 140ms ease;
  }

  .chip:hover:not(.disabled) {
    transform: translateY(-2px);
  }

  /* Нажатие: плашка на миг «вдавливается». Диалогов нет, клик и есть действие. */
  .chip:active:not(.disabled) {
    transform: scale(0.96);
  }

  .chip:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }

  .box {
    flex: none;
    width: 26px;
    height: 26px;
    border: 3px solid #4a4a4a;
    border-radius: 8px;
    background: #1c1c1c;
    box-shadow: inset 0 0 0 3px #1c1c1c;
  }

  .box.on {
    border-color: var(--accent, #f83c5c);
    background: var(--accent, #f83c5c);
  }

  /* Плитки фиксированного размера: на любом экране одинаковые. Сетка добавляет колонки и
     стоит по центру общей области раздела. */
  .grid {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .grid-wallpaper {
    grid-template-columns: repeat(auto-fill, minmax(176px, 1fr));
    --tile-ratio: 16 / 9;
  }

  .grid-cursor {
    grid-template-columns: repeat(auto-fill, minmax(128px, 1fr));
    --tile-ratio: 128 / 94;
  }

  .grid-color {
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    --tile-ratio: 3 / 2;
  }

  .grid li {
    display: flex;
    aspect-ratio: var(--tile-ratio);
    min-width: 0;
  }

  .tile {
    position: relative;
    display: flex;
    flex: 1;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
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
    transition: transform 160ms ease;
  }

  .grid-cursor .tile {
    background-color: #161616 !important;
  }

  .grid-color .tile {
    border-radius: 12px;
    box-shadow: inset 0 0 0 3px #111;
  }

  .tile:hover {
    transform: scale(1.05);
  }

  .tile:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }

  /* Выбранная плитка: белая скруглённая рамка внутри плитки, без галочки и без анимации. */
  .tile.selected::after {
    content: '';
    position: absolute;
    inset: 0;
    border: 3px solid #fff;
    border-radius: inherit;
    box-shadow: inset 0 0 0 1px rgb(0 0 0 / 45%);
    pointer-events: none;
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

  .spin {
    position: absolute;
    right: 8px;
    bottom: 8px;
    width: 16px;
    height: 16px;
    border: 3px solid rgb(255 255 255 / 18%);
    border-top-color: var(--spin, #fff);
    border-radius: 50%;
    animation: spin 700ms linear infinite paused;
  }

  .tile:hover .spin {
    animation-play-state: running;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .cursor :global(svg) {
    display: block;
    width: 64px;
    height: 64px;
  }

  .setting {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
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
    .chip,
    .spin,
    .switch,
    .knob {
      animation: none;
      transition: none;
    }

    .tile:hover,
    .chip:hover:not(.disabled),
    .chip:active:not(.disabled) {
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
