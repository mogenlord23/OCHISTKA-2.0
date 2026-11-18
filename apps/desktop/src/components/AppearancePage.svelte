<script lang="ts">
  import { onMount } from 'svelte';
  import brushesUrl from '../assets/brushes/brushes-a.svg';
  import { cursorSvg, CURSOR_DEFS, tilesFor, type AppearanceKind } from '../lib/appearance';
  import {
    appearanceErrorMessage,
    appearanceStatus,
    applyCursorScheme,
    applyWallpaperTile,
    downloadOriginals,
    isCancelled,
    pickAndApplyWallpaper,
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
  import { thumbBase64 } from '../lib/appearanceWallpapers';
  import { i18n, t, type Key } from '../lib/i18n.svelte';

  let { subsection }: { subsection: string } = $props();

  const PAGE_SIZE = 40;

  const KIND_BY_SUBSECTION: Record<string, AppearanceKind> = {
    wallpapers: 'wallpaper',
    cursors: 'cursor',
    colors: 'color',
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
  /** Анимация курсора в превью: не при «Меньше анимации» и не при системной настройке. */
  const animateCursors = $derived(
    !reduceMotion &&
      !(typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches),
  );
  /** Есть что вернуть: прежнее значение записано до первого изменения. */
  const canRestore = $derived(
    kind === 'wallpaper'
      ? status?.hasPreviousWallpaper === true
      : kind === 'cursor'
        ? !!status?.cursorScheme
        : status?.hasBackup === true || !!status?.accentTile,
  );

  /** Клик по новой плитке ставит значение в Windows. Клик по уже выбранной ничего не делает. */
  async function pick(id: string) {
    if (busy || id === selectedId) return;
    const tile = tiles.find((item) => item.id === id);
    if (!tile) return;
    busy = true;
    message = null;
    try {
      if (kind === 'wallpaper') {
        status = await applyWallpaperTile(
          id,
          tile.thumb ? undefined : tile.preview,
          tile.thumb ? await thumbBase64(tile.thumb) : undefined,
        );
      } else if (kind === 'cursor') {
        const def = cursorById.get(id);
        if (!def) return;
        status = await applyCursorScheme(id, def.color, def.secondary, def.pattern);
      } else {
        status = await setAccent(tile.preview);
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

  /** Единственный способ вернуть прежнее значение: кнопка «Вернуть как было». */
  function restorePrevious(event: Event) {
    if (!canRestore) return;
    if (kind === 'wallpaper') void act(restoreWallpaper);
    else if (kind === 'cursor') void act(restoreCursorScheme);
    else
      ask(
        {
          title: t('appearance.restoreDialog.title'),
          text: t('appearance.restoreDialog.text'),
          confirm: t('appearance.dialog.restore'),
          run: restoreAppearance,
        },
        event.currentTarget,
      );
  }

  function toggleMotion() {
    reduceMotion = !reduceMotion;
    saveSettings({ reduceMotion });
    applyReduceMotion(reduceMotion);
  }

  async function refreshStatus() {
    try {
      status = await appearanceStatus(tilesFor('wallpaper').find((t) => t.thumb)?.id);
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

  const keep = (key: Key) => `${t(key)} ${t('appearance.keep')}`;
  const change = () => t('appearance.dialog.change');

  function askTransparency(event: Event) {
    const enabled = !(status?.transparency ?? true);
    ask(
      {
        title: t(enabled ? 'appearance.transparencyOn.title' : 'appearance.transparencyOff.title'),
        text: keep('appearance.transparency.text'),
        confirm: change(),
        run: () => setTransparency(enabled),
      },
      event.currentTarget,
    );
  }

  function askFill(event: Event) {
    const enabled = !(status?.fillTaskbar ?? false);
    ask(
      {
        title: t(enabled ? 'appearance.fillOn.title' : 'appearance.fillOff.title'),
        text: keep(enabled ? 'appearance.fillOn.text' : 'appearance.fillOff.text'),
        confirm: change(),
        run: () => setFillTaskbar(enabled),
      },
      event.currentTarget,
    );
  }

  function askStartMode(event: Event) {
    const dark = !(status?.darkSystem ?? false);
    ask(
      {
        title: t(dark ? 'appearance.startDark.title' : 'appearance.startLight.title'),
        text: keep('appearance.start.text'),
        confirm: change(),
        run: () => setSystemTheme(dark),
      },
      event.currentTarget,
    );
  }

  function askTheme(dark: boolean, event: Event) {
    ask(
      {
        title: t(dark ? 'appearance.themeDark.title' : 'appearance.themeLight.title'),
        text: keep('appearance.theme.text'),
        confirm: change(),
        run: () => setTheme(dark),
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

<!-- Заголовок, описание и вкладки рисует общий шаблон страницы (PageShell); здесь только содержимое. -->
<section class="appearance" aria-label={t('appearance.aria')}>
  <span class="brushes" aria-hidden="true"><img src={brushesUrl} alt="" /></span>
  {#key subsection}
    <div class="pane">
      {#if isSettings}
        <p class="hint">{t('appearance.settings.hint')}</p>

        <div class="setting">
          <span class="setting-name" id="lang-label">{t('appearance.settings.language')}</span>
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
          <span class="setting-name" id="motion-label">{t('appearance.settings.motion')}</span>
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
        <p class="hint">{t(`appearance.hint.${kind}`)}</p>

        {#if message}
          <p class="msg error" role="status">{message.text}</p>
        {/if}

        {#if kind === 'color'}
          <div class="chips">
            {@render chip(
              t('appearance.chip.transparency'),
              status?.transparency ?? true,
              askTransparency,
            )}
            {@render chip(t('appearance.chip.fill'), status?.fillTaskbar ?? false, askFill)}
            {@render chip(
              t('appearance.chip.startMode'),
              status?.darkSystem ?? false,
              askStartMode,
            )}
          </div>
          <div class="chips">
            {@render chip(t('appearance.chip.dark'), status?.darkApps === true, (e) =>
              askTheme(true, e),
            )}
            {@render chip(t('appearance.chip.light'), status?.darkApps === false, (e) =>
              askTheme(false, e),
            )}
            {@render chip(t('appearance.restore'), false, restorePrevious, !canRestore)}
          </div>
        {:else}
          <div class="actions">
            {#if kind === 'wallpaper'}
              <button type="button" onclick={() => void act(pickAndApplyWallpaper)}
                >{t('appearance.addWallpaper')}</button
              >
            {/if}
            <button
              type="button"
              aria-disabled={!canRestore}
              class:disabled={!canRestore}
              onclick={restorePrevious}>{t('appearance.restore')}</button
            >
            {#if kind === 'wallpaper' && status?.originalsMissing}
              <button type="button" onclick={() => void act(downloadOriginals)}
                >{t('appearance.downloadOriginals')}</button
              >
            {/if}
          </div>
        {/if}

        <ul class="grid grid-{kind}" class:squares={kind !== 'color'}>
          {#each shown as tile, index (tile.id)}
            {@const isSelected = tile.id === selectedId}
            {@const cursor = cursorById.get(tile.id)}
            <li>
              <button
                type="button"
                class="tile"
                class:selected={isSelected}
                aria-pressed={isSelected}
                aria-label="{t(`appearance.name.${kind}`)} {index + 1}"
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
                    width="160"
                    height="160"
                  />
                {:else if cursor}
                  <!-- eslint-disable-next-line svelte/no-at-html-tags -- собственные данные, не ввод -->
                  <span class="cursor" aria-hidden="true"
                    >{@html cursorSvg(cursor, tile.id, 32, animateCursors)}</span
                  >
                {/if}
              </button>
            </li>
          {/each}
        </ul>
        {#if kind === 'wallpaper' && visible < tiles.length}
          <div bind:this={sentinel} class="more">
            <button type="button" onclick={() => (visible += PAGE_SIZE)}>
              {t('appearance.more')} ({tiles.length - visible})
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
          <button type="button" bind:this={cancelButton} onclick={closeDialog}
            >{t('appearance.dialog.cancel')}</button
          >
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
    position: relative;
    color: var(--page-text);
  }

  /* Кисти в шапке: справа от заголовка, поверх пустого места строки заголовка и описания. */
  .brushes {
    position: absolute;
    top: -128px;
    right: 0;
    width: 150px;
    height: 100px;
    pointer-events: none;
  }

  .brushes img {
    display: block;
    width: 100%;
    height: 100%;
  }

  @media (max-width: 760px) {
    .brushes {
      display: none;
    }
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

  .hint {
    margin: 0 0 16px;
    font-size: var(--text-small);
    font-weight: 700;
  }

  .msg {
    margin: 0 0 12px;
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
    gap: 10px;
    margin-bottom: 12px;
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
    margin: 14px 0;
  }

  /* Строка настроек: тёмные скруглённые плашки с квадратным индикатором, как на экране владельца. */
  .chip {
    display: flex;
    align-items: center;
    gap: 14px;
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

  /* Сетка заполняет всю ширину контейнера, как другие разделы: колонки добавляются по ширине,
     плитки растягиваются поровну, пустых полей по бокам нет. Обои и курсоры квадратные. */
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--tile-min), 1fr));
    gap: 12px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .grid-wallpaper {
    --tile-min: 128px;
  }

  .grid-cursor {
    --tile-min: 96px;
  }

  .grid-color {
    --tile-min: 88px;
  }

  .grid li {
    display: block;
    min-width: 0;
  }

  .tile {
    position: relative;
    display: grid;
    place-items: center;
    width: 100%;
    aspect-ratio: 3 / 2;
    padding: 0;
    overflow: hidden;
    border: 0;
    border-radius: 14px;
    background-position: center;
    background-repeat: no-repeat;
    color: var(--ink);
    font: inherit;
    cursor: pointer;
    backface-visibility: hidden;
    transition: transform 160ms ease;
  }

  .squares .tile {
    aspect-ratio: 1 / 1;
  }

  .grid-cursor .tile {
    background-color: #161616 !important;
  }

  .grid-color .tile {
    border-radius: 12px;
    box-shadow: inset 0 0 0 3px #111;
  }

  /* Наведение: только плавное увеличение, без линий и теней, уходящих в сторону. */
  .tile:hover {
    transform: scale(1.04);
  }

  .tile:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }

  /* Выбранная плитка: одна ровная обводка по краю самой плитки (внутри её границы,
     скругление такое же, как у плитки), без галочки и без анимации. */
  .tile.selected::after {
    content: '';
    position: absolute;
    inset: 0;
    border: 3px solid #fff;
    border-radius: inherit;
    box-shadow: inset 0 0 0 1px rgb(0 0 0 / 45%);
    pointer-events: none;
  }

  /* Квадратное превью: картинка по центру, обрезка cover. */
  .tile img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center;
    background: #161616;
  }

  /* Курсор в реальном размере 32 px: размер не зависит от размера окна. */
  .cursor {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
  }

  .cursor :global(svg) {
    display: block;
    flex: none;
    width: 32px;
    height: 32px;
    shape-rendering: geometricPrecision;
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
