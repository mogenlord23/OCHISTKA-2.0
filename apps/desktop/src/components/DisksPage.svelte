<script lang="ts">
  import { onMount } from 'svelte';
  import { getSystemOverview } from '../lib/api';
  import poster from '../assets/pop-art.png';
  import { t } from '../lib/i18n.svelte';
  import { playHeehee } from '../lib/sound';
  import {
    dateLocale,
    driveLetter,
    formatSize,
    kindLabel,
    localizeCategoryTitle,
    modelLabel,
    tf,
  } from '../lib/disks';
  import { SECTION_COLORS } from '../lib/tokens';
  import DiskIcon from './DiskIcon.svelte';
  import type { DiskStatus } from '../lib/types';
  import {
    analyzeCleanup,
    cleanSelected,
    largeFiles,
    listDrives,
    listQuarantine,
    privacyReport,
    purgeItem,
    restoreItem,
    spaceReport,
    stopAnalysis,
    toDisksError,
    type CleanupCategory,
    type DisksErrorInfo,
    type FolderSize,
    type LargeFile,
    type MoveOutcome,
    type PrivacyItem,
    type QuarantineEntry,
  } from '../lib/disksApi';

  let { subsection }: { subsection: string } = $props();

  const SUB_KEYS = {
    cleanup: 'sub.disks.cleanup',
    files: 'sub.disks.files',
    space: 'sub.disks.space',
    privacy: 'sub.disks.privacy',
  } as const;
  /** Единый заголовок подраздела: «ДИСКИ / <подраздел>» (регистр задаёт CSS). */
  const heading = $derived(
    `${t('section.disks')} / ${
      subsection in SUB_KEYS ? t(SUB_KEYS[subsection as keyof typeof SUB_KEYS]) : ''
    }`,
  );
  /** Цвета квадратных иконок плиток по кругу, из палитры разделов. */
  const TILE_COLORS = [
    SECTION_COLORS.disks,
    SECTION_COLORS.viruses,
    SECTION_COLORS.vpn,
    SECTION_COLORS.processes,
  ];

  /** Плитки дисков: только чтение сведений о заполнении. Нет ответа — плиток нет. */
  let disks = $state<DiskStatus[]>([]);
  onMount(() => {
    getSystemOverview()
      .then((overview) => (disks = overview.disks))
      .catch(() => (disks = []));
  });

  type Running = 'cleanup' | 'files' | 'space' | 'privacy' | null;
  let running = $state<Running>(null);
  let error = $state<DisksErrorInfo | null>(null);
  let notice = $state('');

  // Глубокая очистка
  let categories = $state<CleanupCategory[]>([]);
  let selected = $state<Record<string, boolean>>({});
  let outcome = $state<MoveOutcome | null>(null);
  let quarantine = $state<QuarantineEntry[]>([]);
  let alternatives = $state<Record<string, string>>({});
  let occupiedId = $state<string | null>(null);
  let confirmId = $state<string | null>(null);

  // Очистка файлов
  let files = $state<LargeFile[]>([]);
  let filesScanned = $state(0);
  let filesComplete = $state(true);
  let filesLoaded = $state(false);

  // Анализ пространства
  let drives = $state<string[]>([]);
  let drive = $state('');
  let folders = $state<FolderSize[]>([]);
  let looseBytes = $state(0);
  let spaceComplete = $state(true);
  let spaceLoaded = $state(false);

  // Приватность
  let privacy = $state<PrivacyItem[]>([]);
  let privacyLoaded = $state(false);

  const chosenIds = $derived(
    categories
      .filter((item) => selected[item.id] && item.found && item.fileCount > 0)
      .map((item) => item.id),
  );
  const maxFolderBytes = $derived(Math.max(1, ...folders.map((item) => item.bytes)));

  function fail(err: unknown) {
    error = toDisksError(err);
  }

  async function refreshQuarantine() {
    try {
      quarantine = await listQuarantine();
    } catch (err) {
      fail(err);
    }
  }

  onMount(async () => {
    await refreshQuarantine();
    try {
      drives = await listDrives();
      drive = drives[0] ?? '';
    } catch (err) {
      fail(err);
    }
  });

  async function analyze() {
    running = 'cleanup';
    error = null;
    notice = '';
    outcome = null;
    try {
      categories = await analyzeCleanup();
      selected = {};
    } catch (err) {
      fail(err);
    } finally {
      running = null;
    }
  }

  async function clean() {
    running = 'cleanup';
    error = null;
    try {
      outcome = await cleanSelected(chosenIds);
      // План расходуется при переносе: прежние размеры уже неверны.
      categories = [];
      selected = {};
      notice = t('disks.notice.cleaned');
      await refreshQuarantine();
    } catch (err) {
      fail(err);
    } finally {
      running = null;
    }
  }

  async function restore(entry: QuarantineEntry) {
    error = null;
    const alt = (alternatives[entry.id] ?? '').trim();
    try {
      await restoreItem(entry.id, alt === '' ? null : alt);
      occupiedId = null;
      notice = t('disks.notice.restored');
      await refreshQuarantine();
    } catch (err) {
      const info = toDisksError(err);
      if (info.kind === 'occupied') {
        occupiedId = entry.id;
      }
      error = info;
    }
  }

  async function purge(entry: QuarantineEntry) {
    error = null;
    try {
      await purgeItem(entry.id, entry.purgeToken);
      confirmId = null;
      notice = t('disks.notice.purged');
      await refreshQuarantine();
    } catch (err) {
      fail(err);
    }
  }

  async function stop() {
    try {
      await stopAnalysis();
    } catch (err) {
      fail(err);
    }
  }

  async function loadFiles() {
    running = 'files';
    error = null;
    try {
      const report = await largeFiles();
      files = report.files;
      filesScanned = report.scanned;
      filesComplete = report.complete;
      filesLoaded = true;
    } catch (err) {
      fail(err);
    } finally {
      running = null;
    }
  }

  async function loadSpace() {
    running = 'space';
    error = null;
    try {
      const report = await spaceReport(drive);
      folders = report.folders;
      looseBytes = report.looseBytes;
      spaceComplete = report.complete;
      spaceLoaded = true;
    } catch (err) {
      fail(err);
    } finally {
      running = null;
    }
  }

  async function loadPrivacy() {
    running = 'privacy';
    error = null;
    try {
      privacy = await privacyReport();
      privacyLoaded = true;
    } catch (err) {
      fail(err);
    } finally {
      running = null;
    }
  }

  function formatDate(unix: number): string {
    return unix > 0 ? new Date(unix * 1000).toLocaleDateString(dateLocale()) : '—';
  }

  const tail = (complete: boolean) => (complete ? '' : t('disks.incomplete'));

  /** Пасхалка: звук (если включён) и анимация нажатия. */
  let pressed = $state(false);
  function onPoster() {
    pressed = false;
    requestAnimationFrame(() => (pressed = true));
    playHeehee();
  }
</script>

<h2 class="page-heading">{heading}</h2>
{#if disks.length > 0}
  <ul class="disk-tiles" aria-label={t('disks.tiles.aria')}>
    {#each disks as disk, index (disk.mountPoint)}
      <li class="disk-tile">
        <span
          class="swatch"
          style:background={TILE_COLORS[index % TILE_COLORS.length]}
          aria-hidden="true"><DiskIcon kind={disk.kind} /></span
        >
        <span class="disk-info">
          <span class="disk-letter">{driveLetter(disk.mountPoint)}</span>
          <span class="disk-meta">
            <span class="disk-kind">{kindLabel(disk.kind)}</span>
            <span class="disk-model">{modelLabel(disk.model)}</span>
          </span>
          <dl class="disk-grid">
            <dt>{t('disks.capacity')}</dt>
            <dd>{formatSize(disk.totalBytes)}</dd>
            <dt>{t('disks.free')}</dt>
            <dd>{formatSize(Math.max(0, disk.totalBytes - disk.usedBytes))}</dd>
          </dl>
          <span class="disk-bar"
            ><span class="disk-fill" style:width="{disk.usagePercent}%"></span></span
          >
          <span class="disk-used"
            >{tf('disks.used', { percent: Math.round(disk.usagePercent) })}</span
          >
        </span>
      </li>
    {/each}
  </ul>
{/if}
{#if error}
  <div class="alert" role="alert"><p>{error.message}</p></div>
{/if}
{#if notice}
  <p class="notice" role="status">{notice}</p>
{/if}

{#if subsection === 'cleanup'}
  <div class="cleanup-layout">
    <div class="cleanup-main">
      <section class="panel">
        <div class="actions">
          <button
            type="button"
            class="primary"
            onclick={() => void analyze()}
            disabled={running !== null}
          >
            {t('disks.analyze')}
          </button>
          {#if running === 'cleanup'}
            <button type="button" onclick={() => void stop()}>{t('disks.stop')}</button>
            <span class="reason" aria-live="polite">{t('disks.running')}</span>
          {/if}
          <button
            type="button"
            class="primary"
            onclick={() => void clean()}
            disabled={running !== null || chosenIds.length === 0}
          >
            {t('disks.cleanSelected')}
          </button>
        </div>
        <p class="hint">{t('disks.quarantineHint')}</p>
      </section>

      {#if categories.length > 0}
        <ul class="rows">
          {#each categories as item (item.id)}
            <li>
              <label>
                <input
                  type="checkbox"
                  bind:checked={selected[item.id]}
                  disabled={!item.found || item.fileCount === 0}
                />
                <span class="title">{localizeCategoryTitle(item.id, item.title)}</span>
              </label>
              <span class="size">
                {#if !item.found}
                  {t('disks.notFound')}
                {:else}
                  {tf('disks.sizeFiles', {
                    size: formatSize(item.bytes),
                    n: item.fileCount,
                    tail: tail(item.complete),
                  })}
                {/if}
              </span>
            </li>
          {/each}
        </ul>
      {/if}

      {#if outcome}
        <p class="notice" role="status">
          {tf('disks.moved', {
            moved: outcome.moved,
            size: formatSize(outcome.bytes),
            failed: outcome.failed > 0 ? tf('disks.movedFailed', { failed: outcome.failed }) : '',
          })}
        </p>
        {#if outcome.failures.length > 0}
          <ul class="failures">
            {#each outcome.failures as failure (failure.path)}
              <li><span class="path">{failure.path}</span> — {failure.message}</li>
            {/each}
          </ul>
        {/if}
      {/if}

      <h3>{t('disks.quarantine')}</h3>
      {#if quarantine.length === 0}
        <p class="empty">{t('disks.quarantineEmpty')}</p>
      {:else}
        <ul class="rows">
          {#each quarantine as entry (entry.id)}
            <li class="quarantine">
              <div>
                <span class="title">{entry.originalPath}</span>
                <small
                  >{formatSize(entry.size)} · {formatDate(entry.movedAtUnix)} · {entry.reason}</small
                >
              </div>
              <div class="actions">
                <button type="button" onclick={() => void restore(entry)}
                  >{t('disks.restore')}</button
                >
                <button type="button" onclick={() => (confirmId = entry.id)}
                  >{t('disks.purge')}</button
                >
              </div>
              {#if occupiedId === entry.id}
                <label class="alt">
                  {t('disks.occupied')}
                  <input
                    bind:value={alternatives[entry.id]}
                    placeholder={t('disks.altPlaceholder')}
                  />
                </label>
              {/if}
              {#if confirmId === entry.id}
                <div class="confirm" role="alertdialog" aria-label={t('disks.confirmTitle')}>
                  <p>{t('disks.confirmText')}</p>
                  <div class="actions">
                    <button type="button" class="danger" onclick={() => void purge(entry)}
                      >{t('disks.purge')}</button
                    >
                    <button type="button" onclick={() => (confirmId = null)}
                      >{t('disks.cancel')}</button
                    >
                  </div>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <button
      type="button"
      class="egg"
      class:pressed
      aria-label={t('disks.egg.label')}
      onclick={onPoster}
      onanimationend={() => (pressed = false)}
    >
      <img src={poster} alt="" draggable="false" />
    </button>
  </div>
{/if}

{#if subsection === 'files'}
  <section class="panel">
    <div class="actions">
      <button
        type="button"
        class="primary"
        onclick={() => void loadFiles()}
        disabled={running !== null}
      >
        {t('disks.files.find')}
      </button>
      {#if running === 'files'}
        <button type="button" onclick={() => void stop()}>{t('disks.stop')}</button>
      {/if}
    </div>
    <p class="hint">{t('disks.files.hint')}</p>
  </section>
  {#if filesLoaded}
    {#if files.length === 0}
      <p class="empty">{t('disks.files.none')}</p>
    {:else}
      <ul class="rows">
        {#each files as file (file.path)}
          <li>
            <span class="path">{file.path}</span>
            <span class="size">{formatSize(file.size)} · {formatDate(file.modifiedUnix)}</span>
          </li>
        {/each}
      </ul>
    {/if}
    <p class="hint">
      {tf('disks.files.scanned', { n: filesScanned, tail: tail(filesComplete) })}
    </p>
  {/if}
{/if}

{#if subsection === 'space'}
  <section class="panel">
    <div class="actions">
      <label>
        {t('disks.space.drive')}
        <select bind:value={drive} disabled={running !== null}>
          {#each drives as item (item)}
            <option value={item}>{item}</option>
          {/each}
        </select>
      </label>
      <button
        type="button"
        class="primary"
        onclick={() => void loadSpace()}
        disabled={running !== null || drive === ''}
      >
        {t('disks.space.analyze')}
      </button>
      {#if running === 'space'}
        <button type="button" onclick={() => void stop()}>{t('disks.stop')}</button>
      {/if}
    </div>
  </section>
  {#if spaceLoaded}
    <ul class="rows">
      {#each folders as folder (folder.path)}
        <li>
          <span class="title">{folder.name}</span>
          <span class="size"
            >{tf('disks.sizeFiles', {
              size: formatSize(folder.bytes),
              n: folder.files,
              tail: tail(folder.complete),
            })}</span
          >
          <span class="meter" aria-hidden="true"
            ><span style:width="{(folder.bytes / maxFolderBytes) * 100}%"></span></span
          >
        </li>
      {/each}
      <li>
        <span class="title">{t('disks.space.root')}</span>
        <span class="size">{formatSize(looseBytes)}</span>
      </li>
    </ul>
    <p class="hint">{spaceComplete ? t('disks.space.full') : t('disks.space.partial')}</p>
  {/if}
{/if}

{#if subsection === 'privacy'}
  <section class="panel">
    <div class="actions">
      <button
        type="button"
        class="primary"
        onclick={() => void loadPrivacy()}
        disabled={running !== null}
      >
        {t('disks.privacy.check')}
      </button>
      {#if running === 'privacy'}
        <button type="button" onclick={() => void stop()}>{t('disks.stop')}</button>
      {/if}
    </div>
    <p class="hint">{t('disks.privacy.hint')}</p>
  </section>
  {#if privacyLoaded}
    <ul class="rows">
      {#each privacy as item (item.id)}
        <li>
          <span class="title">{localizeCategoryTitle(item.id, item.title)}</span>
          <span class="size">
            {#if item.found}
              {tf('disks.sizeEntries', {
                size: formatSize(item.bytes),
                n: item.files,
                tail: tail(item.complete),
              })}
            {:else}
              {t('disks.notFound')}
            {/if}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
{/if}

<style>
  .page-heading {
    margin: 0 0 16px;
    font-family: var(--font-heading);
    font-size: var(--text-section);
    font-weight: 900;
    text-transform: uppercase;
    color: var(--page-text);
  }

  .disk-tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 16px;
    margin: 0 0 16px;
    padding: 0;
    list-style: none;
  }

  .disk-tile {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    padding: 16px;
    border-radius: var(--radius-md);
    background: var(--black);
    color: var(--ink);
  }

  .swatch {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 72px;
    height: 72px;
    border-radius: 16px;
  }

  .disk-info {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .disk-letter {
    font-family: var(--font-heading);
    font-size: 32px;
    font-weight: 900;
    line-height: 32px;
  }

  .disk-meta {
    display: flex;
    gap: 8px;
    min-width: 0;
    font-size: var(--text-small);
    font-weight: 700;
    color: var(--ink-soft);
  }

  .disk-kind {
    flex: none;
  }

  .disk-model {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Две колонки одинаковой ширины в каждой карточке: подпись 96 px, значение справа от неё. */
  .disk-grid {
    display: grid;
    grid-template-columns: 96px minmax(0, 1fr);
    gap: 8px;
    margin: 0;
    font-size: var(--text-small);
  }

  .disk-grid dt {
    color: var(--muted);
  }

  .disk-grid dd {
    margin: 0;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
  }

  .disk-bar {
    height: 8px;
    border-radius: 4px;
    background: var(--surface);
    overflow: hidden;
  }

  .disk-fill {
    display: block;
    height: 100%;
    background: var(--accent);
  }

  .disk-used {
    font-size: var(--text-caption);
    color: var(--muted);
  }

  .cleanup-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) clamp(192px, 22%, 296px);
    gap: 24px;
    align-items: start;
  }

  .cleanup-main {
    min-width: 0;
  }

  /* Пасхалка: портрет 3:4 с обрезкой cover; точка фокуса ниже центра, лицо не режется. */
  .egg {
    display: block;
    width: 100%;
    aspect-ratio: 3 / 4;
    min-height: 0;
    padding: 0;
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: var(--black);
    cursor: pointer;
  }

  .egg img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: 50% 55%;
    user-select: none;
    pointer-events: none;
  }

  .egg.pressed {
    animation: egg-press 260ms cubic-bezier(0.2, 0, 0, 1);
  }

  @keyframes egg-press {
    0% {
      transform: scale(1);
    }
    35% {
      transform: scale(0.94);
    }
    100% {
      transform: scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .egg.pressed {
      animation: none;
    }
  }

  .panel,
  .rows,
  .alert,
  .notice,
  .empty,
  .hint {
    margin: 0 0 12px;
  }

  .panel {
    padding: 16px;
    border-radius: var(--radius-md);
    background: var(--black);
    color: var(--ink);
  }

  .panel .hint {
    margin: 8px 0 0;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .hint,
  .reason,
  small {
    color: var(--muted);
    font-size: var(--text-caption);
  }

  .alert {
    padding: 12px;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--page-text);
  }

  .notice,
  .empty {
    color: var(--page-text);
  }

  h3 {
    margin: 24px 0 8px;
    font-family: var(--font-heading);
    font-size: var(--text-section);
    color: var(--page-text);
    text-transform: uppercase;
  }

  .rows {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .rows li {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 16px;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: var(--black);
    color: var(--ink);
  }

  .rows li.quarantine {
    flex-direction: column;
    align-items: stretch;
  }

  .title {
    font-weight: 800;
  }

  .path {
    word-break: break-all;
  }

  .size {
    color: var(--ink-soft);
    font-size: var(--text-small);
  }

  .meter {
    flex-basis: 100%;
    height: 6px;
    border-radius: 3px;
    background: var(--surface);
    overflow: hidden;
  }

  .meter span {
    display: block;
    height: 100%;
    background: var(--ink);
  }

  .confirm {
    padding: 8px;
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--ink);
  }

  .confirm p {
    margin: 0 0 8px;
  }

  .alt {
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: var(--ink-soft);
  }

  .failures {
    color: var(--page-text);
    font-size: var(--text-small);
  }

  label {
    display: inline-flex;
    gap: 8px;
    align-items: center;
    color: var(--ink);
  }

  input,
  select {
    min-height: 36px;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
  }

  button {
    min-height: 44px;
    padding: 10px 16px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
    font-weight: 800;
    cursor: pointer;
  }

  button.primary {
    background: var(--btn-bg);
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--btn-border);
  }

  button.danger {
    background: var(--page-text);
    color: var(--black);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
