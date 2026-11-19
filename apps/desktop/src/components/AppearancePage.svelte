<script lang="ts">
  import { onMount } from 'svelte';
  import { cursorSvg, CURSOR_DEFS, tilesFor, type AppearanceKind } from '../lib/appearance';
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
    setSystemTheme,
    setTheme,
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
      hint: 'Акцент меняется во всей Windows. Ещё один клик по выбранному цвету вернёт прежний. Прозрачность и светлый или тёмный режим настраиваются отдельно.',
    },
  };

  /** Действие, ожидающее подтверждения: текст диалога и сама операция. */
  interface Pending {
    title: string;
    text: string;
    confirm: string;
    run: () => Promise<AppearanceStatus>;
  }

  let busy = false;
  let reduceMotion = $state(loadSettings().reduceMotion);
  let visible = $state(PAGE_SIZE);
  let sentinel = $state<HTMLElement | null>(null);
  let status = $state<AppearanceStatus | null>(null);
  let pending = $state<Pending | null>(null);
  let message = $state<{ kind: 'error'; text: string } | null>(null);
  let cancelButton = $state<HTMLButtonElement | null>(null);
  let dialogReturnFocus: HTMLElement | null = null;

  const isSettings = $derived(subsection === 'settings');
  const kind = $derived<AppearanceKind>(KIND_BY_SUBSECTION[subsection] ?? 'wallpaper');
  const tiles = $derived(tilesFor(kind));
  /** Выбор читается из Windows: поставленные из OCHISTKA обои, схема курсора и акцент. */
  const selectedId = $derived(
    kind === 'wallpaper'
      ? status?.wallpaperTile
      : kind === 'cursor'
        ? status?.cursorScheme
        : tiles.find((tile) => tile.preview.toUpperCase() === status?.accentTile)?.id,
  );
  const shown = $derived(kind === 'wallpaper' ? tiles.slice(0, visible) : tiles);
  const cursorById = new Map(CURSOR_DEFS.map((def) => [def.id, def.cursor]));

  /** Клик по плитке ставит значение в Windows, повторный клик по выбранной возвращает прежнее. */
  async function pick(id: string) {
    if (busy) return;
    const tile = tiles.find((item) => item.id === id);
    if (!tile) return;
    const selected = id === selectedId;
    busy = true;
    message = null;
    try {
      if (kind === 'wallpaper') {
        status = selected ? await restoreWallpaper() : await applyWallpaperTile(id);
      } else if (kind === 'cursor') {
        const def = cursorById.get(id);
        if (!def) return;
        status = selected
          ? await restoreCursorScheme()
          : await applyCursorScheme(id, def.color, def.secondary, def.pattern);
      } else {
        status = selected ? await restoreAccent() : await setAccent(tile.preview);
      }
    } catch (err) {
      if (!isCancelled(err)) message = { kind: 'error', text: appearanceErrorMessage(err) };
    } finally {
      busy = false;
    }
  }

  /** Действия без вопросов: сам клик по кнопке и есть действие, всё обратимо. */
  async function act(run: () => Promise<AppearanceStatus>) {
    if (busy) return;
    busy = true;
    message = null;
    try {
      status = await run();
    } catch (err) {
      if (!isCancelled(err)) message = { kind: 'error', text: appearanceErrorMessage(err) };
    } finally {
      busy = false;
    }
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

  function ask(item: Pending, from?: EventTarget | null) {
    message = null;
    dialogReturnFocus = from instanceof HTMLElement ? from : null;
    pending = item;
  }

  function closeDialog() {
    pending = null;
    queueMicrotask(() => dialogReturnFocus?.focus());
  }

  async function confirmPending() {
    const item = pending;
    if (!item) return;
    closeDialog();
    try {
      status = await item.run();
    } catch (err) {
      if (!isCancelled(err)) message = { kind: 'error', text: appearanceErrorMessage(err) };
    }
  }

  const KEEP =
    'Прежнее значение сохранится, его можно вернуть кнопкой «Вернуть оформление Windows».';

  function askTransparency(event: Event) {
    const enabled = !(status?.transparency ?? true);
    ask(
      {
        title: enabled ? 'Включить эффект прозрачности?' : 'Выключить эффект прозрачности?',
        text: `Изменится настройка прозрачности Windows. ${KEEP}`,
        confirm: 'Изменить',
        run: () => setTransparency(enabled),
      },
      event.currentTarget,
    );
  }

  function askFill(event: Event) {
    const enabled = !(status?.fillTaskbar ?? false);
    ask(
      {
        title: enabled ? 'Залить Пуск и панель задач?' : 'Убрать заливку Пуска и панели задач?',
        text: `Акцентный цвет ${enabled ? 'появится' : 'уберётся'} на Пуске и панели задач. ${KEEP}`,
        confirm: 'Изменить',
        run: () => setFillTaskbar(enabled),
      },
      event.currentTarget,
    );
  }

  function askStartMode(event: Event) {
    const dark = !(status?.darkSystem ?? false);
    ask(
      {
        title: dark ? 'Тёмный режим для Пуска?' : 'Светлый режим для Пуска?',
        text: `Изменится тема Пуска и панели задач. Приложения не затрагиваются. ${KEEP}`,
        confirm: 'Изменить',
        run: () => setSystemTheme(dark),
      },
      event.currentTarget,
    );
  }

  function askTheme(dark: boolean, event: Event) {
    ask(
      {
        title: dark ? 'Включить тёмный режим?' : 'Включить светлый режим?',
        text: `Изменится тема приложений, Пуска и панели задач Windows. ${KEEP}`,
        confirm: 'Изменить',
        run: () => setTheme(dark),
      },
      event.currentTarget,
    );
  }

  function askRestore(event: Event) {
    if (!status?.hasBackup) return;
    ask(
      {
        title: 'Вернуть оформление Windows?',
        text: 'Цвета, прозрачность и тема вернутся к значениям, которые были до первого изменения из OCHISTKA.',
        confirm: 'Вернуть',
        run: restoreAppearance,
      },
      event.currentTarget,
    );
  }

  function onDialogKey(event: KeyboardEvent) {
    if (event.key === 'Escape') closeDialog();
  }

  $effect(() => {
    if (pending && cancelButton) cancelButton.focus();
  });

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
            <button type="button" onclick={() => void act(pickAndApplyWallpaper)}
              >Добавить свои обои…</button
            >
            <button
              type="button"
              aria-disabled={!status?.hasPreviousWallpaper}
              class:disabled={!status?.hasPreviousWallpaper}
              onclick={() => {
                if (status?.hasPreviousWallpaper) void act(restoreWallpaper);
              }}>Вернуть прежний фон</button
            >
          </div>
        {:else if kind === 'color'}
          <div class="chips">
            {@render chip('Эффект прозрачности', status?.transparency ?? true, askTransparency)}
            {@render chip('Залить Пуск и всю панель задач', status?.fillTaskbar ?? false, askFill)}
            {@render chip('Режим для Пуска', status?.darkSystem ?? false, askStartMode)}
          </div>
          <div class="chips">
            {@render chip('Тёмный режим', status?.darkApps === true, (e) => askTheme(true, e))}
            {@render chip('Светлый режим', status?.darkApps === false, (e) => askTheme(false, e))}
            {@render chip('Вернуть оформление Windows', false, askRestore, !status?.hasBackup)}
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
                onclick={() => void pick(tile.id)}
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

  {#if pending}
    <div class="backdrop">
      <div
        class="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="appearance-dialog-title"
        aria-describedby="appearance-dialog-text"
        tabindex="-1"
        onkeydown={onDialogKey}
      >
        <h3 id="appearance-dialog-title">{pending.title}</h3>
        <p id="appearance-dialog-text">{pending.text}</p>
        <div class="dialog-actions">
          <button type="button" bind:this={cancelButton} onclick={closeDialog}>Отмена</button>
          <button type="button" class="primary" onclick={() => void confirmPending()}
            >{pending.confirm}</button
          >
        </div>
      </div>
    </div>
  {/if}
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
  .more button,
  .dialog button {
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

  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.6);
  }

  .dialog {
    width: min(440px, calc(100vw - 32px));
    padding: 20px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--ink);
    animation: dialog-in 160ms cubic-bezier(0.2, 0, 0, 1);
  }

  .dialog h3 {
    margin: 0 0 8px;
  }

  .dialog p {
    margin: 0;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }

  .dialog button.primary {
    border-color: transparent;
    background: var(--ink);
    color: var(--black);
  }

  @keyframes dialog-in {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .pane,
    .dialog,
    .tile,
    .chip,
    .switch,
    .knob {
      animation: none;
      transition: none;
    }

    .tile:hover,
    .chip:hover:not(.disabled) {
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
