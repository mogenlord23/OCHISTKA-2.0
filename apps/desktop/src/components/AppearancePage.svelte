<script lang="ts">
  import { onMount } from 'svelte';
  import Check from './Check.svelte';
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
      hint: 'Акцент меняется во всей Windows. Ещё один клик по выбранному цвету вернёт прежний. Прозрачность и заливка панели задач включаются флажками ниже, вместе они не работают.',
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
    /** Взаимоисключающий флажок: гаснет сразу, без отдельной команды (бэкенд делает это сам). */
    also: Partial<Optimistic> = {},
  ) {
    message = null;
    const mine = key ? ++tokens[key] : 0;
    if (key && value !== undefined) (optimistic as Record<string, unknown>)[key] = value;
    const partners = Object.keys(also) as (keyof Optimistic)[];
    const partnerTokens = partners.map((name) => ++tokens[name]);
    for (const name of partners) (optimistic as Record<string, unknown>)[name] = also[name];
    const settle = () => {
      if (key && tokens[key] === mine) delete optimistic[key];
      partners.forEach((name, i) => {
        if (tokens[name] === partnerTokens[i]) delete optimistic[name];
      });
    };
    queue = queue.then(async () => {
      // Команда, которую уже перебил более новый клик по тому же выбору, в Windows не уходит:
      // оформление пишется один раз, сразу в итоговое значение, и интерфейс не отстаёт.
      if (key && value !== undefined && tokens[key] !== mine) return;
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
    optimisticRun(
      'transparency',
      enabled,
      () => setTransparency(enabled),
      enabled ? { fill: false } : {},
    );
  }

  function toggleFill() {
    const enabled = !fillOn;
    optimisticRun(
      'fill',
      enabled,
      () => setFillTaskbar(enabled),
      enabled ? { transparency: false } : {},
    );
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
          <div class="checks">
            <Check controlled checked={transparencyOn} onchange={toggleTransparency}>
              <span class="check-text">Эффект прозрачности</span>
            </Check>
            <Check controlled checked={fillOn} onchange={toggleFill}>
              <span class="check-text">Залить панель задач и Пуск</span>
            </Check>
          </div>
          <div class="actions">
            <button
              type="button"
              aria-disabled={!status?.hasBackup}
              class:disabled={!status?.hasBackup}
              onclick={restoreWindows}>Вернуть оформление Windows</button
            >
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
                {#if tile.id.startsWith('solid-')}
                  <!-- Узор фона приложения поверх цвета: так же рисуются сами обои -->
                  <span class="pattern" aria-hidden="true"></span>
                {:else if tile.thumb}
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

  .actions {
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

  .actions button.disabled {
    color: var(--ink-soft);
    cursor: not-allowed;
  }

  .more {
    display: flex;
    justify-content: center;
    margin: 8px 0;
  }

  /* Флажки одним столбцом по левому краю страницы: квадрат без отступа слева, текст по его
     центру. Состояние показывается в тот же кадр, без плавного перехода. */
  .checks {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
    margin: 0 0 12px;
    font-size: var(--text-small);
    font-weight: 700;
  }

  .checks :global(.box),
  .checks :global(svg) {
    transition: none;
  }

  .check-text {
    line-height: 1.2;
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
    border-radius: var(--radius-lg);
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
    border-radius: var(--radius-md);
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

  /* Узор фона приложения на плитке сплошных обоев (тот же pop-pattern, чуть заметнее). */
  .pattern {
    position: absolute;
    inset: 0;
    background: url('../assets/pop-pattern.png') 0 0 / 48px 48px;
    opacity: 0.14;
    pointer-events: none;
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
